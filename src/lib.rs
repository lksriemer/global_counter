//! This crate implements global, thread-safe counters.
//!
//! Concerning performance, the general ranking is, from fastest to slowest:
//!
//! * [Flushing primitive counters](primitive/fast/index.html)
//! * [Approximate primitive counters](primitive/fast/index.html)
//! * [Exact primitive atomic counters](primitive/exact/index.html)
//! * [Generic counter](generic/struct.Counter.html)
//!
//! Don't forget to make your own benchmarks, as those are very specific to the computing system in general and, in this case, to the OS in specific.

/// This module contains a global, generic counter and the accompanying `Inc` trait.
pub mod generic;

/// This module contains global counters for primitive integer types.
pub mod primitive;

// Hack for macro export.
#[doc(hidden)]
pub mod global_counter_macro_dependencies {
    pub type Lazy<T> = once_cell::sync::Lazy<T>;
}


