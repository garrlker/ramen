//! Helper types and methods for use mainly within `pub(crate)` context.

use std::sync::Arc;

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
