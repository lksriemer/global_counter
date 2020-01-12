pub mod global_counter {

    pub mod primitive {
        use core::sync::atomic::{
            AtomicI16, AtomicI32, AtomicI64, AtomicI8, AtomicU16, AtomicU32, AtomicU64, AtomicU8,
            Ordering,
        };

        macro_rules! primitive_counter {
            ($( $primitive:ident $atomic:ident $counter:ident ), *) => {
                $(
                    /// This is a primitive Counter, implemented using atomics from `std::sync::atomic`.
                    ///
                    /// Regarding atomic ordering, `Ordering::SeqCst` is currently used whenever possible.
                    /// This unstable detail should never be relied on for soundness.
                    ///
                    /// Please note that Atomics may, depending on your compilation target, [be implemented
                    /// using Mutexes](https://llvm.org/docs/Atomics.html),
                    /// meaning lock-freendom can in the general case not be guaranteed.
                    #[derive(Debug, Default)]
                    pub struct $counter($atomic);

                    impl $counter{

                        // TODO: Add method documentation.

                        #[allow(dead_code)]
                        #[inline]
                        pub const fn new(val : $primitive) -> $counter{
                            $counter($atomic::new(val))
                        }

                        #[allow(dead_code)]
                        #[inline]
                        pub fn get(&self) -> $primitive{
                            self.0.load(Ordering::SeqCst)
                        }

                        #[allow(dead_code)]
                        #[inline]
                        pub fn set(&self, val : $primitive){
                            self.0.store(val, Ordering::SeqCst);
                        }

                        #[allow(dead_code)]
                        #[inline]
                        pub fn inc(&self) -> $primitive{
                            self.0.fetch_add(1, Ordering::SeqCst)
                        }

                        #[allow(dead_code)]
                        #[inline]
                        pub fn reset(&self){
                            self.0.store($primitive::default(), Ordering::SeqCst);
                        }
                    }
                )*
            };
        }

        primitive_counter![u8 AtomicU8 CounterU8, u16 AtomicU16 CounterU16, u32 AtomicU32 CounterU32, u64 AtomicU64 CounterU64, i8 AtomicI8 CounterI8, i16 AtomicI16 CounterI16, i32 AtomicI32 CounterI32, i64 AtomicI64 CounterI64];
    }

    pub mod generic {
        use crate::countable::Inc;
        use parking_lot::Mutex;

        /// A generic Counter, counting over `Countables`.
        ///
        /// This counter is `Send + Sync` regardless of its contents, meaning it is always globally available from all threads, concurrently.
        ///
        /// Implement `Countable` for your own types, by implementing `Default + Clone + Inc`.
        /// Implementing `Inc` requires you to supply an impl for incrementing an element of your type.
        ///
        /// Implementation-wise, this is basically a Mutex wrapped in an Arc.
        #[derive(Debug, Default)]
        pub struct Counter<T: Inc>(Mutex<T>);

        /// Creates a new generic, global counter, starting from the given value.
        /// 
        /// This macro is exported at the crates top-level.
        #[macro_export]
        macro_rules! global_counter {
            ($name:ident, $type:ident, $value:ident) => {
                lazy_static::lazy_static! {
                    static ref $name : global_counter::generic::Counter<$type> = global_counter::generic::Counter::new($value);
                }
            };
        }

        /// Creates a new generic, global counter, starting from its (inherited) default value.
        /// 
        /// This macro is exported at the crates top-level.
        #[macro_export]
        macro_rules! global_default_counter {
            ($name:ident, $type:ident) => {
                lazy_static::lazy_static! {
                    static ref $name : global_counter::generic::Counter<$type> = global_counter::generic::Counter::default();
                }
            };
        }

        // TODO: Add method documentation.

        impl<T: Inc> Counter<T> {

            /// Creates a new generic Counter
            /// 
            /// This function is not const yet.
            #[allow(dead_code)]
            #[inline]
            pub fn new(val: T) -> Counter<T> {
                Counter(Mutex::new(val))
            }

            /// Returns (basically) an immutable reference to the underlying value.
            /// 
            /// This is the only way to access the current value of the counter, if T is not `Clone`.
            /// 
            /// 
            #[allow(dead_code)]
            #[inline]
            pub fn get_borrowed(&self) -> impl std::ops::Deref<Target = T> + '_ {
                self.0.lock()
            }

            #[allow(dead_code)]
            #[inline]
            pub fn set(&self, val: T) {
                *self.0.lock() = val;
            }

            #[allow(dead_code)]
            #[inline]
            pub fn inc(&self) {
                (*self.0.lock()).inc();
            }
        }

        impl<T : Inc + Clone> Counter<T>{

            /// Implemented for counter
            #[allow(dead_code)]
            #[inline]
            pub fn get_cloned(&self) -> T {
                (*self.0.lock()).clone()
            }

            #[allow(dead_code)]
            #[inline]
            pub fn inc_cloning(&self) -> T {
                let prev = self.get_cloned();
                self.inc();
                prev
            }
        }

        impl<T: Inc + Default> Counter<T>{
            #[allow(dead_code)]
            #[inline]
            pub fn reset(&self) {
                self.set(T::default());
            }
        }
    }
}

/// This module contains the `Countable` trait,
/// as well as its supertrait `Inc`.
/// Implementations for integer primitives are supplied,
/// however primitive Counters from `global_counter::primitive` should be preferred for performance.
pub mod countable {

    /// This trait abstracts over incrementing behaviour.
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

    imp![u8 u16 u32 u64 u128 i8 i16 i32 i64 i128];
}

#[cfg(test)]
mod tests {

    #[cfg(test)]
    mod generic {

        #![allow(unused_attributes)]
        #[macro_use]
        use crate::*;

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

        #[derive(Clone, Default, PartialEq, Eq, Debug)]
        struct Baz<T> {
            i: i32,
            u: i32,
            _marker: std::marker::PhantomData<T>,
        }

        impl<T> crate::countable::Inc for Baz<T> {
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
                    _marker: core::marker::PhantomData
                }
            );
            COUNTER.inc();
            assert_eq!(
                COUNTER.get_cloned(),
                Baz {
                    i: 1,
                    u: 0,
                    _marker: core::marker::PhantomData
                }
            );
            COUNTER.inc();
            assert_eq!(
                COUNTER.get_cloned(),
                Baz {
                    i: 2,
                    u: 0,
                    _marker: core::marker::PhantomData
                }
            );
            COUNTER.inc();
            assert_eq!(
                COUNTER.get_cloned(),
                Baz {
                    i: 3,
                    u: 0,
                    _marker: core::marker::PhantomData
                }
            );
            COUNTER.inc();
            assert_eq!(
                COUNTER.get_cloned(),
                Baz {
                    i: 4,
                    u: 0,
                    _marker: core::marker::PhantomData
                }
            );
            COUNTER.inc();
            assert_eq!(
                COUNTER.get_cloned(),
                Baz {
                    i: 5,
                    u: 0,
                    _marker: core::marker::PhantomData
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

        // FIXME: Add tests concerning get_cloned and set.
    }

    #[cfg(test)]
    mod primitive {

        use crate::global_counter::primitive::*;

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
