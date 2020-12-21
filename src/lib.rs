//! A neat windowing library.

#[macro_use]
pub(crate) mod macros;

pub mod window;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
