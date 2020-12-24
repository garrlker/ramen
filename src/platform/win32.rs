//! Windows API specific implementations and API extensions.

#[path = "bindings/win32.rs"]
mod bindings;
use self::bindings::*;

use crate::{
    error::Error,
    event::{CloseReason, Event},
    helpers::LazyCell,
    window::{WindowBuilder, WindowImpl},
};
use std::{cell, fmt, mem, ops, ptr, thread};
use std::sync::{Arc, Condvar, Mutex}; // move later

/// Global lock used to synchronize classes being registered or queried.
static CLASS_REGISTRY_LOCK: LazyCell<Mutex<()>> = LazyCell::new(Default::default);

/// Empty wide string (to point to)
static EMPTY_WIDE_STRING: &[WCHAR] = &[0x00];

/// The initial capacity of the `Vec<Event>` structures
///
/// TODO: This should be bigger than normal if input is enabled
const EVENT_BUF_INITIAL_SIZE: usize = 512;

// Custom events
const RAMEN_WM_CLOSE: UINT = WM_USER + 0;

/// Retrieves the base module HINSTANCE.
#[inline]
fn this_hinstance() -> HINSTANCE {
    extern "system" {
        // Microsoft's linkers provide a static HINSTANCE to not have to query it at runtime.
        // Source: https://devblogs.microsoft.com/oldnewthing/20041025-00/?p=37483
        // (I love you Raymond Chen)
        static __ImageBase: [u8; 64];
    }
    (unsafe { &__ImageBase }) as *const [u8; 64] as HINSTANCE
}

unsafe fn error_string_repr(err: DWORD) -> String {
    // This cast is no mistake, the function wants `LPWSTR *`, and not `LPWSTR`
    let mut buffer: *mut WCHAR = ptr::null_mut();
    let buf_ptr = (&mut buffer as *mut LPWSTR) as LPWSTR;

    // Query error string
    let char_count = FormatMessageW(
        FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
        ptr::null(),
        err,
        MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT).into(),
        buf_ptr,
        0,
        ptr::null_mut(),
    );
    debug_assert_ne!(char_count, 0);

    // Convert to `String`, free allocated OS buffer
    let mut message = Vec::new();
    lpcwstr_to_str(buffer, &mut message);
    LocalFree(buffer.cast());
    String::from_utf8_lossy(&message).into_owned()
}

fn str_to_wide_null<'s, 'v>(s: &str, buffer: &mut Vec<WCHAR>) -> LPCWSTR {
    // NOTE: Yes, indeed, `std::os::windows::ffi::OsStr(ing)ext` does exist in the standard library,
    // but it requires you to fit your data in the OsStr(ing) model and it's not hyper optimized
    // unlike mb2wc with handwritten SSE (allegedly), alongside being the native conversion function

    // MultiByteToWideChar can't actually handle 0 length because 0 return means error
    if s.is_empty() {
        return EMPTY_WIDE_STRING.as_ptr()
    }

    unsafe {
        let lpcstr: LPCSTR = s.as_ptr().cast();
        let str_len = s.len() as c_int;
        debug_assert!(s.len() <= c_int::max_value() as usize, "string length overflows c_int");

        // Calculate buffer size
        let wchar_count = MultiByteToWideChar(
            CP_UTF8, 0, lpcstr, str_len,
            ptr::null_mut(), 0, // buffer == NULL means query size
        ) as usize;
        debug_assert_ne!(0, wchar_count, "error in MultiByteToWideChar");

        // Preallocate enough space (including null terminator past end)
        let wchar_count_null = wchar_count + 1;
        buffer.clear();
        buffer.reserve(wchar_count_null);

        // Write to our buffer
        let chars_written = MultiByteToWideChar(
            CP_UTF8, 0, lpcstr, str_len,
            buffer.as_mut_ptr(), wchar_count as c_int,
        );
        buffer.set_len(wchar_count);
        debug_assert_eq!(wchar_count, chars_written as usize, "invalid char count received");

        // Filter nulls, as Rust allows them in &str
        for x in &mut *buffer {
            if *x == 0x00 {
                *x = b' ' as WCHAR; // 0x00 => Space
            }
        }

        // Add null terminator & yield
        *buffer.as_mut_ptr().add(wchar_count) = 0x00;
        buffer.set_len(wchar_count_null);
        buffer.as_ptr()
    }
}

fn lpcwstr_to_str(s: LPCWSTR, buffer: &mut Vec<u8>) {
    // This is more or less the inverse of `str_to_wide_null`, works the same way

    buffer.clear();
    unsafe {
        // Zero-length strings can't be processed like in MB2WC because 0 == Error...
        if *s == 0x00 {
            return
        }

        // Calculate buffer size
        let count = WideCharToMultiByte(
            CP_UTF8, 0, s, -1, ptr::null_mut(),
            0, ptr::null(), ptr::null_mut(),
        );
        debug_assert_ne!(count, 0, "failed to count wchars in wstr -> str");

        // Preallocate required amount, decode to buffer
        buffer.reserve(count as usize);
        let res = WideCharToMultiByte(
            CP_UTF8, 0, s, -1, buffer.as_mut_ptr(),
            count, ptr::null(), ptr::null_mut(),
        );
        debug_assert_ne!(res, 0, "failure in wchar -> str");
        buffer.set_len(count as usize - 1); // nulled input -> null in output
    }
}

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
            message: unsafe { error_string_repr(code) },
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

struct WindowUserData {
    close_reason: Option<CloseReason>,
    event_queue: Mutex<Vec<Event>>,
}

impl Default for WindowUserData {
    fn default() -> Self {
        Self {
            close_reason: None,
            event_queue: Mutex::new(Vec::with_capacity(EVENT_BUF_INITIAL_SIZE)),
        }
    }
}

impl WindowImpl for Window {
    fn events(&self) -> &[Event] {
        self.event_buffer.as_slice()
    }

    fn set_visible(&self, visible: bool) {
        let _ = visible;
    }

    fn swap_events(&mut self) {
        let user_data = unsafe { &mut *self.user_data.get() };
        let mut vec_lock = user_data.event_queue.lock().unwrap();
        mem::swap(&mut self.event_buffer, vec_lock.as_mut());
        vec_lock.clear();
        mem::drop(vec_lock);
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe fn user_data<'a>(hwnd: HWND) -> &'a mut WindowUserData {
        &mut *(get_window_data(hwnd, GWLP_USERDATA) as *mut WindowUserData)
    }

    #[inline]
    unsafe fn push_event(user_data: &mut WindowUserData, event: Event) {
        let mut lock = user_data.event_queue.lock().unwrap();
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
            set_window_data(hwnd, GWLP_USERDATA, params.user_data_ptr as usize);

            DefWindowProcW(hwnd, msg, wparam, lparam)
        },

        WM_NCDESTROY => {
            // finalize
            DefWindowProcW(hwnd, msg, wparam, lparam)
        },

        RAMEN_WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        },

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub(crate) fn make_window(builder: &WindowBuilder) -> Result<WindowRepr, Error> {
    // Condvar & mutex pair for receiving the `Result<WindowRepr, Error>` from spawned thread
    let signal = Arc::new((Mutex::<Option<Result<WindowRepr, Error>>>::new(None), Condvar::new()));

    let builder = builder.clone();
    let cond_pair = Arc::clone(&signal);
    let thread_builder = thread::Builder::new()
        .name(format!("Window Thread (Class \"{}\")", builder.__class_name.as_ref()));
    let window_thread = thread_builder.spawn(move || unsafe {
        // TODO: Sanitize reserved window classes
        let mut class_info = mem::MaybeUninit::<WNDCLASSEXW>::uninit();
        (&mut *class_info.as_mut_ptr()).cbSize = mem::size_of_val(&class_info) as DWORD;

        let mut class_name_buf = Vec::new();
        let class_name = str_to_wide_null(builder.__class_name.as_ref(), &mut class_name_buf);

        // Create the window class if it doesn't exist yet
        let class_registry_lock = CLASS_REGISTRY_LOCK.lock().unwrap();
        if GetClassInfoExW(this_hinstance(), class_name, class_info.as_mut_ptr()) == 0 {
            // The window class not existing sets the thread global error flag,
            // we clear it immediately to avoid any confusion down the line.
            SetLastError(ERROR_SUCCESS);

            // Fill in & register class (`cbSize` is set before this `if` block)
            let class = &mut *class_info.as_mut_ptr();
            class.style = CS_OWNDC;
            class.lpfnWndProc = window_proc;
            class.cbClsExtra = 0;
            class.cbWndExtra = 0;
            class.hInstance = this_hinstance();
            class.hIcon = ptr::null_mut();
            class.hCursor = ptr::null_mut();
            class.hbrBackground = ptr::null_mut();
            class.lpszMenuName = ptr::null_mut();
            class.lpszClassName = class_name;
            class.hIconSm = ptr::null_mut();

            // The fields on `WNDCLASSEXW` are valid so this can't fail
            RegisterClassExW(class);
        }
        mem::drop(class_registry_lock);

        let style = WS_OVERLAPPEDWINDOW | WS_VISIBLE;
        let style_ex = 0;

        let width = 1280;
        let height = 720;
        let mut title = Vec::new();
        let user_data: Box<cell::UnsafeCell<WindowUserData>> = Default::default();

        let builder_ptr = (&builder) as *const WindowBuilder;
        let user_data_ptr = user_data.as_ref().get();
        let mut params = WindowCreateParams { builder_ptr, user_data_ptr, error_return: None };

        // Creates the window - this waits for `WM_NCCREATE` & `WM_CREATE` to return (in that order)
        let hwnd = CreateWindowExW(
            style_ex,
            class_name,
            str_to_wide_null(builder.__title.as_ref(), &mut title),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            ptr::null_mut(),
            ptr::null_mut(),
            this_hinstance(),
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
        let mut lock = mutex.lock().unwrap();
        *lock = Some(match params.error_return {
            Some(err) => Err(err),
            None => Ok(Window {
                hwnd,
                thread: None,
                user_data,
                event_buffer: Vec::with_capacity(EVENT_BUF_INITIAL_SIZE),
            }),
        });
        condvar.notify_one();
        mem::drop(lock);

        // Release condvar + mutex pair so the `Arc` contents are deallocated once the outer fn returns
        mem::drop(cond_pair);

        // Free unused buffers
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
                    DispatchMessageW(&msg);
                },
            }
        }

        // Registered window classes are unregistered automatically when the process closes.
        // Until then, there's no reason not to have them around as the contents never vary.
        // > if something { UnregisterClassW(class_atom); }

        // TODO: Don't do this, obviously.
        ExitProcess(0);
    }).expect("Failed to spawn window thread");

    // Wait until the thread is done creating the window or notifying us why it couldn't do that
    let (mutex, condvar) = &*signal;
    let mut lock = mutex.lock().unwrap();
    loop {
        if let Some(result) = lock.take() {
            break result.map(|mut window| {
                window.thread = Some(window_thread);
                window
            })
        } else {
            lock = condvar.wait(lock).unwrap();
        }
    }
}

impl ops::Drop for Window {
    fn drop(&mut self) {
        unsafe {
            PostMessageW(self.hwnd, RAMEN_WM_CLOSE, 0, 0);
        }
        let _ = self.thread.take().map(thread::JoinHandle::join);
    }
}
