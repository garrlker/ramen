//! A neat windowing library.

#[doc(hidden)]
#[macro_use]
pub mod helpers;

pub mod error;
pub mod monitor;
pub mod platform;
pub mod window;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
