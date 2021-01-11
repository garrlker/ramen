//! Helper types and methods for use mainly within `pub(crate)` context.

use std::{cell::UnsafeCell, ops::Deref, ptr, sync::{Arc, Once}};

/// Helps you create C-compatible string literals, like `c_string!("Hello!")` -> `b"Hello!\0"`.
macro_rules! c_string {
    ($s:expr) => {
        concat!($s, "\0").as_bytes()
    };
}

/// Macro to get around the limitation of not being able to write `#[doc = concat!("a", "b", ...)]`.
macro_rules! document {
    ($comment:expr, $($tt:tt)*) => {
        #[doc = $comment]
        $($tt)*
    };
}

/// Simple builder generator.
macro_rules! gen_builder {
    (
        $(#[$outer:meta])*
        $t_vis:vis struct $t_ident:ident {
            $(#[$inner:meta])*
            pub const fn new() -> Self {
                $($(#[$member_meta:meta])* $name:ident : $ty:ty = $def:expr),* $(,)?
            }
        }
    ) => {
        $(#[$outer])*
        $t_vis struct $t_ident {
            $($(#[$member_meta])* pub(crate) $name : $ty,)*
        }
        impl $t_ident {
            $(#[$inner])*
            pub const fn new() -> Self {
                Self {
                    $($(#[$member_meta])* $name : $def,)*
                }
            }
            $($(#[$member_meta])* #[inline]
            pub fn $name(&mut self, $name : $ty) -> &mut Self {
                self.$name = $name; self
            })*
        }
        impl Default for $t_ident {
            /// Default trait implementation, identical to construction via [`new`](Self::new).
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

/// Trait wrapper generator.
macro_rules! gen_wrapper {
    (
        $(#[$outer:meta])*
        $s_vis:vis struct $s_ident:ident($dyn_ident:ident : $dyn_repr_ident:path) {
            $(self: {$(
                $(#[$fn_outer:meta])*
                fn $fn_name:ident(&self $(, $name:ident : $ty:ty)* $(,)?) -> $ret:ty;
            )*})?

            $(mut self: {$(
                $(#[$fn_outer2:meta])*
                fn $fn_name2:ident(&mut self $(, $name2:ident : $ty2:ty)* $(,)?) -> $ret2:ty;
            )*})?
        }
    ) => {
        $(#[$outer])*
        $s_vis struct $s_ident(pub(crate) $dyn_repr_ident);
        pub(crate) trait $dyn_ident {
            $($($(#[$fn_outer] )* fn $fn_name (&self     $(, $name  : $ty )*)   -> $ret;  )*)*
            $($($(#[$fn_outer2])* fn $fn_name2(&mut self $(, $name2 : $ty2)*)   -> $ret2; )*)*
        }
        impl $s_ident {
            $($(
                $(#[$fn_outer])* #[inline]
                pub fn $fn_name(&self, $($name : $ty ,)*) -> $ret {
                    self.0.$fn_name($($name,)*)
                }
            )*)*
            $($(
                $(#[$fn_outer2])* #[inline]
                pub fn $fn_name2(&mut self, $($name2 : $ty2 ,)*) -> $ret2 {
                    self.0.$fn_name2($($name2,)*)
                }
            )*)*
        }
    };
}

/// Used to const initialize fields which don't necessarily need allocation (ex. str).
pub enum MaybeStatic<T: ?Sized + 'static> {
    Static(&'static T),
    Dynamic(Arc<T>),
}

impl<T: ?Sized + 'static> From<&'static T> for MaybeStatic<T> {
    #[inline]
    fn from(x: &'static T) -> Self {
        Self::Static(x)
    }
}

impl<T: ?Sized + 'static> Clone for MaybeStatic<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Static(x) => Self::Static(x),
            Self::Dynamic(x) => Self::Dynamic(x.clone()),
        }
    }
}

impl<T: ?Sized + 'static> AsRef<T> for MaybeStatic<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &*self
    }
}

impl<T: ?Sized + 'static> Deref for MaybeStatic<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Static(x) => x,
            Self::Dynamic(x) => x.as_ref(),
        }
    }
}

/// Minimal lazily initialized type, similar to the one in `once_cell`.
///
/// Thread safe initialization, immutable-only access.
pub(crate) struct LazyCell<T, F = fn() -> T> {
    // Invariant: Written to at most once on first access.
    init: UnsafeCell<Option<F>>,
    ptr: UnsafeCell<*const T>,

    // Synchronization primitive for initializing `init` and `ptr`.
    once: Once,
}

unsafe impl<T, F> Send for LazyCell<T, F> where T: Send {}
unsafe impl<T, F> Sync for LazyCell<T, F> where T: Sync {}

impl<T, F> LazyCell<T, F> {
    pub const fn new(init: F) -> Self {
        Self {
            init: UnsafeCell::new(Some(init)),
            ptr: UnsafeCell::new(ptr::null()),
            once: Once::new(),
        }
    }
}

impl<T, F: FnOnce() -> T> LazyCell<T, F> {
    pub fn get(&self) -> &T {
        self.once.call_once(|| unsafe {
            if let Some(f) = (&mut *self.init.get()).take() {
                let pointer = Box::into_raw(Box::new(f()));
                ptr::write(self.ptr.get(), pointer);
            } else {
                // If this panic fires, `std::sync::Once` is broken,
                // as `self.{init, ptr}` should only be written to once.
                unreachable!()
            }
        });

        // Safety: The above call to `call_once` initialized the pointer.
        unsafe {
            &**self.ptr.get()
        }
    }
}

impl<T, F: FnOnce() -> T> Deref for LazyCell<T, F> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

/// Rust allows you to have null characters in your strings, which is pretty neat,
/// but the native C APIs we have to interact with see it as a null *terminator*.
///
/// This function replaces all instances of null in a string with b' ' (space).
pub(crate) fn str_filter_nulls(x: &mut String) {
    // Safety: 0x00 and b' ' are one-byte sequences that can't mean anything else in UTF-8.
    unsafe {
        for byte in x.as_mut_vec() {
            if *byte == 0x00 {
                *byte = b' ';
            }
        }
    }
}

/// Wrapper for working with both `std` and `parking_lot`.
/// None of these functions should panic as they're used in FFI.
#[cfg(not(feature = "parking-lot"))]
pub(crate) mod sync {
    pub use std::sync::{Condvar, Mutex, MutexGuard};
    use std::{ptr, process};

    #[inline]
    pub fn condvar_notify1(cvar: &Condvar) {
        cvar.notify_one();
    }

    #[inline]
    pub fn condvar_wait<T>(cvar: &Condvar, guard: &mut MutexGuard<T>) {
        // The signature in `std` is quite terrible and CONSUMES the guard
        // We "move it out" for the duration of the wait
        unsafe {
            let guard_copy = ptr::read(guard);
            let result = cvar.wait(guard_copy).unwrap_or_else(|_poi| {
                eprintln!("Condvar's mutex was poisoned! This is a bug.");
                process::exit(1);
            });
            ptr::write(guard, result);
        }
    }

    pub fn mutex_lock<T>(mtx: &Mutex<T>) -> MutexGuard<T> {
        mtx.lock().unwrap_or_else(|_poi| {
            eprintln!("Mutex was poisoned! This is a bug.");
            process::exit(1);
        })
    }
}
