//! This is a minimal library implementing global, thread-safe counters.

extern crate lazy_static;

// We need to pub use lazy_static, as global_(default_)counter! is expanded to a lazy_static! call.
// Absolute paths wont help here.
// TODO: Think of a way to only pub reexport the lazy_static! macro.
#[doc(hidden)]
pub use lazy_static::*;

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

/// This module contains atomic counters for primitive integer types.
pub mod primitive {
    use std::sync::atomic::{
        AtomicI16, AtomicI32, AtomicI64, AtomicI8, AtomicIsize, AtomicU16, AtomicU32, AtomicU64,
        AtomicU8, AtomicUsize, Ordering,
    };

    macro_rules! primitive_counter {
            ($( $primitive:ident $atomic:ident $counter:ident ), *) => {
                $(
                    /// A primitive counter, implemented using atomics from `std::sync::atomic`.
                    ///
                    /// This counter makes all the same guarantees a generic counter does.
                    /// Especially, calling `inc` N times from different threads will always result in the counter effectively being incremented by N.
                    ///
                    /// Regarding atomic ordering, `Ordering::SeqCst` is currently used whenever possible.
                    /// This unstable detail should never be relied on for soundness.
                    ///
                    /// Please note that Atomics may, depending on your compilation target, [not be implemented using atomic instructions](https://llvm.org/docs/Atomics.html),
                    /// meaning lock-freendom can in the general case not be guaranteed.
                    ///
                    /// This counter should in general be superior in performance, compared to the equivalent generic counter.
                    #[derive(Debug, Default)]
                    pub struct $counter($atomic);

                    impl $counter{
                        /// Creates a new primitive counter. Can be used in const contexts.
                        #[allow(dead_code)]
                        #[inline]
                        pub const fn new(val : $primitive) -> $counter{
                            $counter($atomic::new(val))
                        }

                        /// Gets the current value of the counter.
                        #[allow(dead_code)]
                        #[inline]
                        pub fn get(&self) -> $primitive{
                            self.0.load(Ordering::SeqCst)
                        }

                        /// Sets the counter to a new value.
                        #[allow(dead_code)]
                        #[inline]
                        pub fn set(&self, val : $primitive){
                            self.0.store(val, Ordering::SeqCst);
                        }

                        /// Increments the counter by one, returning the previous value.
                        #[allow(dead_code)]
                        #[inline]
                        pub fn inc(&self) -> $primitive{
                            self.0.fetch_add(1, Ordering::SeqCst)
                        }

                        /// Resets the counter to zero.
                        #[allow(dead_code)]
                        #[inline]
                        pub fn reset(&self){
                            self.0.store(0, Ordering::SeqCst);
                        }
                    }
                )*
            };
        }

    primitive_counter![u8 AtomicU8 CounterU8, u16 AtomicU16 CounterU16, u32 AtomicU32 CounterU32, u64 AtomicU64 CounterU64, usize AtomicUsize CounterUsize, i8 AtomicI8 CounterI8, i16 AtomicI16 CounterI16, i32 AtomicI32 CounterI32, i64 AtomicI64 CounterI64, isize AtomicIsize CounterIsize];
}

/// This module contains a generic, thread-safe counter and the accompanying `Inc` trait.
pub mod generic {
    use parking_lot::Mutex;

    /// This trait promises incrementing behaviour.
    /// Implemented for standard integer types.
    /// The current value is mutated, becoming the new, incremented value.
    pub trait Inc {
        fn inc(&mut self);
    }

    macro_rules! imp {
        ($( $t:ty ) *) => {
            $(
                impl Inc for $t{
                    #[inline]
                    fn inc(&mut self){
                        *self += 1;
                    }
                }
            )*
        };
    }

    imp![u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize];

    /// A generic counter.
    ///
    /// This counter is `Send + Sync` regardless of its contents, meaning it is always globally available from all threads, concurrently.
    ///
    /// Implement `Inc` by supplying an impl for incrementing your type. This implementation does not need to be thread-safe.
    ///
    /// Implementation-wise, this is basically a [Mutex from parking_lot](https://docs.rs/lock_api/*/lock_api/struct.Mutex.html).
    #[derive(Debug, Default)]
    pub struct Counter<T: Inc>(Mutex<T>);

    /// Creates a new generic, global counter, starting from the given value.
    ///
    /// This macro is exported at the crates top-level.
    ///
    /// # Example
    /// ```
    /// # #[macro_use] use crate::global_counter::*;
    /// type CountedType = u32;
    /// fn main(){
    ///     const start_value : u32 = 0;
    ///     global_counter!(COUNTER_NAME, CountedType, start_value);
    ///     assert_eq!(COUNTER_NAME.get_cloned(), 0);
    ///     COUNTER_NAME.inc();
    ///     assert_eq!(COUNTER_NAME.get_cloned(), 1);
    /// }
    /// ```
    #[macro_export]
    macro_rules! global_counter {
        ($name:ident, $type:ident, $value:expr) => {
            lazy_static! {
                static ref $name: global_counter::generic::Counter<$type> =
                    global_counter::generic::Counter::new($value);
            }
        };
    }

    /// Creates a new generic, global counter, starting from its (inherited) default value.
    ///
    /// This macro will fail compilation if the given type is not `Default`.
    ///
    /// This macro is exported at the crates top-level.
    ///
    /// # Example
    /// ```
    /// # #[macro_use] use crate::global_counter::*;
    /// type CountedType = u32;
    /// fn main(){
    ///     global_default_counter!(COUNTER_NAME, CountedType);
    ///     assert_eq!(COUNTER_NAME.get_cloned(), 0);
    ///     COUNTER_NAME.inc();
    ///     assert_eq!(COUNTER_NAME.get_cloned(), 1);
    /// }
    /// ```
    #[macro_export]
    macro_rules! global_default_counter {
        ($name:ident, $type:ty) => {
            lazy_static! {
                static ref $name: global_counter::generic::Counter<$type> =
                    global_counter::generic::Counter::default();
            }
        };
    }

    impl<T: Inc> Counter<T> {
        /// Creates a new generic counter
        ///
        /// This function is not const yet. As soon as [Mutex::new()](https://docs.rs/lock_api/*/lock_api/struct.Mutex.html#method.new) is stable as `const fn`, this will be as well.
        /// Then, the exported macros will no longer be needed.
        #[allow(dead_code)]
        #[inline]
        pub fn new(val: T) -> Counter<T> {
            Counter(Mutex::new(val))
        }

        /// Returns (basically) an immutable borrow of the underlying value.
        /// Best make sure this borrow goes out of scope before any other methods of the counter are being called.
        ///
        /// If `T` is not `Clone`, this is the only way to access the current value of the counter.
        ///
        /// **Warning**: Attempting to access the counter from the thread holding this borrow **will** result in a deadlock.
        /// As long as this borrow is alive, no accesses to the counter from any thread are possible.
        ///
        /// # Good Example - Borrow goes out of scope
        /// ```
        /// # #[macro_use] use crate::global_counter::*;
        /// fn main(){
        ///     global_default_counter!(COUNTER, u8);
        ///     assert_eq!(0, *COUNTER.get_borrowed());
        ///
        ///     // The borrow is already out of scope, we can call inc safely.
        ///     COUNTER.inc();
        ///
        ///     assert_eq!(1, *COUNTER.get_borrowed());}
        /// ```
        ///
        /// # Good Example - At most one concurrent access per thread
        /// ```
        /// # #[macro_use] use crate::global_counter::*;
        /// fn main(){
        ///     global_default_counter!(COUNTER, u8);
        ///     assert_eq!(0, *COUNTER.get_borrowed());
        ///     
        ///     // Using this code, there is no danger of data races, race coditions whatsoever.
        ///     // As at each point in time, each thread either has a borrow of the counters value alive,
        ///     // or is accessing the counter using its api, never both at the same time.
        ///     let t1 = std::thread::spawn(move || {
        ///         COUNTER.inc();
        ///         let value_borrowed = COUNTER.get_borrowed();
        ///         assert!(1 <= *value_borrowed, *value_borrowed <= 3);
        ///     });
        ///     let t2 = std::thread::spawn(move || {
        ///         COUNTER.inc();
        ///         let value_borrowed = COUNTER.get_borrowed();
        ///         assert!(1 <= *value_borrowed, *value_borrowed <= 3);
        ///     });
        ///     let t3 = std::thread::spawn(move || {
        ///         COUNTER.inc();
        ///         let value_borrowed = COUNTER.get_borrowed();
        ///         assert!(1 <= *value_borrowed, *value_borrowed <= 3);
        ///     });
        ///
        ///     t1.join().unwrap();
        ///     t2.join().unwrap();
        ///     t3.join().unwrap();
        ///     
        ///     assert_eq!(3, *COUNTER.get_borrowed());}
        /// ```
        ///
        /// # Bad Example - Deadlock
        /// ```no_run
        /// # #[macro_use] use crate::global_counter::*;
        /// // We spawn a new thread. This thread will try lockig the counter twice, causing a deadlock.
        /// std::thread::spawn(move || {
        ///
        ///     // We could also use get_cloned with this counter, circumventing all these troubles.
        ///     global_default_counter!(COUNTER, u32);
        ///     
        ///     // The borrow is now alive, and this thread now holds a lock onto the counter.
        ///     let counter_value_borrowed = COUNTER.get_borrowed();
        ///     assert_eq!(0, *counter_value_borrowed);
        ///
        ///     // Now we try to lock the counter again, but we already hold a lock in the current thread! Deadlock!
        ///     COUNTER.inc();
        ///     
        ///     // Here we use `counter_value_borrowed` again, ensuring it can't be dropped "fortunately".
        ///     // This line will never actually be reached.
        ///     assert_eq!(0, *counter_value_borrowed);
        /// });
        /// ```
        #[allow(dead_code)]
        #[inline]
        pub fn get_borrowed(&self) -> impl std::ops::Deref<Target = T> + '_ {
            self.0.lock()
        }

        /// Sets the counter to be the given value.
        #[allow(dead_code)]
        #[inline]
        pub fn set(&self, val: T) {
            *self.0.lock() = val;
        }

        /// Increments the counter, delegating the specific implementation to the [Inc](trait.Inc.html) trait.
        #[allow(dead_code)]
        #[inline]
        pub fn inc(&self) {
            (*self.0.lock()).inc();
        }
    }

    impl<T: Inc + Clone> Counter<T> {
        /// This avoid the troubles of [get_borrowed](struct.Counter.html#method.get_borrowed) by cloning the current value.
        ///
        /// Creating a deadlock using this API should be impossible.
        /// The downside of this approach is the cost of a forced clone which may, depending on your use case, not be affordable.
        #[allow(dead_code)]
        #[inline]
        pub fn get_cloned(&self) -> T {
            (*self.0.lock()).clone()
        }

        /// Increments the counter, returning the previous value, cloned.
        #[allow(dead_code)]
        #[inline]
        pub fn inc_cloning(&self) -> T {
            let prev = self.get_cloned();
            self.inc();
            prev
        }
    }

    impl<T: Inc + Default> Counter<T> {
        /// Resets the counter to its default value.
        #[allow(dead_code)]
        #[inline]
        pub fn reset(&self) {
            self.set(T::default());
        }
    }
}

#[cfg(test)]
mod tests {

    #[cfg(test)]
    mod generic {

        #![allow(unused_attributes)]
        #[macro_use]
        use crate::*;

        // TODO: Add tests for get_borrowed.

        #[test]
        fn count_to_five_single_threaded() {
            global_default_counter!(COUNTER, u32);
            assert_eq!(COUNTER.get_cloned(), 0);
            COUNTER.inc();
            assert_eq!(COUNTER.get_cloned(), 1);
            COUNTER.inc();
            assert_eq!(COUNTER.get_cloned(), 2);
            COUNTER.inc();
            assert_eq!(COUNTER.get_cloned(), 3);
            COUNTER.inc();
            assert_eq!(COUNTER.get_cloned(), 4);
            COUNTER.inc();
            assert_eq!(COUNTER.get_cloned(), 5);
        }

        // TODO: Clean up this mess

        #[derive(Clone, Default, PartialEq, Eq, Debug)]
        struct Baz<T> {
            i: i32,
            u: i32,
            _marker: std::marker::PhantomData<T>,
        }

        impl<T> crate::generic::Inc for Baz<T> {
            fn inc(&mut self) {
                self.i += 1;
            }
        }

        type Bar = Baz<std::cell::RefCell<u32>>;

        #[test]
        fn count_struct() {
            global_default_counter!(COUNTER, Bar);
            assert_eq!(
                COUNTER.get_cloned(),
                Baz {
                    i: 0,
                    u: 0,
                    _marker: std::marker::PhantomData
                }
            );
            COUNTER.inc();
            assert_eq!(
                COUNTER.get_cloned(),
                Baz {
                    i: 1,
                    u: 0,
                    _marker: std::marker::PhantomData
                }
            );
            COUNTER.inc();
            assert_eq!(
                COUNTER.get_cloned(),
                Baz {
                    i: 2,
                    u: 0,
                    _marker: std::marker::PhantomData
                }
            );
            COUNTER.inc();
            assert_eq!(
                COUNTER.get_cloned(),
                Baz {
                    i: 3,
                    u: 0,
                    _marker: std::marker::PhantomData
                }
            );
            COUNTER.inc();
            assert_eq!(
                COUNTER.get_cloned(),
                Baz {
                    i: 4,
                    u: 0,
                    _marker: std::marker::PhantomData
                }
            );
            COUNTER.inc();
            assert_eq!(
                COUNTER.get_cloned(),
                Baz {
                    i: 5,
                    u: 0,
                    _marker: std::marker::PhantomData
                }
            );
        }

        #[test]
        fn count_to_50000_single_threaded() {
            global_default_counter!(COUNTER, u32);
            assert_eq!(COUNTER.get_cloned(), 0);

            for _ in 0..50000 {
                COUNTER.inc();
            }

            assert_eq!(COUNTER.get_cloned(), 50000);
        }

        #[test]
        fn count_to_five_seq_threaded() {
            global_default_counter!(COUNTER, u32);
            assert_eq!(COUNTER.get_cloned(), 0);

            let t_0 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            t_0.join().expect("Err joining thread");
            assert_eq!(COUNTER.get_cloned(), 1);

            let t_1 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            t_1.join().expect("Err joining thread");
            assert_eq!(COUNTER.get_cloned(), 2);

            let t_2 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            t_2.join().expect("Err joining thread");
            assert_eq!(COUNTER.get_cloned(), 3);

            let t_3 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            t_3.join().expect("Err joining thread");
            assert_eq!(COUNTER.get_cloned(), 4);

            let t_4 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            t_4.join().expect("Err joining thread");
            assert_eq!(COUNTER.get_cloned(), 5);
        }

        #[test]
        fn count_to_50000_seq_threaded() {
            global_default_counter!(COUNTER, u32);
            assert_eq!(COUNTER.get_cloned(), 0);

            let t_0 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            t_0.join().expect("Err joining thread");
            assert_eq!(COUNTER.get_cloned(), 10000);

            let t_1 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            t_1.join().expect("Err joining thread");
            assert_eq!(COUNTER.get_cloned(), 20000);

            let t_2 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            t_2.join().expect("Err joining thread");
            assert_eq!(COUNTER.get_cloned(), 30000);

            let t_3 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            t_3.join().expect("Err joining thread");
            assert_eq!(COUNTER.get_cloned(), 40000);

            let t_4 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            t_4.join().expect("Err joining thread");
            assert_eq!(COUNTER.get_cloned(), 50000);
        }

        #[test]
        fn count_to_five_par_threaded() {
            global_default_counter!(COUNTER, u32);
            assert_eq!(COUNTER.get_cloned(), 0);

            let t_0 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            let t_1 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            let t_2 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            let t_3 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            let t_4 = std::thread::spawn(|| {
                COUNTER.inc();
            });

            t_0.join().expect("Err joining thread");
            t_1.join().expect("Err joining thread");
            t_2.join().expect("Err joining thread");
            t_3.join().expect("Err joining thread");
            t_4.join().expect("Err joining thread");

            assert_eq!(COUNTER.get_cloned(), 5);
        }

        #[test]
        fn count_to_50000_par_threaded() {
            global_default_counter!(COUNTER, u32);
            assert_eq!(COUNTER.get_cloned(), 0);

            let t_0 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            let t_1 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            let t_2 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            let t_3 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            let t_4 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });

            t_0.join().expect("Err joining thread");
            t_1.join().expect("Err joining thread");
            t_2.join().expect("Err joining thread");
            t_3.join().expect("Err joining thread");
            t_4.join().expect("Err joining thread");

            assert_eq!(COUNTER.get_cloned(), 50000);
        }

        #[test]
        fn reset() {
            global_default_counter!(COUNTER, u32);
            assert_eq!(COUNTER.get_cloned(), 0);
            COUNTER.inc();
            assert_eq!(COUNTER.get_cloned(), 1);
            COUNTER.inc();
            assert_eq!(COUNTER.get_cloned(), 2);
            COUNTER.inc();
            assert_eq!(COUNTER.get_cloned(), 3);

            COUNTER.reset();
            assert_eq!(COUNTER.get_cloned(), 0);
            COUNTER.inc();
            assert_eq!(COUNTER.get_cloned(), 1);
        }
    }

    #[cfg(test)]
    mod primitive {

        use crate::primitive::*;

        #[test]
        fn primitive_new_const() {
            static COUNTERU8: CounterU8 = CounterU8::new(0);
            assert_eq!(COUNTERU8.get(), 0);
            COUNTERU8.inc();
            assert_eq!(COUNTERU8.get(), 1);

            static COUNTERU16: CounterU16 = CounterU16::new(0);
            assert_eq!(COUNTERU16.get(), 0);
            COUNTERU16.inc();
            assert_eq!(COUNTERU16.get(), 1);

            static COUNTERU32: CounterU32 = CounterU32::new(0);
            assert_eq!(COUNTERU32.get(), 0);
            COUNTERU32.inc();
            assert_eq!(COUNTERU32.get(), 1);

            static COUNTERU64: CounterU64 = CounterU64::new(0);
            assert_eq!(COUNTERU64.get(), 0);
            COUNTERU64.inc();
            assert_eq!(COUNTERU64.get(), 1);

            static COUNTERUSIZE: CounterUsize = CounterUsize::new(0);
            assert_eq!(COUNTERUSIZE.get(), 0);
            COUNTERUSIZE.inc();
            assert_eq!(COUNTERUSIZE.get(), 1);

            static COUNTERI8: CounterI8 = CounterI8::new(0);
            assert_eq!(COUNTERI8.get(), 0);
            COUNTERI8.inc();
            assert_eq!(COUNTERI8.get(), 1);

            static COUNTERI16: CounterI16 = CounterI16::new(0);
            assert_eq!(COUNTERI16.get(), 0);
            COUNTERI16.inc();
            assert_eq!(COUNTERI16.get(), 1);

            static COUNTERI32: CounterI32 = CounterI32::new(0);
            assert_eq!(COUNTERI32.get(), 0);
            COUNTERI32.inc();
            assert_eq!(COUNTERI32.get(), 1);

            static COUNTERI64: CounterI64 = CounterI64::new(0);
            assert_eq!(COUNTERI64.get(), 0);
            COUNTERI64.inc();
            assert_eq!(COUNTERI64.get(), 1);

            static COUNTERISIZE: CounterIsize = CounterIsize::new(0);
            assert_eq!(COUNTERISIZE.get(), 0);
            COUNTERISIZE.inc();
            assert_eq!(COUNTERISIZE.get(), 1);
        }

        #[test]
        fn primitive_reset() {
            static COUNTER: CounterU8 = CounterU8::new(0);
            assert_eq!(COUNTER.get(), 0);
            COUNTER.inc();
            assert_eq!(COUNTER.get(), 1);
            COUNTER.inc();
            assert_eq!(COUNTER.get(), 2);
            COUNTER.inc();
            assert_eq!(COUNTER.get(), 3);
            COUNTER.reset();
            assert_eq!(COUNTER.get(), 0);
        }

        #[test]
        fn count_to_five_single_threaded() {
            static COUNTER: CounterU32 = CounterU32::new(0);
            assert_eq!(COUNTER.get(), 0);
            COUNTER.inc();
            assert_eq!(COUNTER.get(), 1);
            COUNTER.inc();
            assert_eq!(COUNTER.get(), 2);
            COUNTER.inc();
            assert_eq!(COUNTER.get(), 3);
            COUNTER.inc();
            assert_eq!(COUNTER.get(), 4);
            COUNTER.inc();
            assert_eq!(COUNTER.get(), 5);
        }

        #[test]
        fn count_to_50000_single_threaded() {
            static COUNTER: CounterU32 = CounterU32::new(0);
            assert_eq!(COUNTER.get(), 0);

            for _ in 0..50000 {
                COUNTER.inc();
            }

            assert_eq!(COUNTER.get(), 50000);
        }

        #[test]
        fn count_to_five_seq_threaded() {
            static COUNTER: CounterU32 = CounterU32::new(0);
            assert_eq!(COUNTER.get(), 0);

            let t_0 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            t_0.join().expect("Err joining thread");
            assert_eq!(COUNTER.get(), 1);

            let t_1 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            t_1.join().expect("Err joining thread");
            assert_eq!(COUNTER.get(), 2);

            let t_2 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            t_2.join().expect("Err joining thread");
            assert_eq!(COUNTER.get(), 3);

            let t_3 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            t_3.join().expect("Err joining thread");
            assert_eq!(COUNTER.get(), 4);

            let t_4 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            t_4.join().expect("Err joining thread");
            assert_eq!(COUNTER.get(), 5);
        }

        #[test]
        fn count_to_50000_seq_threaded() {
            static COUNTER: CounterU32 = CounterU32::new(0);
            assert_eq!(COUNTER.get(), 0);

            let t_0 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            t_0.join().expect("Err joining thread");
            assert_eq!(COUNTER.get(), 10000);

            let t_1 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            t_1.join().expect("Err joining thread");
            assert_eq!(COUNTER.get(), 20000);

            let t_2 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            t_2.join().expect("Err joining thread");
            assert_eq!(COUNTER.get(), 30000);

            let t_3 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            t_3.join().expect("Err joining thread");
            assert_eq!(COUNTER.get(), 40000);

            let t_4 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            t_4.join().expect("Err joining thread");
            assert_eq!(COUNTER.get(), 50000);
        }

        #[test]
        fn count_to_five_par_threaded() {
            static COUNTER: CounterU32 = CounterU32::new(0);
            assert_eq!(COUNTER.get(), 0);

            let t_0 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            let t_1 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            let t_2 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            let t_3 = std::thread::spawn(|| {
                COUNTER.inc();
            });
            let t_4 = std::thread::spawn(|| {
                COUNTER.inc();
            });

            t_0.join().expect("Err joining thread");
            t_1.join().expect("Err joining thread");
            t_2.join().expect("Err joining thread");
            t_3.join().expect("Err joining thread");
            t_4.join().expect("Err joining thread");

            assert_eq!(COUNTER.get(), 5);
        }

        #[test]
        fn count_to_50000_par_threaded() {
            static COUNTER: CounterU32 = CounterU32::new(0);
            assert_eq!(COUNTER.get(), 0);

            let t_0 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            let t_1 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            let t_2 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            let t_3 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });
            let t_4 = std::thread::spawn(|| {
                for _ in 0..10000 {
                    COUNTER.inc();
                }
            });

            t_0.join().expect("Err joining thread");
            t_1.join().expect("Err joining thread");
            t_2.join().expect("Err joining thread");
            t_3.join().expect("Err joining thread");
            t_4.join().expect("Err joining thread");

            assert_eq!(COUNTER.get(), 50000);
        }
    }
}
