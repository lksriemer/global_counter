use std::sync::atomic::{
    AtomicI16, AtomicI32, AtomicI64, AtomicI8, AtomicIsize, AtomicU16, AtomicU32, AtomicU64,
    AtomicU8, AtomicUsize, Ordering,
};

macro_rules! primitive_counter {
        ($( $primitive:ident $atomic:ident $counter:ident ), *) => {
            $(
                /// An atomic primitive counter.
                ///
                /// This counter makes all the same guarantees a generic counter does.
                /// Especially, calling `inc` N times from different threads will always result in the counter effectively being incremented by N.
                /// The counters `get` method will always return exactly the amount of times, `inc` has been called (+ start offset), up to this moment.
                ///
                /// Please note that Atomics may, depending on your compilation target, not be implemented using atomic instructions
                /// (See [here](https://llvm.org/docs/Atomics.html), 'Atomics and Codegen', l.7-11).
                /// Meaning, although lock-freedom is always guaranteed, wait-freedom is not.
                ///
                /// The given atomic ordering is rusts [core::sync::atomic::Ordering](https://doc.rust-lang.org/core/sync/atomic/enum.Ordering.html),
                /// with `AcqRel` translating to `AcqRel`, `Acq` or `Rel`, depending on the operation performed.
                ///
                /// This counter should in general be superior in performance, compared to the equivalent generic counter.
                #[derive(Debug)]
                pub struct $counter($atomic, Ordering);

                impl $counter{
                    /// Creates a new primitive counter. Can be used in const contexts.
                    /// Uses the default `Ordering::SeqCst`, making the strongest ordering guarantees.
                    #[inline]
                    pub const fn new(val : $primitive) -> $counter{
                        $counter($atomic::new(val), Ordering::SeqCst)
                    }

                    /// Creates a new primitive counter with the given atomic ordering. Can be used in const contexts.
                    ///
                    /// Possible orderings are `Relaxed`, `AcqRel` and `SeqCst`.
                    /// Supplying an other ordering is undefined behaviour.
                    #[inline]
                    pub const fn with_ordering(val : $primitive, ordering : Ordering) -> $counter{
                        $counter($atomic::new(val), ordering)
                    }

                    /// Gets the current value of the counter.
                    #[inline]
                    pub fn get(&self) -> $primitive{
                        self.0.load(match self.1{ Ordering::AcqRel => Ordering::Acquire, other => other })
                    }

                    /// Sets the counter to a new value.
                    #[inline]
                    pub fn set(&self, val : $primitive){
                        self.0.store(val, match self.1{ Ordering::AcqRel => Ordering::Release, other => other });
                    }

                    /// Increments the counter by one, returning the previous value.
                    #[inline]
                    pub fn inc(&self) -> $primitive{
                        self.0.fetch_add(1, self.1)
                    }

                    /// Resets the counter to zero.
                    #[inline]
                    pub fn reset(&self){
                        self.0.store(0, match self.1{ Ordering::AcqRel => Ordering::Release, other => other });
                    }
                }
            )*
        };
    }

primitive_counter![u8 AtomicU8 CounterU8, u16 AtomicU16 CounterU16, u32 AtomicU32 CounterU32, u64 AtomicU64 CounterU64, usize AtomicUsize CounterUsize, i8 AtomicI8 CounterI8, i16 AtomicI16 CounterI16, i32 AtomicI32 CounterI32, i64 AtomicI64 CounterI64, isize AtomicIsize CounterIsize];

#[cfg(test)]
mod tests {

    use super::*;

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

    // FIXME: Add with_ordering test.

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
