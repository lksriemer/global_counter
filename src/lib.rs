//! This crate implements global, thread-safe counters.
//!
//! Concerning performance, the general ranking is, from fastest to slowest:
//!
//! * [FlushingCounter](primitive/struct.FlushingCounter.html)
//! * [ApproxCounter](primitive/struct.ApproxCounter.html)
//! * [Exact primitive atomic counter](primitive/index.html)
//! * [Counter](generic/struct.Counter.html)
//!
//! Don't forget to make your own benchmarks.

extern crate lazy_static;

// We need to pub use lazy_static, as global_(default_)counter! is expanded to a lazy_static! call.
// Absolute paths wont help here.
// TODO: Think of a way to only pub reexport the lazy_static! macro.
#[doc(hidden)]
pub use lazy_static::*;

/// This module contains a generic, thread-safe counter and the accompanying `Inc` trait.
pub mod generic;

/// This module contains exact atomic counters for primitive integer types.
pub mod primitive;

// Hack for macro export.
// In foreign crates, `global_counter::generic::Counter` will be the name of our counter,
// but in this crate (for testing), we need to artificially introduce this path.
// TODO: Think of a better way to do this.
#[doc(hidden)]
pub mod global_counter {
    pub mod generic {
        pub type Counter<T> = crate::generic::Counter<T>;
    }
}
