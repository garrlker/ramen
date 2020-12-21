//! Helper types and methods for use mainly within `pub(crate)` context.

use std::sync::Arc;

/// Used to `const` initialize fields which don't necessarily need allocation.
#[derive(Clone)]
pub enum MaybeStatic<T: ?Sized> {
    Static(&'static T),
    Dynamic(Arc<T>),
}

impl<T: ?Sized> From<&'static T> for MaybeStatic<T> {
    #[inline]
    fn from(x: &'static T) -> Self {
        Self::Static(x)
    }
}

/// Rust allows you to have null characters in your strings, which is pretty neat,
/// but the native C APIs we have to interact with see it as a null *terminator*.
///
/// This function replaces all instances of null in a string with b' ' (space).
pub fn str_filter_nulls(x: &mut String) {
    // Safety: 0x00 and b' ' are one-byte sequences that can't mean anything else in UTF-8.
    unsafe {
        for byte in x.as_mut_vec() {
            if *byte == 0x00 {
                *byte = b' ';
            }
        }
    }
}
