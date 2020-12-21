//! [`Window`] and related types.

use crate::helpers::MaybeStatic;
use std::sync::Arc;

/// The whole point of the crate.
///
/// To instantiate windows, use a [`builder`](Window::builder).
pub struct Window {
    // ...
}

gen_builder! {
    /// Builder for creating [`Window`] instances.
    #[derive(Clone)]
    pub struct WindowBuilder {
        /// Constructs a new `WindowBuilder`.
        ///
        /// Prefer [`Window::builder`] for instantiation to avoid needing additional imports.
        pub const fn new() -> Self {
            /// Sets whether the window is initially visible.
            ///
            /// Defaults to `true`.
            visible: bool = true,

            // These members below have a wrapper for additional necessary checks.
            // You may directly use `builder.__title(x)`, etc. to avoid checks & allocation.
            #[doc(hidden)]
            __class_name: MaybeStatic<str> = MaybeStatic::Static("ramen_window"),
            #[doc(hidden)]
            __title: MaybeStatic<str> = MaybeStatic::Static(""),
        }
    }
}

impl Window {
    /// Constructs a [`WindowBuilder`] for instantiating windows.
    pub const fn builder() -> WindowBuilder {
        WindowBuilder::new()
    }
}
