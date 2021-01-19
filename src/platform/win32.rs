//! Win32 specific implementations and API extensions.

pub(crate) mod api;
pub(crate) mod util;

use api::*;
use crate::{
    error::Error,
    event::{CloseReason, Event},
    helpers::{LazyCell, sync::{condvar_notify1, condvar_wait, mutex_lock, Condvar, Mutex}},
    window::{WindowBuilder, WindowImpl, WindowStyle},
};
use std::{cell, fmt, mem, ops, ptr, sync, thread};

/// Global lock used to synchronize classes being registered or queried.
static CLASS_REGISTRY_LOCK: LazyCell<Mutex<()>> = LazyCell::new(Default::default);

/// Dynamically queried Win32 functions and constants.
static WIN32: LazyCell<util::Win32> = LazyCell::new(Default::default);

/// Marker to filter out implementation magic like `CicMarshalWndClass`
const HOOKPROC_MARKER: &[u8; 4] = b"viri";

/// The initial capacity of the `Vec<Event>` structures
///
/// TODO: This should be bigger than normal if input is enabled
const EVENT_BUF_INITIAL_SIZE: usize = 512;

// Custom events
const RAMEN_WM_EXECUTE: UINT = WM_USER + 0;
const RAMEN_WM_CLOSE: UINT = WM_USER + 1;
const RAMEN_WM_SETTEXT_ASYNC: UINT = WM_USER + 2;

#[derive(Debug)]
pub struct InternalError {
    code: DWORD,
    context: &'static str,
    message: String,
}

impl std::error::Error for InternalError {}
impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (Code {:#06X}; {})", self.message.as_str(), self.code, self.context)
    }
}

impl InternalError {
    pub fn from_winapi(context: &'static str, code: DWORD) -> Self {
        Self {
            code,
            context,
            message: unsafe { util::error_string_repr(code) },
        }
    }
}

pub(crate) struct Window {
    // guts
    hwnd: HWND,
    thread: Option<thread::JoinHandle<()>>,

    // api
    user_data: Box<cell::UnsafeCell<WindowUserData>>,
    event_buffer: Vec<Event>,
}
unsafe impl Send for Window {}
unsafe impl Sync for Window {}

/// Win32 specific extensions to the [`WindowBuilder`](crate::window::WindowBuilder) API.
///
/// # Example
///
/// ```rust
/// use ramen::platform::win32::WindowBuilderExt as _;
/// use ramen::window::Window;
///
/// let window = Window::builder()
///     .tool_window(true)
///     .build()?;
/// ```
pub trait WindowBuilderExt {
    /// Sets whether the window uses the [`WS_EX_TOOLWINDOW`](
    /// https://docs.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles#WS_EX_TOOLWINDOW)
    /// style.
    ///
    /// This is equivalent to the .NET [`WindowStyle.ToolWindow`](
    /// https://docs.microsoft.com/en-us/dotnet/api/system.windows.windowstyle?view=net-5.0#System_Windows_WindowStyle_ToolWindow)
    /// property.
    ///
    /// From MSDN: *The window is intended to be used as a floating toolbar.*
    /// *A tool window has a title bar that is shorter than a normal title bar,*
    /// *and the window title is drawn using a smaller font.*
    /// *A tool window does not appear in the taskbar or in the dialog*
    /// *that appears when the user presses ALT+TAB.*
    fn tool_window(&mut self, tool_window: bool) -> &mut Self;
}

impl WindowBuilderExt for WindowBuilder {
    fn tool_window(&mut self, tool_window: bool) -> &mut Self {
        self.style.tool_window = tool_window;
        self
    }
}

pub(crate) type WindowRepr = Window;

struct WindowCreateParams {
    builder_ptr: *const WindowBuilder,
    user_data_ptr: *mut WindowUserData,
    error_return: Option<Error>,
}

impl WindowStyle {
    /// Gets this style as a bitfield. Note that it does not include the close button.
    /// The close button is a menu property, not a window style.
    pub fn dword_style(&self) -> DWORD {
        let mut style = 0;

        if self.borderless {
            // TODO: Why does this just not work without THICKFRAME? Borderless is dumb.
            style |= WS_POPUP | WS_THICKFRAME;
        } else {
            style |= WS_OVERLAPPED | WS_BORDER | WS_CAPTION;
        }

        if self.resizable {
            style |= WS_THICKFRAME;
        }

        if self.visible {
            style |= WS_VISIBLE;
        }

        if let Some(controls) = &self.controls {
            if controls.minimize {
                style |= WS_MINIMIZEBOX;
            }
            if controls.maximize {
                style |= WS_MAXIMIZEBOX;
            }
            style |= WS_SYSMENU;
        }

        style
    }

    pub fn dword_style_ex(&self) -> DWORD {
        let mut style = 0;

        if self.rtl_layout {
            style |= WS_EX_LAYOUTRTL;
        }

        if self.tool_window {
            style |= WS_EX_TOOLWINDOW;
        }

        style
    }
}

struct WindowUserData {
    close_reason: Option<CloseReason>,
    event_queue: Mutex<Vec<Event>>,
    destroy_flag: bool,
    window_style: WindowStyle,
}

impl Default for WindowUserData {
    fn default() -> Self {
        Self {
            close_reason: None,
            event_queue: Mutex::new(Vec::with_capacity(EVENT_BUF_INITIAL_SIZE)),
            destroy_flag: false,
            window_style: Default::default(),
        }
    }
}

pub(crate) fn make_window(builder: &WindowBuilder) -> Result<WindowRepr, Error> {
    // Force this so it panics the main thread if something somehow goes wrong
    let _ = WIN32.get();

    // Condvar & mutex pair for receiving the `Result<WindowRepr, Error>` from spawned thread
    let signal = sync::Arc::new((Mutex::<Option<Result<WindowRepr, Error>>>::new(None), Condvar::new()));

    let builder = builder.clone();
    let cond_pair = sync::Arc::clone(&signal);
    let thread_builder = thread::Builder::new()
        .name(format!("Window Thread (Class \"{}\")", builder.class_name.as_ref()));
    let window_thread = thread_builder.spawn(move || unsafe {
        // TODO: Sanitize reserved window classes
        let mut class_info = mem::MaybeUninit::<WNDCLASSEXW>::uninit();
        (&mut *class_info.as_mut_ptr()).cbSize = mem::size_of_val(&class_info) as DWORD;

        let mut class_name_buf = Vec::new();
        let class_name = util::str_to_wide_null(builder.class_name.as_ref(), &mut class_name_buf);

        // Create the window class if it doesn't exist yet
        let class_registry_lock = mutex_lock(&*CLASS_REGISTRY_LOCK);
        let mut class_created_here = false; // did this thread create the class?
        if GetClassInfoExW(util::this_hinstance(), class_name, class_info.as_mut_ptr()) == 0 {
            // The window class not existing sets the thread global error flag,
            // we clear it immediately to avoid any confusion down the line.
            SetLastError(ERROR_SUCCESS);

            // Fill in & register class (`cbSize` is set before this `if` block)
            let class = &mut *class_info.as_mut_ptr();
            class.style = CS_OWNDC;
            class.lpfnWndProc = window_proc;
            class.cbClsExtra = mem::size_of::<usize>() as c_int;
            class.cbWndExtra = 0;
            class.hInstance = util::this_hinstance();
            class.hIcon = ptr::null_mut();
            class.hCursor = ptr::null_mut();
            class.hbrBackground = ptr::null_mut();
            class.lpszMenuName = ptr::null_mut();
            class.lpszClassName = class_name;
            class.hIconSm = ptr::null_mut();

            // The fields on `WNDCLASSEXW` are valid so this can't fail
            let _ = RegisterClassExW(class);
            class_created_here = true;
        }
        mem::drop(class_registry_lock);

        let style = builder.style.dword_style();
        let style_ex = builder.style.dword_style_ex();

        let width = 600;
        let height = 360;
        let user_data: Box<cell::UnsafeCell<WindowUserData>> = Default::default();

        let builder_ptr = (&builder) as *const WindowBuilder;
        let user_data_ptr = user_data.as_ref().get();
        (&mut *user_data_ptr).window_style = builder.style.clone();
        let mut params = WindowCreateParams {
            builder_ptr,
            user_data_ptr,

            error_return: None,
        };

        // Creates the window - this waits for `WM_NCCREATE` & `WM_CREATE` to return (in that order)
        let mut title = Vec::new();
        let hwnd = CreateWindowExW(
            style_ex,
            class_name,
            util::str_to_wide_null(builder.title.as_ref(), &mut title),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            ptr::null_mut(),
            ptr::null_mut(),
            util::this_hinstance(),
            (&mut params) as *mut WindowCreateParams as LPVOID,
        );

        // Handle the window failing to create from an unknown reason
        if hwnd.is_null() && params.error_return.is_none() {
            params.error_return = Some(Error::from_internal(InternalError::from_winapi(
                "CreateWindowExW returned NULL.",
                GetLastError(),
            )));
        }

        // Yield window struct, signal outer function
        // NOTE: For future developments, do not insert panics before this,
        // or the waiting condvar never gets anything and thus the outer fn never returns
        let (mutex, condvar) = &*cond_pair;
        let mut lock = mutex_lock(&mutex);
        *lock = Some(match params.error_return {
            Some(err) => Err(err),
            None => Ok(Window {
                hwnd,
                thread: None,
                user_data,
                event_buffer: Vec::with_capacity(EVENT_BUF_INITIAL_SIZE),
            }),
        });
        condvar_notify1(&condvar);
        mem::drop(lock);

        // Release condvar + mutex pair so the `Arc` contents are deallocated once the outer fn returns
        mem::drop(cond_pair);

        // Free unused wide string buffers
        mem::drop(class_name_buf);
        mem::drop(title);

        // Set marker to identify our windows in HOOKPROC functions
        if class_created_here {
            let _ = set_class_data(hwnd, 0, u32::from_le_bytes(*HOOKPROC_MARKER) as usize);
        }

        // Setup `HCBT_DESTROYWND` hook
        let thread_id = GetCurrentThreadId();
        let hhook = SetWindowsHookExW(WH_CBT, hcbt_destroywnd_hookproc, ptr::null_mut(), thread_id);
        // TODO: What if this fails? Can it?
        assert!(!hhook.is_null());

        // Run message loop until error or exit
        let mut msg = mem::MaybeUninit::<MSG>::zeroed().assume_init();
        'message_loop: loop {
            // `HWND hWnd` is set to NULL here to query all messages on the thread,
            // as the exit condition/signal `WM_QUIT` is not associated with any window.
            // This is one of the main motivations (besides no blocking) to give each window a thread.
            match GetMessageW(&mut msg, ptr::null_mut(), 0, 0) {
                -1 => panic!("Hard error {:#06X} in GetMessageW loop!", GetLastError()),
                0 => if (&*user_data_ptr).destroy_flag {
                    break 'message_loop
                },
                _ => {
                    // Dispatch message to WindowProc
                    let _ = DispatchMessageW(&msg);
                },
            }
        }

        // Registered window classes are unregistered automatically when the process closes.
        // Until then, there's no reason not to have them around as the contents never vary.
        // > if something { UnregisterClassW(class_atom); }

        // Free `HCBT_DESTROYWND` hook (the one associated with this thread)
        let _ = UnhookWindowsHookEx(hhook);
    }).expect("Failed to spawn window thread");

    // Wait until the thread is done creating the window or notifying us why it couldn't do that
    let (mutex, condvar) = &*signal;
    let mut lock = mutex_lock(&mutex);
    loop {
        if let Some(result) = (&mut *lock).take() {
            break result.map(|mut window| {
                window.thread = Some(window_thread);
                window
            })
        } else {
            condvar_wait(&condvar, &mut lock);
        }
    }
}

impl WindowImpl for Window {
    #[inline]
    fn events(&self) -> &[Event] {
        self.event_buffer.as_slice()
    }

    fn execute(&self, mut f: &mut dyn FnMut()) {
        let wrap: *mut &mut dyn FnMut() = (&mut f) as *mut _;
        assert_eq!(mem::size_of_val(&wrap), mem::size_of::<WPARAM>());
        unsafe {
            let _ = SendMessageW(
                self.hwnd,
                RAMEN_WM_EXECUTE,
                wrap as WPARAM,
                0,
            );
        }
    }

    fn set_title(&self, title: &str) {
        let mut wstr = Vec::new();
        let ptr = util::str_to_wide_null(title, &mut wstr);
        unsafe {
            let _ = SendMessageW(self.hwnd, WM_SETTEXT, 0, ptr as LPARAM);
        }
    }

    fn set_title_async(&self, title: &str) {
        // Win32 has special behaviour on WM_SETTEXT, since it takes a pointer to a buffer.
        // You can't actually call it asynchronously, in case it's being sent from a different process.
        // Only if they had Rust back then, this poorly documented stupid detail would not exist,
        // as trying to use PostMessageW with WM_SETTEXT silently fails because it's scared of lifetimes.
        // As a workaround, we just define our own event, X_WM_SET_TITLE_ASYNC, and still support WM_SETTEXT.
        // This is better than using the "unused parameter" in WM_SETTEXT anyways.
        // More info: https://devblogs.microsoft.com/oldnewthing/20110916-00/?p=9623
        let mut wstr: Vec<WCHAR> = Vec::new();
        unsafe {
            if *util::str_to_wide_null(title, &mut wstr) == 0x00 {
                // There's a special implementation for lParam == NULL
                let _ = PostMessageW(self.hwnd, RAMEN_WM_SETTEXT_ASYNC, 0, 0);
            } else {
                // Post async message - `window_proc` manages the memory
                let lparam = wstr.as_ptr() as LPARAM;
                let _ = PostMessageW(self.hwnd, RAMEN_WM_SETTEXT_ASYNC, wstr.len() as WPARAM, lparam);
                mem::forget(wstr);
            }
        }
    }

    #[inline]
    fn set_visible(&self, visible: bool) {
        unsafe {
            let _ = ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    #[inline]
    fn set_visible_async(&self, visible: bool) {
        unsafe {
            let _ = ShowWindowAsync(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    fn swap_events(&mut self) {
        let user_data = unsafe { &mut *self.user_data.get() };
        let mut vec_lock = mutex_lock(&user_data.event_queue);
        mem::swap(&mut self.event_buffer, vec_lock.as_mut());
        vec_lock.clear();
        mem::drop(vec_lock);
    }
}

unsafe fn user_data<'a>(hwnd: HWND) -> &'a mut WindowUserData {
    &mut *(get_window_data(hwnd, GWLP_USERDATA) as *mut WindowUserData)
}

unsafe extern "system" fn hcbt_destroywnd_hookproc(code: c_int, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HCBT_DESTROYWND {
        let hwnd = wparam as HWND;
        if get_class_data(hwnd, GCL_CBCLSEXTRA) == mem::size_of::<usize>()
            && (get_class_data(hwnd, 0) as u32).to_le_bytes() == *HOOKPROC_MARKER
        {
            if user_data(hwnd).destroy_flag {
                0 // Allow
            } else {
                1 // Prevent
            }
        } else {
            0 // Allow (disallow further hooks on HCBT_DESTROYWND)
        }
    } else {
        CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    #[inline]
    unsafe fn push_event(user_data: &mut WindowUserData, event: Event) {
        let mut lock = mutex_lock(&user_data.event_queue);
        lock.push(event);
        mem::drop(lock);
    }

    match msg {
        WM_DESTROY => {
            if user_data(hwnd).destroy_flag {
                PostQuitMessage(0);
            }
            0
        },

        WM_CLOSE => {
            let user_data = user_data(hwnd);
            let reason = user_data.close_reason.take().unwrap_or(CloseReason::Unknown);
            push_event(user_data, Event::CloseRequest(reason));
            0
        },

        WM_NCCREATE => {
            // `lpCreateParams` is the first member, so `CREATESTRUCTW *` is `WindowCreateParams **`
            let params = &mut **(lparam as *const *mut WindowCreateParams);

            // Store user data pointer
            let _ = set_window_data(hwnd, GWLP_USERDATA, params.user_data_ptr as usize);

            let _ = params.builder_ptr;

            DefWindowProcW(hwnd, msg, wparam, lparam)
        },

        WM_NCDESTROY => {
            // finalize
            DefWindowProcW(hwnd, msg, wparam, lparam)
        },

        // Custom event: Run arbitrary functions.
        // wParam: Function pointer of type `*mut &mut dyn FnMut()`.
        // lParam: Unused, set to zero.
        RAMEN_WM_EXECUTE => {
            // TODO: Before release, test if any blocking functions in here can deadlock.
            // It shouldn't actually be possible, but better safe than sorry.
            let f = wparam as *mut &mut dyn FnMut();
            (*f)();
            0
        },

        // Custom event: Close the window, but for real (`WM_CLOSE` is rejected always).
        // wParam & lParam: Unused, set to zero.
        RAMEN_WM_CLOSE => {
            user_data(hwnd).destroy_flag = true;
            let _ = DestroyWindow(hwnd);
            0
        },

        // Custom event: Set the title asynchronously.
        // wParam: Buffer length, if lParam != NULL.
        // lParam: Vec<WCHAR> pointer or NULL for empty.
        RAMEN_WM_SETTEXT_ASYNC => {
            if lparam != 0 {
                let vec = Vec::from_raw_parts(lparam as *mut WCHAR, wparam as usize, wparam as usize);
                let _ = DefWindowProcW(hwnd, WM_SETTEXT, 0, vec.as_ptr() as LPARAM);
                mem::drop(vec); // managed by `window_proc`, caller should `mem::forget`
            } else {
                let _ = DefWindowProcW(hwnd, WM_SETTEXT, 0, util::WSTR_EMPTY.as_ptr() as LPARAM);
            }
            0
        },

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

impl ops::Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = PostMessageW(self.hwnd, RAMEN_WM_CLOSE, 0, 0);
        }
        let _ = self.thread.take().map(thread::JoinHandle::join);
    }
}
