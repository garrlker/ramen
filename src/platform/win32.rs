//! Win32 specific implementations and API extensions.

pub(crate) mod api;
pub(crate) mod util;

use api::*;
use crate::{
    error::Error,
    event::{CloseReason, Event},
    helpers::{LazyCell, sync::{condvar_notify1, condvar_wait, mutex_lock, Condvar, Mutex}},
    window::{WindowBuilder, WindowControls, WindowImpl, WindowStyle},
};
use std::{cell, fmt, mem, ops, ptr, sync::{self, atomic::{self, AtomicBool}}, thread};

#[cfg(feature = "cursor-lock")]
use crate::window::CursorLock;

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
const RAMEN_WM_DESTROY: UINT = WM_USER + 1;
const RAMEN_WM_SETTEXT_ASYNC: UINT = WM_USER + 2;
const RAMEN_WM_SETCONTROLS: UINT = WM_USER + 3;
const RAMEN_WM_SETTHICKFRAME: UINT = WM_USER + 4;
#[cfg(feature = "cursor-lock")]
const RAMEN_WM_SETCURSORLOCK: UINT = WM_USER + 5;

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
    pub(crate) fn dword_style(&self) -> DWORD {
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

    pub(crate) fn dword_style_ex(&self) -> DWORD {
        let mut style = 0;

        if self.rtl_layout {
            style |= WS_EX_LAYOUTRTL;
        }

        if self.tool_window {
            style |= WS_EX_TOOLWINDOW;
        }

        style
    }

    pub(crate) fn set_for(&self, hwnd: HWND) {
        let style = self.dword_style();
        let style_ex = self.dword_style_ex();
        unsafe {
            let _ = set_window_data(hwnd, GWL_STYLE, style as usize);
            let _ = set_window_data(hwnd, GWL_EXSTYLE, style_ex as usize);
        }
    }
}

struct WindowUserData {
    close_reason: Option<CloseReason>,
    #[cfg(feature = "cursor-lock")]
    cursor_constrain_escaped: bool,
    #[cfg(feature = "cursor-lock")]
    cursor_lock: Option<CursorLock>,
    destroy_flag: AtomicBool,
    event_queue: Mutex<Vec<Event>>,
    focus_state: bool,
    window_style: WindowStyle,
}

impl Default for WindowUserData {
    fn default() -> Self {
        Self {
            close_reason: None,
            #[cfg(feature = "cursor-lock")]
            cursor_constrain_escaped: false,
            #[cfg(feature = "cursor-lock")]
            cursor_lock: None,
            destroy_flag: AtomicBool::new(false),
            event_queue: Mutex::new(Vec::with_capacity(EVENT_BUF_INITIAL_SIZE)),
            focus_state: false,
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

        let dpi = util::BASE_DPI;
        let (width, height) = WIN32.adjust_window_for_dpi(builder.inner_size, style, style_ex, dpi);
        let user_data: Box<cell::UnsafeCell<WindowUserData>> = Default::default();

        let builder_ptr = (&builder) as *const WindowBuilder;
        let user_data_ptr = user_data.as_ref().get();
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
                0 => if (&*user_data_ptr).destroy_flag.load(atomic::Ordering::Acquire) {
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

    #[inline]
    fn set_controls(&self, controls: Option<WindowControls>) {
        let controls = controls.map(|c| c.to_bits()).unwrap_or(!0);
        unsafe {
            let _ = SendMessageW(self.hwnd, RAMEN_WM_SETCONTROLS, controls as WPARAM, 0);
        }
    }

    #[inline]
    fn set_controls_async(&self, controls: Option<WindowControls>) {
        let controls = controls.map(|c| c.to_bits()).unwrap_or(!0);
        unsafe {
            let _ = PostMessageW(self.hwnd, RAMEN_WM_SETCONTROLS, controls as WPARAM, 0);
        }
    }

    #[cfg(feature = "cursor-lock")]
    #[inline]
    fn set_cursor_lock(&self, mode: Option<CursorLock>) {
        let mode = mode.map(|e| e as u32).unwrap_or(0);
        unsafe {
            let _ = SendMessageW(self.hwnd, RAMEN_WM_SETCURSORLOCK, mode as WPARAM, 0);
        }
    }

    #[cfg(feature = "cursor-lock")]
    #[inline]
    fn set_cursor_lock_async(&self, mode: Option<CursorLock>) {
        let mode = mode.map(|e| e as u32).unwrap_or(0);
        unsafe {
            let _ = PostMessageW(self.hwnd, RAMEN_WM_SETCURSORLOCK, mode as WPARAM, 0);
        }
    }

    #[inline]
    fn set_resizable(&self, resizable: bool) {
        unsafe {
            let _ = SendMessageW(self.hwnd, RAMEN_WM_SETTHICKFRAME, resizable as WPARAM, 0);
        }
    }

    #[inline]
    fn set_resizable_async(&self, resizable: bool) {
        unsafe {
            let _ = PostMessageW(self.hwnd, RAMEN_WM_SETTHICKFRAME, resizable as WPARAM, 0);
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
        // As a workaround, we just define our own event, WM_SETTEXT_ASYNC, and still support WM_SETTEXT.
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
    &mut *(get_window_data(hwnd, GWL_USERDATA) as *mut WindowUserData)
}

unsafe extern "system" fn hcbt_destroywnd_hookproc(code: c_int, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HCBT_DESTROYWND {
        let hwnd = wparam as HWND;
        if get_class_data(hwnd, GCL_CBCLSEXTRA) == mem::size_of::<usize>()
            && (get_class_data(hwnd, 0) as u32).to_le_bytes() == *HOOKPROC_MARKER
        {
            // Note that nothing is forwarded here, we decide for our windows
            if user_data(hwnd).destroy_flag.load(atomic::Ordering::Acquire) {
                0 // Allow
            } else {
                1 // Prevent
            }
        } else {
            // Unrelated window, forward
            CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
        }
    } else {
        // Unrelated event, forward
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
        // No-op event with various uses such as pinging the event loop.
        WM_NULL => 0,

        // Init event, completed *before* `CreateWindowExW` returns, but *after* `WM_NCCREATE`.
        // Return 0 to continue creation or -1 to destroy and return NULL from `CreateWindowExW`.
        WM_CREATE => {
            // `lpCreateParams` is the first member, so `CREATESTRUCTW *` is `WindowCreateParams **`
            let params = &mut **(lparam as *const *mut WindowCreateParams);
            let builder = &*params.builder_ptr;
            let mut user_data = &mut *params.user_data_ptr;

            // The close button is part of the system menu, so it's updated here
            if let Some(false) = builder.style.controls.as_ref().map(|c| c.close) {
                util::set_close_button(hwnd, false);
            }

            // Copy style, cursor lock mode, etc
            user_data.window_style = builder.style.clone();
            #[cfg(feature = "cursor-lock")]
            {
                user_data.cursor_lock = builder.cursor_lock;
            }

            0 // OK
        },

        // Received as the window is being destroyed after it has been removed from the screen.
        // Nothing can be done once this stage is hit naturally, which is why the CBT hook exists.
        WM_DESTROY => {
            if user_data(hwnd).destroy_flag.load(atomic::Ordering::Acquire) {
                PostQuitMessage(0);
            }
            0
        },

        // Received after the window has been moved, sent from DefWndProc's `WM_WINDOWPOSCHANGED`.
        // Since the window is on its own thread, this won't block and is just instead sent 1000 times.
        WM_MOVE => {
            // TODO: Do it
            0
        },

        // << Event 0x0004 non-existent >>

        // Received *after* the window has been resized, sent from DefWndProc's `WM_WINDOWPOSCHANGED`.
        WM_SIZE => {
            // TODO: Do it
            0
        },

        // Received when the window loses or gains focus.
        WM_ACTIVATE => {
            let user_data = user_data(hwnd);

            // Quoting MSDN:
            // The high-order word specifies the minimized state of the window being activated
            // or deactivated. A nonzero value indicates the window is minimized.
            //
            // So, if we don't do some logic here we get two events on unfocusing
            // by clicking on the taskbar icon for example, among other things:
            // 1) WM_INACTIVE (HIWORD == 0)
            // 2) WM_ACTIVATE (HIWORD != 0)
            // Note that the second event means focused & minimized at the same time. Fantastic.
            let focus = wparam & 0xFFFF != 0;
            let is_minimize = (wparam >> 16) & 0xFFFF != 0;
            match (focus, is_minimize) {
                (true, true) => return 0, // nonsense
                (state, _) => push_event(user_data, Event::Focus(state)),
            }
            user_data.focus_state = focus;

            #[cfg(feature = "cursor-lock")]
            {
                // We need to update the cursor lock here, *if* we are cursor locking.
                // Unfortunately if the user clicks the maximize/minimize/close button,
                // locking yanks the mouse away from the button, while still setting capture,
                // making us unable to detect it (in a reasonable, sane way, as far as I know).
                // To make it worse, this doesn't even set `WM_CLICKACTIVE` for wParam of this message.
                // As a compromise, we let the user drag the window around or click those buttons.
                // This is `cursor_constrain_escaped`, a flag indicating we let the cursor escape.
                // The lock is re-acquired naturally once the mouse re-enters the window.
                if focus && user_data.cursor_lock.is_some() {
                    unsafe fn m1_down() -> bool {
                        let vk_primary = if GetSystemMetrics(SM_SWAPBUTTON) == 0 {
                            1 // VK_LBUTTON
                        } else {
                            2 // VK_RBUTTON
                        };
                        (GetAsyncKeyState(vk_primary) >> 15) != 0
                    }

                    if m1_down() && util::is_cursor_in_titlebar(hwnd) {
                        user_data.cursor_constrain_escaped = true;
                    } else {
                        util::update_cursor_lock(hwnd, user_data.cursor_lock, false);
                    }
                } else {
                    util::update_cursor_lock(hwnd, None, true);
                }
            }

            0
        },

        WM_CLOSE => {
            let user_data = user_data(hwnd);
            let reason = user_data.close_reason.take().unwrap_or(CloseReason::Unknown);
            push_event(user_data, Event::CloseRequest(reason));
            0
        },

        WM_SHOWWINDOW => {
            // If `lparam == 0`, this was received from `ShowWindow` or `ShowWindowAsync`
            if lparam == 0 {
                user_data(hwnd).window_style.visible = wparam != 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        },

        WM_NCCREATE => {
            // `lpCreateParams` is the first member, so `CREATESTRUCTW *` is `WindowCreateParams **`
            let params = &mut **(lparam as *const *mut WindowCreateParams);

            // Enable the non-client scaling patch for PMv1
            let win32 = WIN32.get();
            if matches!(win32.dpi_mode, util::DpiMode::PerMonitorV1) && win32.at_least_anniversary_update {
                if let Some(FALSE) | None = win32.dl.EnableNonClientDpiScaling(hwnd) {
                    // TODO: do something other than panicking, write the error
                    panic!("Failed to enable non-client DPI scaling on Win10 v1607+!");
                }
            }

            // Store user data pointer
            let _ = set_window_data(hwnd, GWL_USERDATA, params.user_data_ptr as usize);

            DefWindowProcW(hwnd, msg, wparam, lparam)
        },

        WM_NCDESTROY => {
            // finalize
            DefWindowProcW(hwnd, msg, wparam, lparam)
        },

        // Received when the user selects a window control.
        WM_SYSCOMMAND => {
            if wparam == SC_CLOSE {
                user_data(hwnd).close_reason = Some(CloseReason::SystemMenu);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        },

        // TODO: what do you think
        WM_MOUSEMOVE => {
            let user_data = user_data(hwnd);

            #[cfg(feature = "cursor-lock")]
            {
                match user_data.cursor_lock {
                    Some(CursorLock::Constrain) if user_data.cursor_constrain_escaped => {
                        util::update_cursor_lock(hwnd, user_data.cursor_lock, false);
                    },
                    Some(CursorLock::Center) if user_data.focus_state => {
                        util::update_cursor_lock(hwnd, user_data.cursor_lock, false);
                    },
                    _ => (),
                }
                user_data.cursor_constrain_escaped = false;
            }

            let _ = user_data; // soon used

            0
        },

        // MSDN: Sent one time to a window, after it has exited the moving or sizing modal loop.
        // wParam & lParam are unused.
        WM_EXITSIZEMOVE => {
            #[cfg(feature = "cursor-lock")]
            {
                let user_data = user_data(hwnd);
                if user_data.cursor_lock.is_some() {
                    util::update_cursor_lock(hwnd, user_data.cursor_lock, false);
                }
            }
            0
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

        // Custom event: Destroy the window (`WM_CLOSE` & `DestroyWindow` are rejected normally).
        // wParam & lParam: Unused, set to zero.
        RAMEN_WM_DESTROY => {
            user_data(hwnd).destroy_flag.store(true, atomic::Ordering::Release);
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

        // Custom event: Update window controls.
        // wParam: If anything but !0 (~0 in C terms), window controls bits, else None.
        // lParam: Unused, set to zero.
        RAMEN_WM_SETCONTROLS => {
            let mut user_data = user_data(hwnd);
            let controls = {
                let bits = wparam as u32;
                if bits != !0 {
                    Some(WindowControls::from_bits(bits))
                } else {
                    None
                }
            };
            if user_data.window_style.controls != controls {
                user_data.window_style.controls = controls;

                // Update system menu's close button if present
                if let Some(close) = user_data.window_style.controls.as_ref().map(|c| c.close) {
                    util::set_close_button(hwnd, close);
                }

                // Set styles, refresh
                user_data.window_style.set_for(hwnd);
                util::ping_window_frame(hwnd);
            }
            0
        },

        // Custom event: Set whether the window is resizable.
        // wParam: If non-zero, resizable, otherwise not resizable.
        // lParam: Unused, set to zero.
        RAMEN_WM_SETTHICKFRAME => {
            let mut user_data = user_data(hwnd);
            let resizable = wparam != 0;
            if user_data.window_style.resizable != resizable {
                user_data.window_style.resizable = resizable;
                user_data.window_style.set_for(hwnd);
            }
            0
        },

        // Custom event: Set the cursor lock.
        // wParam: If non-zero, a `CursorLock` variant, else `None`.
        // lParam: Unused, set to zero.
        #[cfg(feature = "cursor-lock")]
        RAMEN_WM_SETCURSORLOCK => {
            let mut user_data = user_data(hwnd);
            if wparam != 0 {
                user_data.cursor_lock = Some(mem::transmute::<_, CursorLock>(wparam as u32));
            } else {
                user_data.cursor_lock = None;
            }
            util::update_cursor_lock(hwnd, user_data.cursor_lock, true);
            0
        },

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

impl ops::Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = PostMessageW(self.hwnd, RAMEN_WM_DESTROY, 0, 0);
        }
        let _ = self.thread.take().map(thread::JoinHandle::join);
    }
}
