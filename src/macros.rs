//! Utility macros intended for use within `pub(crate)` context.

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
