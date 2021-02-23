use super::api::*;
use crate::{monitor::Size};
use std::{mem, ptr, slice};

#[cfg(feature = "cursor-lock")]
use crate::window::CursorLock;

/// The base DPI at 100% scaling
pub const BASE_DPI: UINT = 96;

/// Empty wide string (to point to)
pub static WSTR_EMPTY: &[WCHAR] = &[0x00];

/// Retrieves the base module HINSTANCE.
#[inline]
pub fn this_hinstance() -> HINSTANCE {
    extern "system" {
        // Microsoft's linkers provide a static HINSTANCE to not have to query it at runtime.
        // Source: https://devblogs.microsoft.com/oldnewthing/20041025-00/?p=37483
        // (I love you Raymond Chen)
        static __ImageBase: [u8; 64];
    }
    (unsafe { &__ImageBase }) as *const [u8; 64] as HINSTANCE
}

// TODO: make sure this actually works
pub unsafe fn error_string_repr(err: DWORD) -> String {
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
    let _ = LocalFree(buffer.cast());
    String::from_utf8_lossy(&message).into_owned()
}

pub fn str_to_wide_null(src: &str, buffer: &mut Vec<WCHAR>) -> LPCWSTR {
    // NOTE: Yes, indeed, `std::os::windows::ffi::OsStr(ing)ext` does exist in the standard library,
    // but it requires you to fit your data in the OsStr(ing) model and it's not hyper optimized
    // unlike mb2wc with handwritten SSE (allegedly), alongside being the native conversion function

    // MultiByteToWideChar can't actually handle 0 length because 0 return means error
    if src.is_empty() || src.len() > c_int::max_value() as usize {
        return WSTR_EMPTY.as_ptr()
    }

    unsafe {
        let str_ptr: LPCSTR = src.as_ptr().cast();
        let str_len = src.len() as c_int;

        // Calculate buffer size
        let req_buffer_size = MultiByteToWideChar(
            CP_UTF8, 0,
            str_ptr, str_len,
            ptr::null_mut(), 0, // `lpWideCharStr == NULL` means query size
        ) as usize + 1; // +1 for null terminator

        // Ensure buffer capacity
        buffer.clear();
        buffer.reserve(req_buffer_size);

        // Write to our buffer
        let chars_written = MultiByteToWideChar(
            CP_UTF8, 0,
            str_ptr, str_len,
            buffer.as_mut_ptr(), req_buffer_size as c_int,
        ) as usize;

        // Filter nulls, as Rust allows them in &str
        for x in slice::from_raw_parts_mut(buffer.as_mut_ptr(), chars_written) {
            if *x == 0x00 {
                *x = b' ' as WCHAR; // 0x00 => Space
            }
        }

        // Add null terminator & yield
        *buffer.as_mut_ptr().add(chars_written) = 0x00;
        buffer.set_len(req_buffer_size);
        buffer.as_ptr()
    }
}

pub fn lpcwstr_to_str(src: LPCWSTR, buffer: &mut Vec<u8>) {
    // This is more or less the inverse of `str_to_wide_null`, works the same way

    buffer.clear();
    unsafe {
        // Zero-length strings can't be processed like in MB2WC because 0 == Error...
        if *src == 0x00 {
            return
        }

        // Calculate buffer size
        let count = WideCharToMultiByte(
            CP_UTF8, 0,
            src, -1, ptr::null_mut(),
            0, ptr::null(), ptr::null_mut(),
        );

        // Preallocate required amount, decode to buffer
        buffer.reserve(count as usize);
        let _ = WideCharToMultiByte(
            CP_UTF8, 0,
            src, -1, buffer.as_mut_ptr(),
            count, ptr::null(), ptr::null_mut(),
        );
        buffer.set_len(count as usize - 1); // nulled input -> null in output
    }
}

pub unsafe fn is_windows_ver_or_greater(dl: &Win32DL, major: WORD, minor: WORD, sp_major: WORD) -> bool {
    let mut osvi: OSVERSIONINFOEXW = mem::zeroed();
    osvi.dwOSVersionInfoSize = mem::size_of_val(&osvi) as DWORD;
    osvi.dwMajorVersion = major.into();
    osvi.dwMinorVersion = minor.into();
    osvi.wServicePackMajor = sp_major.into();

    let mask = VER_MAJORVERSION | VER_MINORVERSION | VER_SERVICEPACKMAJOR;
    let mut cond = VerSetConditionMask(0, VER_MAJORVERSION, VER_GREATER_EQUAL);
    cond = VerSetConditionMask(cond, VER_MINORVERSION, VER_GREATER_EQUAL);
    cond = VerSetConditionMask(cond, VER_SERVICEPACKMAJOR, VER_GREATER_EQUAL);

    dl.RtlVerifyVersionInfo(&mut osvi, mask, cond) == Some(0)
}

pub unsafe fn is_win10_ver_or_greater(dl: &Win32DL, build: WORD) -> bool {
    let mut osvi: OSVERSIONINFOEXW = mem::zeroed();
    osvi.dwOSVersionInfoSize = mem::size_of_val(&osvi) as DWORD;
    osvi.dwMajorVersion = 10;
    osvi.dwMinorVersion = 0;
    osvi.dwBuildNumber = build.into();

    let mask = VER_MAJORVERSION | VER_MINORVERSION | VER_BUILDNUMBER;
    let mut cond = VerSetConditionMask(0, VER_MAJORVERSION, VER_GREATER_EQUAL);
    cond = VerSetConditionMask(cond, VER_MINORVERSION, VER_GREATER_EQUAL);
    cond = VerSetConditionMask(cond, VER_BUILDNUMBER, VER_GREATER_EQUAL);

    dl.RtlVerifyVersionInfo(&mut osvi, mask, cond) == Some(0)
}

/// Due to legacy reasons, the close button is a system menu item and not a window style.
/// This function is for turning it on and off (enabled and disabled, rather).
pub unsafe fn set_close_button(hwnd: HWND, enabled: bool) {
    let menu: HMENU = GetSystemMenu(hwnd, FALSE);
    let flag = if enabled {
        MF_BYCOMMAND | MF_ENABLED
    } else {
        MF_BYCOMMAND | MF_DISABLED | MF_GRAYED
    };
    let _ = EnableMenuItem(menu, SC_CLOSE as UINT, flag);
}

/// Due to legacy reasons, changing the window frame does nothing (since it's cached),
/// until you update it with SetWindowPos with just "oh yeah, the frame changed, that's about it".
#[inline]
pub unsafe fn ping_window_frame(hwnd: HWND) {
    const MASK: UINT = SWP_NOMOVE | SWP_NOSIZE | SWP_NOOWNERZORDER | SWP_NOZORDER | SWP_FRAMECHANGED;
    let _ = SetWindowPos(hwnd, ptr::null_mut(), 0, 0, 0, 0, MASK);
}

#[inline]
pub fn rect_to_size2d(rect: &RECT) -> (LONG, LONG) {
    (rect.right - rect.left, rect.bottom - rect.top)
}

pub unsafe fn client_area_screen_space(hwnd: HWND) -> RECT {
    let mut client_area: RECT = mem::zeroed();
    let _ = GetClientRect(hwnd, &mut client_area);
    let _ = ClientToScreen(hwnd, (&mut client_area.left) as *mut _ as *mut POINT);
    let _ = ClientToScreen(hwnd, (&mut client_area.right) as *mut _ as *mut POINT);
    client_area
}


#[cfg(feature = "cursor-lock")]
pub unsafe fn update_cursor_lock(hwnd: HWND, kind: Option<CursorLock>, new: bool) {
    match kind {
        Some(CursorLock::Constrain) => {
            let client_area = client_area_screen_space(hwnd);
            let _ = ClipCursor(&client_area);
        },
        Some(CursorLock::Center) => {
            let client_area = client_area_screen_space(hwnd);
            let (width, height) = rect_to_size2d(&client_area);
            let _ = SetCursorPos(client_area.left + (width / 2), client_area.top + (height / 2));
        },
        None if new => {
            let _ = ClipCursor(ptr::null());
        },
        None => (),
    }
}

#[cfg(feature = "cursor-lock")]
pub unsafe fn is_cursor_in_titlebar(hwnd: HWND) -> bool {
    let mut title_bar: TITLEBARINFO = mem::zeroed();
    title_bar.cbSize = mem::size_of_val(&title_bar) as DWORD;
    let _ = GetTitleBarInfo(hwnd, &mut title_bar);
    let mut mouse_pos: POINT = mem::zeroed();
    let _ = GetCursorPos(&mut mouse_pos);
    PtInRect(&title_bar.rcTitleBar, POINT { ..mouse_pos }) != 0
}

pub enum DpiMode {
    Unsupported,
    System,
    PerMonitorV1,
    PerMonitorV2,
}

pub struct Win32 {
    /// Dynamically linked Win32 functions that might not be available on all systems.
    pub dl: Win32DL,

    /// The DPI mode that's enabled process-wide. The newest available is selected.
    /// This is done at runtime, and not in the manifest, because that's incredibly stupid.
    pub dpi_mode: DpiMode,

    // Cached version checks, as the system can't magically upgrade without restarting
    pub at_least_vista: bool,
    pub at_least_8_point_1: bool,
    pub at_least_anniversary_update: bool,
    pub at_least_creators_update: bool,
}

impl Win32 {
    pub fn new() -> Self {
        const VISTA_MAJ: WORD = (_WIN32_WINNT_VISTA >> 8) & 0xFF;
        const VISTA_MIN: WORD = _WIN32_WINNT_VISTA & 0xFF;
        const W81_MAJ: WORD = (_WIN32_WINNT_WINBLUE >> 8) & 0xFF;
        const W81_MIN: WORD = _WIN32_WINNT_WINBLUE & 0xFF;

        unsafe {
            let dl = Win32DL::link();

            let at_least_vista = is_windows_ver_or_greater(&dl, VISTA_MAJ, VISTA_MIN, 0);
            let at_least_8_point_1 = is_windows_ver_or_greater(&dl, W81_MAJ, W81_MIN, 0);
            let at_least_anniversary_update = is_win10_ver_or_greater(&dl, 14393);
            let at_least_creators_update = is_win10_ver_or_greater(&dl, 15063);

            let dpi_mode = if at_least_creators_update {
                let _ = dl.SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
                DpiMode::PerMonitorV2
            } else if at_least_8_point_1 {
                let _ = dl.SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);
                DpiMode::PerMonitorV1
            } else if at_least_vista {
                let _ = dl.SetProcessDPIAware();
                DpiMode::System
            } else {
                DpiMode::Unsupported
            };

            Self {
                dl,
                dpi_mode,
                at_least_vista,
                at_least_8_point_1,
                at_least_anniversary_update,
                at_least_creators_update,
            }
        }
    }

    pub unsafe fn adjust_window_for_dpi(&self, size: Size, style: DWORD, style_ex: DWORD, dpi: UINT) -> (LONG, LONG) {
        let (width, height) = size.scale_if_logical(dpi as f64 / BASE_DPI as f64);
        let mut window = RECT { left: 0, top: 0, bottom: height as LONG, right: width as LONG };
        if match self.dpi_mode {
            // Non-client area DPI scaling is enabled in PMv1 Win10 1607+ and PMv2 (any).
            // For PMv1, this is done with EnableNonClientDpiScaling at WM_NCCREATE.
            DpiMode::PerMonitorV1 if self.at_least_anniversary_update => true,
            DpiMode::PerMonitorV2 => true,
            _ => false,
        } {
            let _ = self.dl.AdjustWindowRectExForDpi(&mut window, style, FALSE, style_ex, dpi);
        } else {
            // TODO: This *is* correct for old PMv1, right? How does broken NC scaling work?
            let _ = AdjustWindowRectEx(&mut window, style, FALSE, style_ex);
        }
        rect_to_size2d(&window)
    }
}

impl Default for Win32 {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
