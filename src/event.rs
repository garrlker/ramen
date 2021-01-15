//! the event api

#[derive(Copy, Clone)]
pub enum Event {
    CloseRequest(CloseReason),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CloseReason {
    /// The user has pressed a system control to close the window.
    ///
    /// This is usually the "X button" or the red stop light on the control menu.
    SystemMenu,

    /// The user has pressed the native keyboard shortcut to close the active window.
    ///
    /// This is usually Alt+F4, Command+W, or Control+W.
    KeyboardShortcut,

    /// The reason for the close request is unknown.
    ///
    /// Likely reasons include external programs sending the signal.
    Unknown,
}
