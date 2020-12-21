//! Windows API specific implementations and API extensions.

use crate::window::WindowImpl;

pub(crate) struct Window {
    // ...
}

pub(crate) type WindowRepr = Window;

impl WindowImpl for Window {
    fn set_visible(&self, visible: bool) {
        let _ = visible;
    }

    fn swap_events(&mut self) {
        // ...
    }
}
