#![allow(bad_style, dead_code, overflowing_literals)]

// Opaque handles
macro_rules! def_handle {
    // documented, exported
    ($doc: literal, $name: ident, $private_name: ident $(,)?) => {
        #[doc(hidden)]
        pub enum $private_name {}
        #[doc = $doc]
        pub type $name = *mut $private_name;
    };
    // internal
    ($name: ident, $private_name: ident $(,)?) => {
        #[doc(hidden)]
        pub enum $private_name {}
        pub type $name = *mut $private_name;
    };
}
def_handle!("Opaque handle to the executable file in memory.", HINSTANCE, HINSTANCE__);
def_handle!("Opaque handle to a monitor.", HMONITOR, HMONITOR__);
def_handle!("Opaque handle to a window.", HWND, HWND__);
def_handle!(DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT__);
def_handle!(HBRUSH, HBRUSH__);
def_handle!(HDC, HDC__);
def_handle!(HHOOK, HHOOK__);
def_handle!(HICON, HICON__);
def_handle!(HMENU, HMENU__);
def_handle!(HMODULE, HMODULE__);
pub type HCURSOR = HICON;

// Typedefs
use core::ffi::c_void;
pub type c_char = i8;
pub type c_schar = i8;
pub type c_uchar = u8;
pub type c_short = i16;
pub type c_ushort = u16;
pub type c_int = i32;
pub type c_uint = u32;
pub type c_long = i32;
pub type c_ulong = u32;
pub type c_longlong = i64;
pub type c_ulonglong = u64;
pub type wchar_t = u16;
pub type ATOM = WORD;
pub type BOOL = c_int;
pub type BYTE = c_uchar;
pub type CHAR = c_char;
pub type DWORD = c_ulong;
pub type HANDLE = *mut c_void;
pub type HLOCAL = HANDLE;
pub type HRESULT = c_long;
pub type INT = c_int;
pub type LANGID = USHORT;
pub type LONG = c_long;
pub type LONG_PTR = isize;
pub type LPARAM = LONG_PTR;
pub type LPCSTR = *const CHAR;
pub type LPCVOID = *const c_void;
pub type LPCWSTR = *const WCHAR;
pub type LPVOID = *mut c_void;
pub type LPWSTR = *mut WCHAR;
pub type LRESULT = LONG_PTR;
pub type UINT = c_uint;
pub type UINT_PTR = usize;
pub type ULONG_PTR = usize;
pub type USHORT = c_ushort;
pub type WCHAR = wchar_t;
pub type WORD = c_ushort;
pub type WPARAM = UINT_PTR;

// Function typedefs
pub type HOOKPROC = unsafe extern "system" fn(c_int, WPARAM, LPARAM) -> LRESULT;
pub type WNDPROC = unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT;

// Constants
pub const CP_UTF8: DWORD = 65001;
pub const CS_OWNDC: UINT = 0x0020;
pub const CW_USEDEFAULT: c_int = 0x80000000;
pub const ERROR_SUCCESS: DWORD = 0; // lol
pub const FORMAT_MESSAGE_ALLOCATE_BUFFER: DWORD = 0x00000100;
pub const FORMAT_MESSAGE_FROM_SYSTEM: DWORD = 0x00001000;
pub const FORMAT_MESSAGE_IGNORE_INSERTS: DWORD = 0x00000200;
pub const GCL_CBCLSEXTRA: c_int = -20;
pub const GWLP_USERDATA: c_int = -21;
pub const HCBT_DESTROYWND: c_int = 4;
pub const LANG_NEUTRAL: USHORT = 0x00;
pub const SUBLANG_DEFAULT: USHORT = 0x01;
pub const SW_HIDE: c_int = 0;
pub const SW_SHOW: c_int = 5;
pub const WH_CBT: c_int = 5;
pub const WM_CREATE: UINT = 0x0001;
pub const WM_DESTROY: UINT = 0x0002;
pub const WM_CLOSE: UINT = 0x0010;
pub const WM_NCCREATE: UINT = 0x0081;
pub const WM_NCDESTROY: UINT = 0x0082;
pub const WM_USER: UINT = 0x0400;
pub const WS_BORDER: DWORD = 0x00800000;
pub const WS_CAPTION: DWORD = 0x00C00000;
pub const WS_CHILD: DWORD = 0x40000000;
pub const WS_CLIPCHILDREN: DWORD = 0x02000000;
pub const WS_CLIPSIBLINGS: DWORD = 0x04000000;
pub const WS_DISABLED: DWORD = 0x08000000;
pub const WS_DLGFRAME: DWORD = 0x00400000;
pub const WS_GROUP: DWORD = 0x00020000;
pub const WS_HSCROLL: DWORD = 0x00100000;
pub const WS_ICONIC: DWORD = WS_MINIMIZE;
pub const WS_MAXIMIZE: DWORD = 0x01000000;
pub const WS_MAXIMIZEBOX: DWORD = 0x00010000;
pub const WS_MINIMIZE: DWORD = 0x20000000;
pub const WS_MINIMIZEBOX: DWORD = 0x00020000;
pub const WS_OVERLAPPED: DWORD = 0x00000000;
pub const WS_OVERLAPPEDWINDOW: DWORD =
    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX;
pub const WS_POPUP: DWORD = 0x80000000;
pub const WS_SIZEBOX: DWORD = WS_THICKFRAME;
pub const WS_SYSMENU: DWORD = 0x00080000;
pub const WS_TABSTOP: DWORD = 0x00010000;
pub const WS_THICKFRAME: DWORD = 0x00040000;
pub const WS_TILED: DWORD = WS_OVERLAPPED;
pub const WS_TILEDWINDOW: DWORD = WS_OVERLAPPEDWINDOW;
pub const WS_VISIBLE: DWORD = 0x10000000;
pub const WS_VSCROLL: DWORD = 0x00200000;

// Structs
#[repr(C)]
pub struct POINT {
    pub x: LONG,
    pub y: LONG,
}
#[repr(C)]
pub struct RECT {
    pub left: LONG,
    pub top: LONG,
    pub right: LONG,
    pub bottom: LONG,
}
#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
}
#[repr(C)]
pub struct WNDCLASSEXW {
    pub cbSize: UINT,
    pub style: UINT,
    pub lpfnWndProc: WNDPROC,
    pub cbClsExtra: c_int,
    pub cbWndExtra: c_int,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: HCURSOR,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: LPCWSTR,
    pub lpszClassName: LPCWSTR,
    pub hIconSm: HICON,
}

// Statically linked imports
#[link(name = "Kernel32")]
extern "system" {
    pub fn GetLastError() -> DWORD;
    pub fn SetLastError(dwErrCode: DWORD);
    pub fn ExitProcess(uExitCode: UINT);
    pub fn GetCurrentThreadId() -> DWORD;

    pub fn LocalFree(hMem: HLOCAL) -> HLOCAL;
    pub fn FormatMessageW(
        dwFlags: DWORD,
        lpSource: LPCVOID,
        dwMessageId: DWORD,
        dwLanguageId: DWORD,
        lpBuffer: LPWSTR,
        nSize: DWORD,
        Arguments: *mut c_void, // `va_list` (we don't use it)
    ) -> DWORD;
    pub fn MultiByteToWideChar(
        CodePage: UINT,
        dwFlags: DWORD,
        lpMultiByteStr: LPCSTR,
        cbMultiByte: c_int,
        lpWideCharStr: LPWSTR,
        cchWideChar: c_int,
    ) -> c_int;
    pub fn WideCharToMultiByte(
        CodePage: UINT,
        dwFlags: DWORD,
        lpWideCharStr: LPCWSTR,
        cchWideChar: c_int,
        lpMultiByteStr: *mut u8,
        cbMultiByte: c_int,
        lpDefaultChar: LPCSTR,
        lpUsedDefaultChar: *mut BOOL,
    ) -> c_int;
}
#[link(name = "User32")]
extern "system" {
    // Window class management
    pub fn GetClassInfoExW(hinst: HINSTANCE, lpszClass: LPCWSTR, lpwcx: *mut WNDCLASSEXW) -> BOOL;
    pub fn RegisterClassExW(lpWndClass: *const WNDCLASSEXW) -> ATOM;

    // Window management
    pub fn CreateWindowExW(
        dwExStyle: DWORD,
        lpClassName: LPCWSTR,
        lpWindowName: LPCWSTR,
        dwStyle: DWORD,
        x: c_int,
        y: c_int,
        nWidth: c_int,
        nHeight: c_int,
        hWndParent: HWND,
        hMenu: HMENU,
        hInstance: HINSTANCE,
        lpParam: LPVOID,
    ) -> HWND;
    pub fn DestroyWindow(hWnd: HWND) -> BOOL;

    // Hooking API
    pub fn CallNextHookEx(hhk: HHOOK, nCode: c_int, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn SetWindowsHookExW(idHook: c_int, lpfn: HOOKPROC, hmod: HINSTANCE, dwThreadId: DWORD) -> HHOOK;
    pub fn UnhookWindowsHookEx(hhk: HHOOK) -> BOOL;

    // Message loop
    pub fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn GetMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: UINT, wMsgFilterMax: UINT) -> BOOL;
    pub fn PostMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> BOOL;
    pub fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn DispatchMessageW(lpmsg: *const MSG) -> LRESULT;
    pub fn PostQuitMessage(nExitCode: c_int);

    // Message loop utility
    pub fn ShowWindow(hWnd: HWND, nCmdShow: c_int) -> BOOL;
    pub fn ShowWindowAsync(hWnd: HWND, nCmdShow: c_int) -> BOOL;

    // Class/window storage manipulation
    pub fn GetClassLongW(hWnd: HWND, nIndex: c_int) -> DWORD;
    pub fn SetClassLongW(hWnd: HWND, nIndex: c_int, dwNewLong: LONG) -> DWORD;
    pub fn GetWindowLongW(hWnd: HWND, nIndex: c_int) -> LONG;
    pub fn SetWindowLongW(hWnd: HWND, nIndex: c_int, dwNewLong: LONG) -> LONG;
    #[cfg(target_pointer_width = "64")]
    pub fn GetClassLongPtrW(hWnd: HWND, nIndex: c_int) -> ULONG_PTR;
    #[cfg(target_pointer_width = "64")]
    pub fn SetClassLongPtrW(hWnd: HWND, nIndex: c_int, dwNewLong: LONG_PTR) -> ULONG_PTR;
    #[cfg(target_pointer_width = "64")]
    pub fn GetWindowLongPtrW(hWnd: HWND, nIndex: c_int) -> LONG_PTR;
    #[cfg(target_pointer_width = "64")]
    pub fn SetWindowLongPtrW(hWnd: HWND, nIndex: c_int, dwNewLong: LONG_PTR) -> LONG_PTR;
}

// These functions are #define'd as one or the other based on arch in the Win32 headers.
// Unfortunately, their signatures do not match, so it's better to rewrap it like this.
// Both LONG and LONG_PTR are equivalent to the size of usize on their respective targets.
#[cfg(target_pointer_width = "32")]
pub unsafe fn get_class_data(hwnd: HWND, offset: c_int) -> usize {
    GetClassLongW(hwnd, offset) as usize
}
#[cfg(target_pointer_width = "64")]
pub unsafe fn get_class_data(hwnd: HWND, offset: c_int) -> usize {
    GetClassLongPtrW(hwnd, offset) as usize
}
#[cfg(target_pointer_width = "32")]
pub unsafe fn set_class_data(hwnd: HWND, offset: c_int, data: usize) -> usize {
    SetClassLongW(hwnd, offset, data as LONG) as usize
}
#[cfg(target_pointer_width = "64")]
pub unsafe fn set_class_data(hwnd: HWND, offset: c_int, data: usize) -> usize {
    SetClassLongPtrW(hwnd, offset, data as LONG_PTR) as usize
}
#[cfg(target_pointer_width = "32")]
pub unsafe fn get_window_data(hwnd: HWND, offset: c_int) -> usize {
    GetWindowLongW(hwnd, offset) as usize
}
#[cfg(target_pointer_width = "64")]
pub unsafe fn get_window_data(hwnd: HWND, offset: c_int) -> usize {
    GetWindowLongPtrW(hwnd, offset) as usize
}
#[cfg(target_pointer_width = "32")]
pub unsafe fn set_window_data(hwnd: HWND, offset: c_int, data: usize) -> usize {
    SetWindowLongW(hwnd, offset, data as LONG) as usize
}
#[cfg(target_pointer_width = "64")]
pub unsafe fn set_window_data(hwnd: HWND, offset: c_int, data: usize) -> usize {
    SetWindowLongPtrW(hwnd, offset, data as LONG_PTR) as usize
}

// Macros
#[inline]
pub const fn MAKELANGID(p: USHORT, s: USHORT) -> LANGID {
    (s << 10) | p
}
