use super::api::*;
use std::{ptr, slice};

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
