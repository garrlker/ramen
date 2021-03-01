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

macro_rules! dyn_link {
    (
        $(#[$outer:meta])*
        $s_vis:vis struct $s_ident:ident($dlopen:expr => $dlopen_ty:ty | $dlsym:expr) {
            $($($module_name:literal)|+ {
                $(
                    $(#[$fn_outer:meta])*
                    fn $sym_fn:ident($($name:ident : $ty:ty),*$(,)?) -> $ret:ty;
                )*
            }),* $(,)?
        }
    ) => {
        $(#[$outer])*
        pub struct $s_ident {
            $($(
                $(#[$fn_outer])*
                $sym_fn : ::std::option::Option<
                    unsafe extern "system" fn($($name : $ty ,)*) -> $ret
                > ,
            )*)*
        }

        impl $s_ident {
            #[allow(unused_doc_comments)]
            unsafe fn _link() -> Self {
                let mut inst = ::std::mem::MaybeUninit::<Self>::uninit();
                let mut inst_ref = &mut *(inst.as_mut_ptr());
                $(
                    let mut handle = 0 as $dlopen_ty;
                    for name in &[$(c_string!($module_name) ,)*] {
                        handle = $dlopen(name.as_ptr().cast());
                        if handle != 0 as $dlopen_ty {
                            break
                        }
                    }
                    if handle != 0 as $dlopen_ty {
                        $(
                            $(#[$fn_outer])*
                            {
                                inst_ref.$sym_fn =
                                    ::std::mem::transmute::<_,
                                        Option<unsafe extern "system" fn($($name : $ty ,)*) -> $ret>>
                                    ($dlsym(handle, c_string!(stringify!($sym_fn)).as_ptr().cast()));
                            }
                        )*
                    } else {
                        $(
                            $(#[$fn_outer])*
                            {
                                inst_ref.$sym_fn = None;
                            }
                        )*
                    }
                )*
                inst.assume_init()
            }
            $($(
                $(#[$fn_outer])*
                $s_vis unsafe fn $sym_fn(&self, $($name : $ty ,)*) -> ::std::option::Option<$ret> {
                    self.$sym_fn.map(|f| f($($name ,)*))
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

/// Wrapper for working with both `std` and `parking_lot`.
/// None of these functions should panic when used correctly as they're used in FFI.
#[cfg(not(feature = "parking-lot"))]
pub(crate) mod sync {
    pub use std::sync::{Condvar, Mutex, MutexGuard};
    use std::ptr;

    #[inline]
    pub fn condvar_notify1(cvar: &Condvar) {
        cvar.notify_one();
    }

    pub fn condvar_wait<T>(cvar: &Condvar, guard: &mut MutexGuard<T>) {
        // The signature in `std` is quite terrible and CONSUMES the guard
        // We "move it out" for the duration of the wait
        unsafe {
            let guard_copy = ptr::read(guard);
            let result = cvar.wait(guard_copy).expect("cvar mutex poisoned (this is a bug)");
            ptr::write(guard, result);
        }
    }

    pub fn mutex_lock<T>(mtx: &Mutex<T>) -> MutexGuard<T> {
        mtx.lock().expect("mutex poisoned (this is a bug)")
    }
}
#[cfg(feature = "parking-lot")]
pub(crate) mod sync {
    pub use parking_lot::{Condvar, Mutex, MutexGuard};

    #[inline]
    pub fn condvar_notify1(cvar: &Condvar) {
        let _ = cvar.notify_one();
    }

    #[inline]
    pub fn condvar_wait<T>(cvar: &Condvar, guard: &mut MutexGuard<T>) {
        cvar.wait(guard);
    }

    #[inline]
    pub fn mutex_lock<T>(mtx: &Mutex<T>) -> MutexGuard<T> {
        mtx.lock()
    }
}
