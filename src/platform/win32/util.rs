use super::api::*;
use std::ptr;

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
    LocalFree(buffer.cast());
    String::from_utf8_lossy(&message).into_owned()
}

pub fn str_to_wide_null<'s, 'v>(s: &str, buffer: &mut Vec<WCHAR>) -> LPCWSTR {
    // NOTE: Yes, indeed, `std::os::windows::ffi::OsStr(ing)ext` does exist in the standard library,
    // but it requires you to fit your data in the OsStr(ing) model and it's not hyper optimized
    // unlike mb2wc with handwritten SSE (allegedly), alongside being the native conversion function

    // MultiByteToWideChar can't actually handle 0 length because 0 return means error
    if s.is_empty() {
        return WSTR_EMPTY.as_ptr()
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

pub fn lpcwstr_to_str(s: LPCWSTR, buffer: &mut Vec<u8>) {
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
