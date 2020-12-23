//! Error types used within the crate.

use crate::platform::imp;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Internal(InternalError),
}

#[derive(Debug)]
#[repr(transparent)]
pub struct InternalError(imp::InternalError);

impl std::error::Error for Error {}
impl std::error::Error for InternalError {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TODO") // TODO: !
    }
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error {
    pub(crate) fn from_internal(err: imp::InternalError) -> Error {
        Error::Internal(InternalError(err))
    }
}
