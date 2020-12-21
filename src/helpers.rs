//! Helper types and methods for use mainly within `pub(crate)` context.

use std::sync::Arc;

/// Helps you create C-compatible string literals, like `c_string!("Hello!")` -> `b"Hello!\0"`.
macro_rules! c_string {
    ($s:expr) => {
        concat!($s, "\0").as_bytes()
    };
}

/// Macro to get around the limitation of not being able to write `#[doc = concat!("a", "b", ...)]`.
macro_rules! document {
    ($comment:expr, $($tt:tt)*) => {
        #[doc = $comment]
        $($tt)*
    };
}

/// Simple builder generator.
macro_rules! gen_builder {
    (
        $(#[$outer:meta])*
        $t_vis:vis struct $t_ident:ident {
            $(#[$inner:meta])*
            pub const fn new() -> Self {
                $($(#[$member_meta:meta])* $name:ident : $ty:ty = $def:expr),* $(,)?
            }
        }
    ) => {
        $(#[$outer])*
        $t_vis struct $t_ident {
            $($(#[$member_meta])* pub(crate) $name : $ty,)*
        }
        impl $t_ident {
            $(#[$inner])*
            pub const fn new() -> Self {
                Self {
                    $($(#[$member_meta])* $name : $def,)*
                }
            }
            $($(#[$member_meta])* #[inline]
            pub fn $name(&mut self, $name : $ty) -> &mut Self {
                self.$name = $name; self
            })*
        }
        impl Default for $t_ident {
            /// Default trait implementation, identical to construction via [`new`](Self::new).
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

/// Used to const initialize fields which don't necessarily need allocation (ex. str).
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
pub(crate) fn str_filter_nulls(x: &mut String) {
    // Safety: 0x00 and b' ' are one-byte sequences that can't mean anything else in UTF-8.
    unsafe {
        for byte in x.as_mut_vec() {
            if *byte == 0x00 {
                *byte = b' ';
            }
        }
    }
}
