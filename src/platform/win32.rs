//! Windows API specific implementations and API extensions.

pub(crate) mod api;
pub(crate) mod util;

use self::api::*;

use crate::{
    error::Error,
    event::{CloseReason, Event},
    helpers::{LazyCell, sync::{condvar_notify1, condvar_wait, mutex_lock, Condvar, Mutex}},
    window::{WindowBuilder, WindowImpl, WindowStyle},
};
use std::{cell, fmt, mem, ops, ptr, sync, thread};

/// Global lock used to synchronize classes being registered or queried.
static CLASS_REGISTRY_LOCK: LazyCell<Mutex<()>> = LazyCell::new(Default::default);

/// The initial capacity of the `Vec<Event>` structures
///
/// TODO: This should be bigger than normal if input is enabled
const EVENT_BUF_INITIAL_SIZE: usize = 512;

// Custom events
const RAMEN_WM_EXECUTE: UINT = WM_USER + 0;
const RAMEN_WM_CLOSE: UINT = WM_USER + 1;

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
}

struct WindowUserData {
    close_reason: Option<CloseReason>,
    event_queue: Mutex<Vec<Event>>,
    window_style: WindowStyle,
}

impl Default for WindowUserData {
    fn default() -> Self {
        Self {
            close_reason: None,
            event_queue: Mutex::new(Vec::with_capacity(EVENT_BUF_INITIAL_SIZE)),
            window_style: Default::default(),
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

pub(crate) fn make_window(builder: &WindowBuilder) -> Result<WindowRepr, Error> {
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
        if GetClassInfoExW(util::this_hinstance(), class_name, class_info.as_mut_ptr()) == 0 {
            // The window class not existing sets the thread global error flag,
            // we clear it immediately to avoid any confusion down the line.
            SetLastError(ERROR_SUCCESS);

            // Fill in & register class (`cbSize` is set before this `if` block)
            let class = &mut *class_info.as_mut_ptr();
            class.style = CS_OWNDC;
            class.lpfnWndProc = window_proc;
            class.cbClsExtra = 0;
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
        }
        mem::drop(class_registry_lock);

        let style = builder.style.dword_style();
        let style_ex = 0;

        let width = 1280;
        let height = 720;
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

        // Run message loop until error or exit
        let mut msg = mem::MaybeUninit::<MSG>::zeroed().assume_init();
        'message_loop: loop {
            // `HWND hWnd` is set to NULL here to query all messages on the thread,
            // as the exit condition/signal `WM_QUIT` is not associated with any window.
            // This is one of the main motivations (besides no blocking) to give each window a thread.
            match GetMessageW(&mut msg, ptr::null_mut(), 0, 0) {
                -1 => panic!("Hard error {:#06X} in GetMessageW loop!", GetLastError()),
                0 => break 'message_loop,
                _ => {
                    // Dispatch message to WindowProc
                    let _ = DispatchMessageW(&msg);
                },
            }
        }

        // Registered window classes are unregistered automatically when the process closes.
        // Until then, there's no reason not to have them around as the contents never vary.
        // > if something { UnregisterClassW(class_atom); }
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

unsafe extern "system" fn window_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe fn user_data<'a>(hwnd: HWND) -> &'a mut WindowUserData {
        &mut *(get_window_data(hwnd, GWLP_USERDATA) as *mut WindowUserData)
    }

    #[inline]
    unsafe fn push_event(user_data: &mut WindowUserData, event: Event) {
        let mut lock = mutex_lock(&user_data.event_queue);
        lock.push(event);
        mem::drop(lock);
    }

    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
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
        RAMEN_WM_EXECUTE => {
            // TODO: Before release, test if any blocking functions in here can deadlock.
            // It shouldn't actually be possible, but better safe than sorry.
            let f = wparam as *mut &mut (dyn FnMut());
            (*f)();
            0
        },

        // Custom event: Close the window, but for real (`WM_CLOSE` is rejected always).
        RAMEN_WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
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
