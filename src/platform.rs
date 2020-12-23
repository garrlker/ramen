//! Platform-specific implementations and API extensions.

// Developer Notes
// ===============
//
// To implement your own platforms, here's what you need to export:
// - The type `InternalError` which implements `std::error::Error` and is `Send + Sync`
//   This is for OS error codes - you should construct these with `Error::from_internal`
//
// - The type `WindowRepr` that is callable as `WindowImpl` and is `Send + Sync`
//
// - The function `make_window` of type `fn(&WindowBuilder) -> Result<WindowRepr, Error>`

#[cfg(windows)]
pub mod win32;
#[cfg(windows)]
pub(crate) use win32 as imp;
