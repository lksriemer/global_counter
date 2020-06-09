use core::cell::UnsafeCell;
use core::sync::atomic::{
    AtomicI16, AtomicI32, AtomicI64, AtomicI8, AtomicIsize, AtomicU16, AtomicU32, AtomicU64,
    AtomicU8, AtomicUsize, Ordering,
};
use std::thread::LocalKey;

macro_rules! flushing_counter {
    ($( $primitive:ident $atomic:ident $counter:ident ), *) => {
        $(
            /// A flushing counter.
            ///
            /// This counter is intended to be used in one specific way:
            /// * First, all counting threads increment the counter.
            /// * Every counting thread calls `flush` after it is done incrementing.
            /// * After every flush is guaranteed to have been executed, `get` will return the exact amount of times `inc` has been called (+ the start offset).
            ///
            /// In theory, this counter is equivalent to an approximate counter with its resolution set to infinity.
            pub struct $counter {
                global_counter: $atomic,

                // This could also be a RefCell, but this impl is also safe- or at least I hope so-
                // and more efficient, as no runtime borrowchecking is needed.
                thread_local_counter: &'static LocalKey<UnsafeCell<$primitive>>,
            }

            impl $counter {
                /// Creates a new counter, with the given starting value. Can be used in static contexts.
                #[inline]
                pub const fn new(start: $primitive) -> Self {
                    thread_local!(pub static TL_COUNTER : UnsafeCell<$primitive> = UnsafeCell::new(0));
                    $counter {
                        global_counter: $atomic::new(start),
                        thread_local_counter: &TL_COUNTER,
                    }
                }

                /// Increments the counter by one.
                #[inline]
                pub fn inc(&self) {
                    self.thread_local_counter.with(|tlc| unsafe {
                        // This is safe, because concurrent accesses to a thread-local are obviously not possible,
                        // and aliasing is not possible using the counters API.
                        let tlc = &mut *tlc.get();
                        *tlc += 1;
                    });
                }

                /// Gets the current value of the counter. This only returns the correct value after all local counters have been flushed.
                #[inline]
                pub fn get(&self) -> $primitive {
                    self.global_counter.load(Ordering::Relaxed)
                }

                /// Flushes the local counter to the global.
                #[inline]
                pub fn flush(&self) {
                    self.thread_local_counter.with(|tlc| unsafe {
                        let tlc = &mut *tlc.get();
                        self.global_counter.fetch_add(*tlc, Ordering::Relaxed);
                        *tlc = 0;
                    });
                }
            }
        )*
    };
}
flushing_counter![u8 AtomicU8 FlushingCounterU8, u16 AtomicU16 FlushingCounterU16, u32 AtomicU32 FlushingCounterU32, u64 AtomicU64 FlushingCounterU64, usize AtomicUsize FlushingCounterUsize, i8 AtomicI8 FlushingCounterI8, i16 AtomicI16 FlushingCounterI16, i32 AtomicI32 FlushingCounterI32, i64 AtomicI64 FlushingCounterI64, isize AtomicIsize FlushingCounterIsize];

macro_rules! approx_counter {
    ($( $primitive:ident $atomic:ident $counter:ident $resolution:ty), *) => {
        $(
            /// An approximate counter.
            ///
            /// This counter operates by having a local counter for each thread, which is occasionally flushed to the main global counter.
            ///
            /// The accuracy of the counter is determined by its `resolution` and the number of threads counting on it:
            /// The value returned by `get` is guaranteed to always be less than or to equal this number of threads multiplied with the resolution minus one
            /// away from the actual amount of times `inc` has been called (+ start offset):
            ///
            /// `|get - (actual + start)| <= num_threads * (resolution - 1)`
            ///
            /// With resolution being >= 1. This is the only guarantee made.
            ///
            /// Setting the resolution to 0 or 1 will just make it a worse primitive counter, don't do that. Increasing the resolution increases this counters performance.
            ///
            /// This counter also features a `flush` method,
            /// which can be used to manually flush the local counter of the current thread, increasing the accuracy,
            /// and ultimately making it possible to achieve absolute accuracy
            pub struct $counter {
                // Always making the resolution unsigned was a deliberate choice.
                // The resolution is used to upper-bound an absolute value. It cannot be negative.
                // The thread-local counters have to be unsigned as well, to prevent unnecessary casts.
                threshold: $resolution,
                global_counter: $atomic,
                // This could also be a RefCell, but this impl is also safe- or at least I hope so-
                // and more efficient, as no runtime borrowchecking is needed.
                thread_local_counter: &'static LocalKey<UnsafeCell<$resolution>>,
            }
            impl $counter {
                /// Creates a new counter, with the given start value and resolution. Can be used in static contexts.
                ///
                /// The start value is a lower bound for the value returned by `get`, not guaranteed to be the exact value on subsequent calls.
                #[inline]
                pub const fn new(start: $primitive, resolution: $resolution) -> Self {
                    thread_local!(pub static TL_COUNTER : UnsafeCell<$resolution> = UnsafeCell::new(0));
                    $counter {
                        threshold: resolution,
                        global_counter: $atomic::new(start),
                        thread_local_counter: &TL_COUNTER,
                    }
                }
                /// Increments the counter by one.
                ///
                /// Note that this call will probably leave the value returned by `get` unchanged.
                #[inline]
                pub fn inc(&self) {
                    self.thread_local_counter.with(|tlc| unsafe {
                        // This is safe, because concurrent accesses to a thread-local are obviously not possible,
                        // and aliasing is not possible using the counters API.
                        let tlc = &mut *tlc.get();
                        *tlc += 1;
                        if *tlc >= self.threshold {
                            // These as-casts will be optimized away if the primitive is also unsigned.
                            // Otherwise, they will only occur on this non-hot path.
                            // Same in `flush`.
                            self.global_counter.fetch_add(*tlc as $primitive, Ordering::Relaxed);
                            *tlc = 0;
                        }
                    });
                }
                /// Gets the current value of the counter. For more information, see the struct-level documentation.
                ///
                /// Especially note, that two calls to `get` with one `inc` interleaved are not guaranteed to, and almost certainely wont, return different values.
                #[inline]
                pub fn get(&self) -> $primitive {
                    self.global_counter.load(Ordering::Relaxed)
                }
                /// Flushes the local counter to the global.
                ///
                /// Note that this only means the local counter of the thread calling is flushed. If you want to flush the local counters of multiple threads,
                /// each thread needs to call this method.
                ///
                /// If every thread which incremented this counter has flushed its local counter, and no other increments have been made nor are being made,
                /// a subsequent call to `get` is guaranteed to return the exact count.
                /// However, if you can make use of this, consider if a flushing counter fits your usecase better.
                // TODO: Introduce example(s).
                #[inline]
                pub fn flush(&self) {
                    self.thread_local_counter.with(|tlc| unsafe {
                        let tlc = &mut *tlc.get();
                        self.global_counter.fetch_add(*tlc as $primitive, Ordering::Relaxed);
                        *tlc = 0;
                    });
                }
                // There is no set/reset method, as it would not be compatible with the guarantees made.
                // Specifically, setting the global counter without setting all local counters too, which is hardly possible,
                // would result in the counter going 'out of sync', resulting in an approximation to high.
                // TODO: Evaluate if exposing a set_local, set_global API would be useful and/or idiomatic.
            }
        )*
    };
}
approx_counter![u8 AtomicU8 ApproxCounterU8 u8, u16 AtomicU16 ApproxCounterU16 u16, u32 AtomicU32 ApproxCounterU32 u32, u64 AtomicU64 ApproxCounterU64 u64, usize AtomicUsize ApproxCounterUsize usize, i8 AtomicI8 ApproxCounterI8 u8, i16 AtomicI16 ApproxCounterI16 u16, i32 AtomicI32 ApproxCounterI32 u32, i64 AtomicI64 ApproxCounterI64 u64, isize AtomicIsize ApproxCounterIsize usize];

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn approx_new_const() {
        static COUNTER: ApproxCounterUsize = ApproxCounterUsize::new(0, 1024);
        assert_eq!(COUNTER.get(), 0);
        COUNTER.inc();
        assert!(COUNTER.get() <= 1);
    }

    #[test]
    fn approx_flush_single_threaded() {
        static COUNTER: ApproxCounterU64 = ApproxCounterU64::new(0, 1024);
        assert_eq!(COUNTER.get(), 0);
        COUNTER.inc();
        COUNTER.flush();
        assert_eq!(COUNTER.get(), 1);
    }

    #[test]
    fn approx_negative_start_flush() {
        static COUNTER: ApproxCounterI64 = ApproxCounterI64::new(-1154, 1024);
        assert_eq!(COUNTER.get(), -1154);
        COUNTER.inc();
        COUNTER.flush();
        assert_eq!(COUNTER.get(), -1153);
    }

    #[test]
    fn approx_negative_to_positive() {
        static COUNTER: ApproxCounterI64 = ApproxCounterI64::new(-999, 1000);
        assert_eq!(COUNTER.get(), -999);

        for _ in 0..1000 {
            COUNTER.inc();
        }
        assert!(COUNTER.get() > 0);
    }

    #[test]
    fn approx_count_to_50000_single_threaded() {
        const NUM_THREADS: u32 = 1;
        const LOCAL_ACC: u32 = 1024;
        const GLOBAL_ACC: u32 = LOCAL_ACC * NUM_THREADS;
        static COUNTER: ApproxCounterU32 = ApproxCounterU32::new(0, LOCAL_ACC);
        assert_eq!(COUNTER.get(), 0);

        for _ in 0..50000 {
            COUNTER.inc();
        }

        assert!(50000 - GLOBAL_ACC <= COUNTER.get() && COUNTER.get() <= 50000 + GLOBAL_ACC);
    }

    #[test]
    fn approx_count_to_50000_seq_threaded() {
        const NUM_THREADS: u16 = 5;
        const LOCAL_ACC: u16 = 256;
        const GLOBAL_ACC: u16 = (LOCAL_ACC - 1) * NUM_THREADS;
        static COUNTER: ApproxCounterU16 = ApproxCounterU16::new(0, LOCAL_ACC);
        assert_eq!(COUNTER.get(), 0);

        let t_0 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
        });
        t_0.join().expect("Err joining thread");
        assert!(10000 - GLOBAL_ACC <= COUNTER.get() && COUNTER.get() <= 10000 + GLOBAL_ACC);

        let t_1 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
        });
        t_1.join().expect("Err joining thread");
        assert!(20000 - GLOBAL_ACC <= COUNTER.get() && COUNTER.get() <= 20000 + GLOBAL_ACC);

        let t_2 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
        });
        t_2.join().expect("Err joining thread");
        assert!(30000 - GLOBAL_ACC <= COUNTER.get() && COUNTER.get() <= 30000 + GLOBAL_ACC);

        let t_3 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
        });
        t_3.join().expect("Err joining thread");
        assert!(40000 - GLOBAL_ACC <= COUNTER.get() && COUNTER.get() <= 40000 + GLOBAL_ACC);

        let t_4 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
        });
        t_4.join().expect("Err joining thread");
        assert!(50000 - GLOBAL_ACC <= COUNTER.get() && COUNTER.get() <= 50000 + GLOBAL_ACC);
    }

    #[test]
    fn approx_count_to_50000_par_threaded() {
        const NUM_THREADS: u32 = 5;
        const LOCAL_ACC: u32 = 419;
        const GLOBAL_ACC: u32 = (LOCAL_ACC - 1) * NUM_THREADS;
        static COUNTER: ApproxCounterI32 = ApproxCounterI32::new(0, LOCAL_ACC);
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

        assert!(
            (50000 - GLOBAL_ACC) as i32 <= COUNTER.get()
                && COUNTER.get() <= (50000 + GLOBAL_ACC) as i32
        );
    }

    #[test]
    fn approx_flushed_count_to_50000_par_threaded() {
        const LOCAL_ACC: usize = 419;
        static COUNTER: ApproxCounterIsize = ApproxCounterIsize::new(0, LOCAL_ACC);
        assert_eq!(COUNTER.get(), 0);

        let t_0 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });
        let t_1 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });
        let t_2 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });
        let t_3 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });
        let t_4 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });

        t_0.join().expect("Err joining thread");
        t_1.join().expect("Err joining thread");
        t_2.join().expect("Err joining thread");
        t_3.join().expect("Err joining thread");
        t_4.join().expect("Err joining thread");

        assert_eq!(50000, COUNTER.get());
    }

    #[test]
    fn flushing_new_const() {
        static COUNTER: FlushingCounterUsize = FlushingCounterUsize::new(0);
        assert_eq!(COUNTER.get(), 0);
    }

    #[test]
    fn flushing_count_to_50000_single_threaded() {
        static COUNTER: FlushingCounterU64 = FlushingCounterU64::new(0);
        assert_eq!(COUNTER.get(), 0);

        for _ in 0..50000 {
            COUNTER.inc();
        }

        COUNTER.flush();

        assert_eq!(50000, COUNTER.get());
    }

    #[test]
    fn flushing_count_to_50000_seq_threaded() {
        static COUNTER: FlushingCounterU32 = FlushingCounterU32::new(0);
        assert_eq!(COUNTER.get(), 0);

        let t_0 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });
        t_0.join().expect("Err joining thread");
        assert_eq!(10000, COUNTER.get());

        let t_1 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });
        t_1.join().expect("Err joining thread");
        assert_eq!(20000, COUNTER.get());

        let t_2 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });
        t_2.join().expect("Err joining thread");
        assert_eq!(30000, COUNTER.get());

        let t_3 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });
        t_3.join().expect("Err joining thread");
        assert_eq!(40000, COUNTER.get());

        let t_4 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });
        t_4.join().expect("Err joining thread");
        assert_eq!(50000, COUNTER.get());
    }

    #[test]
    fn flushing_count_to_50000_par_threaded() {
        static COUNTER: FlushingCounterU16 = FlushingCounterU16::new(0);
        assert_eq!(COUNTER.get(), 0);

        let t_0 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });
        let t_1 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });
        let t_2 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });
        let t_3 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });
        let t_4 = std::thread::spawn(|| {
            for _ in 0..10000 {
                COUNTER.inc();
            }
            COUNTER.flush();
        });

        t_0.join().expect("Err joining thread");
        t_1.join().expect("Err joining thread");
        t_2.join().expect("Err joining thread");
        t_3.join().expect("Err joining thread");
        t_4.join().expect("Err joining thread");

        assert_eq!(50000, COUNTER.get());
    }
}
