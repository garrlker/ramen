//! [`Window`] and related types.

use crate::{
    helpers::{self, MaybeStatic},
    monitor::{/*Point,*/ Size},
};
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
            /// Sets the inner size of the window.
            ///
            /// If the size is given in *logical* numbers,
            /// DPI scaling is applied and will update dynamically.\
            /// If the size is given in *physical* numbers,
            /// no DPI scaling is done and it's used as an exact pixel value.
            ///
            /// Defaults to `Size::Logical(800.0, 608.0)`.
            inner_size: Size = Size::Logical(800.0, 608.0),

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

impl WindowBuilder {
    /// Sets the initial window title.
    ///
    /// Defaults to an empty string (blank).
    pub fn title(&mut self, title: impl Into<String>) -> &mut Self {
        let mut title = title.into();
        helpers::str_filter_nulls(&mut title);
        self.__title = MaybeStatic::Dynamic(title.into());
        self
    }
}
