//! Windows API specific implementations and API extensions.

#[path = "bindings/win32.rs"]
mod bindings;
use self::bindings::*;

use crate::{
    error::Error,
    helpers::LazyCell,
    window::{WindowBuilder, WindowImpl},
};
use std::{cell, ffi, mem, ops, ptr, thread};
use std::sync::{Arc, Condvar, Mutex}; // move later

/// Global lock used to synchronize classes being registered or queried.
static CLASS_REGISTRY_LOCK: LazyCell<Mutex<()>> = LazyCell::new(Default::default);

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

fn widen_string(s: &str) -> impl Iterator<Item = WCHAR> + '_ {
    use std::os::windows::ffi::OsStrExt;
    <str as AsRef<ffi::OsStr>>::as_ref(s)
        .encode_wide()
        .chain(Some(0x00))
}

pub(crate) struct Window {
    // guts
    class: ATOM,
    hwnd: HWND,
    thread: Option<thread::JoinHandle<()>>,

    // api
    user_data: Box<cell::UnsafeCell<WindowUserData>>,
}
unsafe impl Send for Window {}
unsafe impl Sync for Window {}

pub(crate) type WindowRepr = Window;

struct WindowCreateParams {
    builder_ptr: *const WindowBuilder,
    user_data_ptr: *mut WindowUserData,
}

struct WindowUserData {
    destroy_class: bool,
}

impl Default for WindowUserData {
    fn default() -> Self {
        Self {
            destroy_class: false,
        }
    }
}

impl WindowImpl for Window {
    fn set_visible(&self, visible: bool) {
        let _ = visible;
    }

    fn swap_events(&mut self) {
        // ...
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe fn user_data<'a>(hwnd: HWND) -> &'a mut WindowUserData {
        &mut *(get_window_data(hwnd, GWLP_USERDATA) as *mut WindowUserData)
    }

    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        },

        WM_CLOSE => {
            DestroyWindow(hwnd);
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
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub(crate) fn make_window(builder: &WindowBuilder) -> Result<WindowRepr, Error> {
    let signal = Arc::new((Mutex::<Option<Result<WindowRepr, Error>>>::new(None), Condvar::new()));

    let builder = builder.clone();
    let cond_pair = Arc::clone(&signal);
    let window_thread = thread::spawn(move || unsafe {
        // TODO: Sanitize reserved window classes
        let mut class_info = mem::MaybeUninit::<WNDCLASSEXW>::uninit();
        let class_name = widen_string(builder.__class_name.as_ref()).collect::<Vec<_>>();
        (&mut *class_info.as_mut_ptr()).cbSize = mem::size_of_val(&class_info) as DWORD;

        // Create the window class if it doesn't exist yet
        let class_atom: ATOM;
        let class_registry_lock = CLASS_REGISTRY_LOCK.lock().unwrap();
        if GetClassInfoExW(this_hinstance(), class_name.as_ptr(), class_info.as_mut_ptr()) == 0 {
            // The window class not existing sets the thread global error flag,
            // we clear it immediately to avoid any confusion down the line.
            SetLastError(ERROR_SUCCESS);

            // Fill in & register class (cbSize is set before this `if` block)
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
            class.lpszClassName = class_name.as_ptr();
            class.hIconSm = ptr::null_mut();

            class_atom = RegisterClassExW(class);
            if class_atom == 0 {
                todo!("handle the class not registering")
            }
        } else {
            // If the class already exists, query the atom for the class name
            class_atom = GlobalFindAtomW(class_name.as_ptr());
        }
        mem::drop(class_registry_lock);

        // TODO: what
        assert_ne!(class_atom, 0);

        // `class_name` is no longer needed, as `class_atom` maps to a copy managed by the OS
        mem::drop(class_name);

        let style = WS_OVERLAPPEDWINDOW | WS_VISIBLE;
        let style_ex = 0;

        let width = 1280;
        let height = 720;
        let title = widen_string(builder.__title.as_ref()).collect::<Vec<_>>();
        let user_data: Box<cell::UnsafeCell<WindowUserData>> = Default::default();

        let builder_ptr = (&builder) as *const WindowBuilder;
        let user_data_ptr = user_data.as_ref().get();
        let mut params = WindowCreateParams { builder_ptr, user_data_ptr };

        let hwnd = CreateWindowExW(
            style_ex,
            class_atom as LPCWSTR,
            title.as_ptr(),
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

        // TODO: if hwnd bad

        // Yield window struct, signal outer function
        let (mutex, condvar) = &*cond_pair;
        let mut lock = mutex.lock().unwrap();
        *lock = Some(Ok(Window {
            class: class_atom,
            hwnd,
            thread: None,
            user_data,
        }));
        condvar.notify_one();
        mem::drop(lock);

        // Release condvar + mutex pair so the Arc is deallocated once the outer function returns
        mem::drop(cond_pair);

        // Run message loop until error or exit
        let mut msg = mem::MaybeUninit::<MSG>::zeroed().assume_init();
        'message_loop: loop {
            // `HWND hWnd` is set to NULL here to query all messages on the thread,
            // as the exit condition/signal (WM_QUIT) is not associated with any window.
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
    });

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
        // unsafe {
        //     // TODO: X_WM_CLOSE, etc
        // }
        let _ = self.thread.take().map(thread::JoinHandle::join);
    }
}
