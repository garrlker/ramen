//! [`Window`] and related types.

use crate::{
    error::Error,
    event::Event,
    helpers::MaybeStatic,
    monitor::{/*Point,*/ Size},
    platform::imp,
};
use std::borrow::Cow;

/// Represents a window, of course.
///
/// To create a window, use a [`builder`](Window::builder).
pub struct Window {
    pub(crate) inner: imp::WindowRepr,
}

pub(crate) trait WindowImpl {
    fn events(&self) -> &[Event];
    fn execute(&self, f: &mut dyn FnMut());
    fn set_title(&self, title: &str);
    fn set_title_async(&self, title: &str);
    fn set_visible(&self, visible: bool);
    fn set_visible_async(&self, visible: bool);
    fn swap_events(&mut self);
}

impl Window {
    /// Constructs a [`WindowBuilder`] for instantiating windows.
    pub const fn builder() -> WindowBuilder {
        WindowBuilder::new()
    }
}

impl Window {
    /// Gets the current event buffer. Events are in the order they were received.
    ///
    /// To acquire new events, call [`swap_events`](Self::swap_events);
    /// repeated calls to this function will not advance the buffer.
    ///
    /// ```rust
    /// loop {
    ///     for event in window.events() {
    ///         // process events!
    ///     }
    ///
    ///     // acquire new events
    ///     window.swap_events();
    /// }
    /// ```
    #[inline]
    pub fn events(&self) -> &[Event] {
        self.inner.events()
    }

    /// Executes an arbitrary function in the window thread, blocking until it returns.
    ///
    /// This is **not** how functions such as [`set_visible`](Self::set_visible) are implemented,
    /// but rather a way to guarantee that native low-level calls are executed in the remote thread if necessary,
    /// especially on platforms like Win32 that make excessive use of thread globals.
    ///
    /// ```rust
    /// window.execute(|window| {
    ///     println!("Hello from the window thread!");
    ///     window.set_title("hi"); // window accessible
    /// });
    /// ```
    #[inline]
    pub fn execute<F>(&self, mut f: F)
    where
        F: FnMut(&Self) + Send,
    {
        self.inner.execute(&mut move || f(self));
    }

    #[inline]
    pub fn set_title(&self, title: &str) {
        self.inner.set_title(title);
    }

    #[inline]
    pub fn set_title_async(&self, title: &str) {
        self.inner.set_title_async(title);
    }

    /// Sets whether the window is visible or hidden.
    #[inline]
    pub fn set_visible(&self, visible: bool) {
        self.inner.set_visible(visible);
    }

    /// Non-blocking variant of [`set_visible`](Self::set_visible).
    #[inline]
    pub fn set_visible_async(&self, visible: bool) {
        self.inner.set_visible_async(visible);
    }

    /// Acquires events that have occured since the last call to [`swap_events`](Self::swap_events), if ever.
    ///
    /// The buffer containing those events is accessible via
    /// [`events`](Self::events) - see that function for more information.
    #[inline]
    pub fn swap_events(&mut self) {
        self.inner.swap_events();
    }
}

/// Builder for creating [`Window`] instances.
///
/// To create a builder, use [`Window::builder`].
#[derive(Clone)]
pub struct WindowBuilder {
    pub(crate) class_name: MaybeStatic<str>,
    pub(crate) inner_size: Size,
    pub(crate) style: WindowStyle,
    pub(crate) title: MaybeStatic<str>,
}

impl WindowBuilder {
    pub(crate) const fn new() -> Self {
        Self {
            class_name: MaybeStatic::Static("ramen_window_class"),
            inner_size: Size::Logical(800.0, 608.0),
            style: WindowStyle {
                borderless: false,
                controls: Some(WindowControls::no_maximize()),
                resizable: true,
                visible: true,
            },
            title: MaybeStatic::Static("a nice window"),
        }
    }

    /// what do you think
    pub fn build(&self) -> Result<Window, Error> {
        imp::make_window(self).map(|inner| Window { inner })
    }
}

impl WindowBuilder {
    /// Sets whether the window is initially without a border.
    ///
    /// Defaults to `false`.
    pub fn borderless(&mut self, borderless: bool) -> &mut Self {
        self.style.borderless = borderless;
        self
    }

    /// Sets the platform-specific window class name.
    ///
    /// - Win32: `lpszClassName` in
    /// [`WNDCLASSEXW`](https://docs.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-wndclassexw)
    /// - TODO: Other platforms!
    ///
    /// Defaults to `"ramen_window_class"`.
    pub fn class_name<T>(&mut self, class_name: T) -> &mut Self
    where
        T: Into<Cow<'static, str>>,
    {
        self.class_name = match class_name.into() {
            Cow::Borrowed(x) => x.into(),
            Cow::Owned(x) => MaybeStatic::Dynamic(x.into()),
        };
        self
    }

    /// Sets the initial window controls. `None` indicates that no controls are desired.
    ///
    /// Defaults to [`WindowControls::no_maximize`].
    pub fn controls(&mut self, controls: Option<WindowControls>) -> &mut Self {
        self.style.controls = controls;
        self
    }

    /// Sets the initial inner size of the window.
    ///
    /// Defaults to `Size::Logical(800.0, 608.0)`.
    // TODO: explain "if physical no scaling" etc
    pub fn inner_size(&mut self, inner_size: Size) -> &mut Self {
        self.inner_size = inner_size;
        self
    }

    /// Sets whether the window is initially resizable.
    ///
    /// Defaults to `true`.
    pub fn resizable(&mut self, resizable: bool) -> &mut Self {
        self.style.resizable = resizable;
        self
    }

    /// Sets the initial window title.
    ///
    /// Defaults to `"a nice window"`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ramen::window::Window;
    ///
    /// let mut builder = Window::builder()
    ///     .title("Cool Window") // static reference, or
    ///     .title(String::from("Cool Window")); // owned data
    /// ```
    pub fn title<T>(&mut self, title: T) -> &mut Self
    where
        T: Into<Cow<'static, str>>,
    {
        self.title = match title.into() {
            Cow::Borrowed(x) => x.into(),
            Cow::Owned(x) => MaybeStatic::Dynamic(x.into()),
        };
        self
    }

    /// Sets whether the window is initially visible.
    ///
    /// Defaults to `true`.
    pub fn visible(&mut self, visible: bool) -> &mut Self {
        self.style.visible = visible;
        self
    }
}

impl Default for WindowBuilder {
    /// Identical to construction via [`Window::builder`].
    fn default() -> Self {
        Self::new()
    }
}

/// Represents the availability of the minimize, maximize, and close buttons on a [`Window`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowControls {
    pub minimize: bool,
    pub maximize: bool,
    pub close: bool,
}

impl WindowControls {
    /// Creates window controls from the provided values.
    pub const fn new(minimize: bool, maximize: bool, close: bool) -> Self {
        Self {
            minimize,
            maximize,
            close,
        }
    }

    /// Creates window controls with all 3 buttons enabled.
    pub const fn enabled() -> Self {
        Self::new(true, true, true)
    }

    /// Creates window controls with the minimize & close buttons available.
    pub const fn no_maximize() -> Self {
        Self::new(true, false, true)
    }
}

impl Default for WindowControls {
    /// Default trait implementation, same as [`WindowControls::new`].
    fn default() -> Self {
        Self::enabled()
    }
}

#[derive(Default, Clone)]
pub(crate) struct WindowStyle {
    pub borderless: bool,
    pub resizable: bool,
    pub visible: bool,
    pub controls: Option<WindowControls>,
}
