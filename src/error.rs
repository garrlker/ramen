//! Error types used within the crate.

use std::fmt;

#[derive(Debug)]
pub struct Error {

}

impl std::error::Error for Error {}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TODO") // TODO
    }
}
