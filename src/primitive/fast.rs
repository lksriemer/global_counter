use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};
use std::thread::LocalKey;

/// A flushing counter.
///
/// This counter is intended to be used in one specific way:
/// * First, all counting threads increment the counter.
/// * Every counting thread calls `flush` after it is done incrementing.
/// * Then, after every flush is guaranteed to have been executed, `get` will return the exact amount of times `inc` has been called (+ the start offset).
///
/// In theory, this counter is equivalent to an approximate counter with its resolution set to infinity.
///
/// This counter is only available for usize, if you need other types drop by the repo and open an issue.
/// I wasn't able to think of a reason why somebody would want to flush count using i8s.
pub struct FlushingCounter {
    global_counter: AtomicUsize,

    // This could also be a RefCell, but this impl is also safe- or at least I hope so-
    // and more efficient, as no runtime borrowchecking is needed.
    thread_local_counter: &'static LocalKey<UnsafeCell<usize>>,
}

impl FlushingCounter {
    /// Creates a new counter, with the given starting value. Can be used in static contexts.
    #[inline]
    pub const fn new(start: usize) -> Self {
        thread_local!(pub static TL_COUNTER : UnsafeCell<usize> = UnsafeCell::new(0));
        FlushingCounter {
            global_counter: AtomicUsize::new(start),
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
    pub fn get(&self) -> usize {
        self.global_counter.load(Ordering::Relaxed)
    }

    /// Flushes the local counter to the global.
    ///
    /// For more information, see the struct-level documentation.
    #[inline]
    pub fn flush(&self) {
        self.thread_local_counter.with(|tlc| unsafe {
            let tlc = &mut *tlc.get();
            self.global_counter.fetch_add(*tlc, Ordering::Relaxed);
            *tlc = 0;
        });
    }
}

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
///
/// This is the only guarantee made.
///
/// Setting the resolution to 1 will just make it a worse primitive counter, don't do that. Increasing the resolution increases this counters performance.
///
/// This counter also features a `flush` method,
/// which can be used to manually flush the local counter of the current thread, increasing the accuracy,
/// and ultimately making it possible to achieve absolute accuracy.
///
/// This counter is only available for usize, if you need other types drop by the repo and open an issue.
/// I wasn't able to think of a reason why somebody would want to approximately count using i8s.
pub struct ApproxCounter {
    threshold: usize,
    global_counter: AtomicUsize,

    // This could also be a RefCell, but this impl is also safe- or at least I hope so-
    // and more efficient, as no runtime borrowchecking is needed.
    thread_local_counter: &'static LocalKey<UnsafeCell<usize>>,
}

impl ApproxCounter {
    /// Creates a new counter, with the given start value and resolution. Can be used in static contexts.
    #[inline]
    pub const fn new(start: usize, resolution: usize) -> Self {
        thread_local!(pub static TL_COUNTER : UnsafeCell<usize> = UnsafeCell::new(0));
        ApproxCounter {
            threshold: resolution,
            global_counter: AtomicUsize::new(start),
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
                self.global_counter.fetch_add(*tlc, Ordering::Relaxed);
                *tlc = 0;
            }
        });
    }

    /// Gets the current value of the counter. For more information, see the struct-level documentation.
    ///
    /// Especially note, that two calls to `get` with one `inc` interleaved are not guaranteed to, and almost certainely wont, return different values.
    #[inline]
    pub fn get(&self) -> usize {
        self.global_counter.load(Ordering::Relaxed)
    }

    /// Flushes the local counter to the global.
    ///
    /// Note that this only means the local counter of the thread calling is flushed. If you want to flush the local counters of multiple threads,
    /// each thread needs to call this method.
    ///
    /// If every thread which incremented this counter has flushed its local counter, and no other increments have been made or are being made,
    /// a subsequent call to `get` is guaranteed to return the exact count.
    /// However, if you can make use of this, consider if a [FlushingCounter](struct.FlushingCounter.html) fits your usecase better.
    // TODO: Introduce example(s).
    #[inline]
    pub fn flush(&self) {
        self.thread_local_counter.with(|tlc| unsafe {
            let tlc = &mut *tlc.get();
            self.global_counter.fetch_add(*tlc, Ordering::Relaxed);
            *tlc = 0;
        });
    }

    // There is no set/reset method, as it would not be compatible with the guarantees made.
    // Specifically, setting the global counter without setting all local counters too, which is hardly possible,
    // would result in the counter going 'out of sync', resulting in an approximation to high.
    // TODO: Evalaute, if exposing a set_local, set_global API would be useful and/or idiomatic.
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn approx_new_const() {
        static COUNTER: ApproxCounter = ApproxCounter::new(0, 1024);
        assert_eq!(COUNTER.get(), 0);
        COUNTER.inc();
        assert!(COUNTER.get() <= 1);
    }

    #[test]
    fn approx_flush_single_threaded() {
        static COUNTER: ApproxCounter = ApproxCounter::new(0, 1024);
        assert_eq!(COUNTER.get(), 0);
        COUNTER.inc();
        COUNTER.flush();
        assert_eq!(COUNTER.get(), 1);
    }

    #[test]
    fn approx_count_to_50000_single_threaded() {
        const NUM_THREADS: usize = 1;
        const LOCAL_ACC: usize = 1024;
        const GLOBAL_ACC: usize = LOCAL_ACC * NUM_THREADS;
        static COUNTER: ApproxCounter = ApproxCounter::new(0, LOCAL_ACC);
        assert_eq!(COUNTER.get(), 0);

        for _ in 0..50000 {
            COUNTER.inc();
        }

        assert!(50000 - GLOBAL_ACC <= COUNTER.get() && COUNTER.get() <= 50000 + GLOBAL_ACC);
    }

    #[test]
    fn approx_count_to_50000_seq_threaded() {
        const NUM_THREADS: usize = 5;
        const LOCAL_ACC: usize = 256;
        const GLOBAL_ACC: usize = (LOCAL_ACC - 1) * NUM_THREADS;
        static COUNTER: ApproxCounter = ApproxCounter::new(0, LOCAL_ACC);
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
        const NUM_THREADS: usize = 5;
        const LOCAL_ACC: usize = 419;
        const GLOBAL_ACC: usize = (LOCAL_ACC - 1) * NUM_THREADS;
        static COUNTER: ApproxCounter = ApproxCounter::new(0, LOCAL_ACC);
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

        assert!(50000 - GLOBAL_ACC <= COUNTER.get() && COUNTER.get() <= 50000 + GLOBAL_ACC);
    }

    #[test]
    fn approx_flushed_count_to_50000_par_threaded() {
        const LOCAL_ACC: usize = 419;
        static COUNTER: ApproxCounter = ApproxCounter::new(0, LOCAL_ACC);
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
        static COUNTER: FlushingCounter = FlushingCounter::new(0);
        assert_eq!(COUNTER.get(), 0);
    }

    #[test]
    fn flushing_count_to_50000_single_threaded() {
        static COUNTER: FlushingCounter = FlushingCounter::new(0);
        assert_eq!(COUNTER.get(), 0);

        for _ in 0..50000 {
            COUNTER.inc();
        }

        COUNTER.flush();

        assert_eq!(50000, COUNTER.get());
    }

    #[test]
    fn flushing_count_to_50000_seq_threaded() {
        static COUNTER: FlushingCounter = FlushingCounter::new(0);
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
        static COUNTER: FlushingCounter = FlushingCounter::new(0);
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
