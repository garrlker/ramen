//! Platform-specific implementations and API extensions.

// Developer Notes
// ===============
//
// To implement your own platforms, here's what you need to export:
// - The type `WindowRepr` that derefs to or implements `WindowImpl` (is callable as such).
// - The function `make_window` of type `fn(&WindowBuilder) -> Result<WindowRepr, Error>`

#[cfg(windows)]
pub mod win32;
#[cfg(windows)]
pub(crate) use win32 as imp;
