//! A cross-platform windowing crate, built for performance.
//!
//! # Features
//! - `cursor-lock`: Adds the ability to constrain the cursor
//! to the inner bounds of the window or lock it to the center.
//! - `parking-lot`: Replaces the `std` for synchronization primitives
//! with the [`parking_lot`](https://crates.io/crates/parking_lot) crate.
//! Highly recommended, at least for release builds.

#![cfg_attr(feature = "nightly-docs", feature(doc_cfg))]
#![deny(unused_results)]

#[doc(hidden)]
#[macro_use]
pub mod helpers;

pub mod error;
pub mod event;
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
