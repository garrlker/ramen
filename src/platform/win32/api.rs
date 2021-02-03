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
def_handle!(FARPROC, __some_function);
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
pub type NTSTATUS = LONG;
pub type PROCESS_DPI_AWARENESS = u32;
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
pub const _WIN32_WINNT_VISTA: WORD = 0x0600;
pub const _WIN32_WINNT_WINBLUE: WORD = 0x0603;
pub const CP_UTF8: DWORD = 65001;
pub const CS_OWNDC: UINT = 0x0020;
pub const CW_USEDEFAULT: c_int = 0x80000000;
pub const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: DPI_AWARENESS_CONTEXT = -4isize as _;
pub const E_INVALIDARG: HRESULT = 0x80070057;
pub const ERROR_SUCCESS: DWORD = 0; // lol
pub const FALSE: BOOL = 0;
pub const FORMAT_MESSAGE_ALLOCATE_BUFFER: DWORD = 0x00000100;
pub const FORMAT_MESSAGE_FROM_SYSTEM: DWORD = 0x00001000;
pub const FORMAT_MESSAGE_IGNORE_INSERTS: DWORD = 0x00000200;
pub const GCL_CBCLSEXTRA: c_int = -20;
pub const GWL_EXSTYLE: c_int = -20;
pub const GWL_STYLE: c_int = -16;
pub const GWL_USERDATA: c_int = -21;
pub const HCBT_DESTROYWND: c_int = 4;
pub const LANG_NEUTRAL: USHORT = 0x00;
pub const MF_BYCOMMAND: UINT = 0x00000000;
pub const MF_DISABLED: UINT = 0x00000002;
pub const MF_ENABLED: UINT = 0x00000000;
pub const MF_GRAYED: UINT = 0x00000001;
pub const PROCESS_PER_MONITOR_DPI_AWARE: PROCESS_DPI_AWARENESS = 2;
pub const PROCESS_SYSTEM_DPI_AWARE: PROCESS_DPI_AWARENESS = 1;
pub const SUBLANG_DEFAULT: USHORT = 0x01;
pub const S_OK: HRESULT = 0;
pub const SC_CLOSE: WPARAM = 0xF060;
pub const SW_HIDE: c_int = 0;
pub const SW_SHOW: c_int = 5;
pub const SWP_ASYNCWINDOWPOS: UINT = 0x4000;
pub const SWP_DEFERERASE: UINT = 0x2000;
pub const SWP_DRAWFRAME: UINT = SWP_FRAMECHANGED;
pub const SWP_FRAMECHANGED: UINT = 0x0020;
pub const SWP_HIDEWINDOW: UINT = 0x0080;
pub const SWP_NOACTIVATE: UINT = 0x0010;
pub const SWP_NOCOPYBITS: UINT = 0x0100;
pub const SWP_NOMOVE: UINT = 0x0002;
pub const SWP_NOOWNERZORDER: UINT = 0x0200;
pub const SWP_NOREDRAW: UINT = 0x0008;
pub const SWP_NOREPOSITION: UINT = SWP_NOOWNERZORDER;
pub const SWP_NOSENDCHANGING: UINT = 0x0400;
pub const SWP_NOSIZE: UINT = 0x0001;
pub const SWP_NOZORDER: UINT = 0x0004;
pub const SWP_SHOWWINDOW: UINT = 0x0040;
pub const TRUE: BOOL = 1;
pub const VER_BUILDNUMBER: DWORD = 0x0000004;
pub const VER_GREATER_EQUAL: BYTE = 3;
pub const VER_MAJORVERSION: DWORD = 0x0000002;
pub const VER_MINORVERSION: DWORD = 0x0000001;
pub const VER_SERVICEPACKMAJOR: DWORD = 0x0000020;
pub const VER_SERVICEPACKMINOR: DWORD = 0x0000010;
pub const WH_CBT: c_int = 5;
pub const WM_NULL: UINT = 0x0000;
pub const WM_CREATE: UINT = 0x0001;
pub const WM_DESTROY: UINT = 0x0002;
pub const WM_MOVE: UINT = 0x0003;
// !! no 0x0004 event !!
pub const WM_SIZE: UINT = 0x0005;
pub const WM_ACTIVATE: UINT = 0x0006;
pub const WM_SETTEXT: UINT = 0x000C;
pub const WM_CLOSE: UINT = 0x0010;
pub const WM_SHOWWINDOW: UINT = 0x0018;
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
pub const WS_EX_LAYOUTRTL: DWORD = 0x00400000;
pub const WS_EX_TOOLWINDOW: DWORD = 0x00000080;
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
pub struct OSVERSIONINFOEXW {
    pub dwOSVersionInfoSize: DWORD,
    pub dwMajorVersion: DWORD,
    pub dwMinorVersion: DWORD,
    pub dwBuildNumber: DWORD,
    pub dwPlatformId: DWORD,
    pub szCSDVersion: [WCHAR; 128],
    pub wServicePackMajor: WORD,
    pub wServicePackMinor: WORD,
    pub wSuiteMask: WORD,
    pub wProductType: BYTE,
    pub wReserved: BYTE,
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

    pub fn GetProcAddress(hModule: HMODULE, lpProcName: LPCSTR) -> FARPROC;
    pub fn LoadLibraryExA(lpLibFileName: LPCSTR, hFile: HANDLE, dwFlags: DWORD) -> HMODULE;
    pub fn VerSetConditionMask(ConditionMask: c_ulonglong, TypeMask: DWORD, Condition: BYTE) -> c_ulonglong;

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
    pub fn AdjustWindowRectEx(lpRect: *mut RECT, dwStyle: DWORD, bMenu: BOOL, dwExStyle: DWORD) -> BOOL;
    pub fn SetWindowPos(hWnd: HWND, hWndInsertAfter: HWND, X: c_int, Y: c_int, cx: c_int, cy: c_int, uFlags: UINT) -> BOOL;
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

    // Misc legacy garbage
    pub fn EnableMenuItem(hMenu: HMENU, uIDEnableItem: UINT, uEnable: UINT) -> BOOL;
    pub fn GetSystemMenu(hWnd: HWND, bRevert: BOOL) -> HMENU;

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
#[inline]
pub unsafe fn get_class_data(hwnd: HWND, offset: c_int) -> usize {
    GetClassLongW(hwnd, offset) as usize
}
#[cfg(target_pointer_width = "64")]
#[inline]
pub unsafe fn get_class_data(hwnd: HWND, offset: c_int) -> usize {
    GetClassLongPtrW(hwnd, offset) as usize
}
#[cfg(target_pointer_width = "32")]
#[inline]
pub unsafe fn set_class_data(hwnd: HWND, offset: c_int, data: usize) -> usize {
    SetClassLongW(hwnd, offset, data as LONG) as usize
}
#[cfg(target_pointer_width = "64")]
#[inline]
pub unsafe fn set_class_data(hwnd: HWND, offset: c_int, data: usize) -> usize {
    SetClassLongPtrW(hwnd, offset, data as LONG_PTR) as usize
}
#[cfg(target_pointer_width = "32")]
#[inline]
pub unsafe fn get_window_data(hwnd: HWND, offset: c_int) -> usize {
    GetWindowLongW(hwnd, offset) as usize
}
#[cfg(target_pointer_width = "64")]
#[inline]
pub unsafe fn get_window_data(hwnd: HWND, offset: c_int) -> usize {
    GetWindowLongPtrW(hwnd, offset) as usize
}
#[cfg(target_pointer_width = "32")]
#[inline]
pub unsafe fn set_window_data(hwnd: HWND, offset: c_int, data: usize) -> usize {
    SetWindowLongW(hwnd, offset, data as LONG) as usize
}
#[cfg(target_pointer_width = "64")]
#[inline]
pub unsafe fn set_window_data(hwnd: HWND, offset: c_int, data: usize) -> usize {
    SetWindowLongPtrW(hwnd, offset, data as LONG_PTR) as usize
}

// Macros
#[inline]
pub const fn MAKELANGID(p: USHORT, s: USHORT) -> LANGID {
    (s << 10) | p
}

// ---------------------
// -- Dynamic Linking --
// ---------------------

#[inline]
unsafe fn dlopen(name: LPCSTR) -> HMODULE {
    LoadLibraryExA(name, 0 as HANDLE, 0)
}

dyn_link! {
    pub struct Win32DL(dlopen => HMODULE | GetProcAddress) {
        "Dwmapi.dll" {
            /// (Windows Vista+)
            /// Advanced querying of window attributes via the desktop window manager.
            fn DwmGetWindowAttribute(
                hWnd: HWND,
                dwAttribute: DWORD,
                pvAttribute: LPVOID,
                cbAttribute: DWORD,
            ) -> HRESULT;

            /// (Windows Vista+)
            /// Advanced setting of window attributes via the desktop window manager.
            fn DwmSetWindowAttribute(
                hWnd: HWND,
                dwAttribute: DWORD,
                pvAttribute: LPCVOID,
                cbAttribute: DWORD,
            ) -> HRESULT;
        },

        "Ntdll.dll" {
            /// (Win2000+)
            /// This is used in place of VerifyVersionInfoW, as it's not manifest dependent, and doesn't lie.
            fn RtlVerifyVersionInfo(
                VersionInfo: *mut OSVERSIONINFOEXW,
                TypeMask: DWORD,
                ConditionMask: c_ulonglong,
            ) -> NTSTATUS;
        },

        "Shcore.dll" {
            /// (Win8.1+)
            /// The intended way to query a monitor's DPI values since PMv1 and above.
            fn GetDpiForMonitor(
                hmonitor: HMONITOR,
                dpiType: u32,
                dpiX: *mut UINT,
                dpiY: *mut UINT,
            ) -> HRESULT;
        },

        "User32.dll" {
            // (Win10 1607+)
            // It's a version of AdjustWindowRectEx with DPI, but they added it 7 years late.
            // The DPI parameter accounts for scaled non-client areas, not to scale client areas.
            fn AdjustWindowRectExForDpi(
                lpRect: *mut RECT,
                dwStyle: DWORD,
                bMenu: BOOL,
                dwExStyle: DWORD,
                dpi: UINT,
            ) -> BOOL;

            /// (Win10 1603+)
            /// Enables automatic scaling of the non-client area as a hack for PMv1 DPI mode.
            fn EnableNonClientDpiScaling(hwnd: HWND) -> BOOL;

            /// (Vista+)
            /// First introduction of DPI awareness, this function enables System-Aware DPI.
            fn SetProcessDPIAware() -> BOOL;

            /// (Win8.1+)
            /// Allows you to set either System-Aware DPI mode, or Per-Monitor-Aware (v1).
            fn SetProcessDpiAwareness(value: PROCESS_DPI_AWARENESS) -> HRESULT;

            /// (Win10 1703+)
            /// Allows you to set either System-Aware DPI mode, or Per-Monitor-Aware (v1 *or* v2).
            fn SetProcessDpiAwarenessContext(value: DPI_AWARENESS_CONTEXT) -> BOOL;
        },
    }
}

impl Win32DL {
    pub unsafe fn link() -> Self {
        // Trying to load a nonexistent dynamic library or symbol sets the thread-global error.
        // Since this is intended and acceptable for missing functions, we restore the error state.

        let prev_error = GetLastError();
        let instance = Self::_link();
        SetLastError(prev_error);
        instance
    }
}

