//! the event api

#[derive(Copy, Clone, Debug)]
pub enum Event {
    /// The window has requested to close.
    /// For more information on why, see the associated [`CloseReason`].
    CloseRequest(CloseReason),

    /// The window focus has been updated: `true` if focused, `false` if unfocused.
    Focus(bool),
}

/// Details why a `CloseRequest` [`Event`] was received.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CloseReason {
    /// The user has pressed a system control to close the window.
    ///
    /// This is usually the "X button" or the red stop light on the control menu.
    SystemMenu,

    /// The user has pressed the system keyboard shortcut to close the active window.
    ///
    /// This is usually something like Alt+F4, Command+W, or Control+W.
    KeyboardShortcut,

    /// The reason for the close request is unknown.
    ///
    /// Likely reasons include external programs sending the signal.
    Unknown,
}
