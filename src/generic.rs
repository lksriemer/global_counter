#![allow(unused_macros)]

#[cfg(parking_lot)]
use parking_lot::Mutex;

#[cfg(not(parking_lot))]
use std::sync::Mutex;

/// This trait promises incrementing behaviour.
/// Implemented for standard integer types.
/// The current value is mutated, becoming the new, incremented value.
///
/// Implement this trait for the types you want to generically count on.
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

/// A generic, gobal counter.
///
/// This counter holds up rusts guarantees of freedom of data-races. Any caveats are clearly pointed out in the documentation.
///
/// This counter is implemented using a Mutex, which can be slow if a lot of contention is involved.
/// To circumvent this, consider extracting the 'counted parts' of your struct into primitives,
/// which can then be counted by much faster primitive counters. Abstracting can then restore the original interface.
///
/// Avoid premature optimzation though!
#[derive(Debug, Default)]
pub struct Counter<T: Inc>(Mutex<T>);

/// Creates a new global, generic counter, starting from the given value.
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
        static $name: ::global_counter::global_counter_macro_dependencies::Lazy<::global_counter::generic::Counter<$type>> =
        ::global_counter::global_counter_macro_dependencies::Lazy::new(|| ::global_counter::generic::Counter::new($value));
    };
}

// A hack for local usage.
macro_rules! global_counter_2 {
    ($name:ident, $type:ident, $value:expr) => {
        use once_cell::sync::Lazy;
        static $name: Lazy<Counter<$type>> =
            Lazy::new(|| Counter::new($value));
    };
}

/// Creates a new generic, global counter, starting from its default value.
///
/// This macro will fail compilation if the given type is not `Default`.
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
    ($name:ident, $type:ident) => {
        global_counter!($name, $type, $type::default());
    };
}

// A hack for local usage.
macro_rules! global_default_counter_2{
    ($name:ident, $type:ident) => {
        global_counter_2!($name, $type, $type::default());
    };
}

impl<T: Inc> Counter<T> {
    /// Creates a new generic counter.
    ///
    /// This function is not const yet. As soon as [Mutex::new()](https://docs.rs/lock_api/*/lock_api/struct.Mutex.html#method.new) is stable as `const fn`, this will be as well, if the `parking_lot` feature is not disabled.
    /// Then, the exported macros will no longer be needed.
    #[inline]
    pub fn new(val: T) -> Counter<T> {
        Counter(Mutex::new(val))
    }

    /// Returns (basically) an immutable borrow of the underlying value.
    /// Best make sure this borrow goes out of scope before any other methods of the counter are being called.
    ///
    /// If `T` is not `Clone`, this is the only way to access the current value of the counter.
    ///
    /// **Warning**: Attempting to access the counter from the thread holding this borrow will result in a deadlock or panic.
    /// This is usual mutex behaviour.
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
    /// # Bad Example
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
    #[inline]
    pub fn get_borrowed(&self) -> impl std::ops::Deref<Target = T> + '_ {
        self.lock()
    }

    /// Returns a mutable borrow of the counted value, meaning the actual value counted by this counter can be mutated through this borrow.
    ///
    /// The constraints pointed out for [get_borrowed](struct.Counter.html#method.get_borrowed) also apply here.
    ///
    /// Although this API is in theory as safe as its immutable equivalent, usage of it is discouraged, as it is highly unidiomatic.
    #[inline]
    pub fn get_mut_borrowed(&self) -> impl std::ops::DerefMut<Target = T> + '_ {
        self.lock()
    }

    /// Sets the counted value to the given value.
    #[inline]
    pub fn set(&self, val: T) {
        *self.lock() = val;
    }

    /// Increments the counter, delegating the specific implementation to the [Inc](trait.Inc.html) trait.
    #[inline]
    pub fn inc(&self) {
        self.lock().inc();
    }

    #[cfg(parking_lot)]
    #[inline]
    fn lock(&self) -> impl std::ops::DerefMut<Target = T> + '_ {
        self.0.lock()
    }

    #[cfg(not(parking_lot))]
    #[inline]
    fn lock(&self) -> impl std::ops::DerefMut<Target = T> + '_ {
        self.0.lock().expect("Global counter lock failed. This indicates another user paniced while holding a lock to the counter.")
    }
}

impl<T: Inc + Clone> Counter<T> {
    /// This avoid the troubles of [get_borrowed](struct.Counter.html#method.get_borrowed) by cloning the current value.
    ///
    /// Creating a deadlock using this API should be impossible, it might however violate implicit synchronization assumptions.
    #[inline]
    pub fn get_cloned(&self) -> T {
        self.lock().clone()
    }

    /// Increments the counter, returning the previous value, cloned.
    #[inline]
    pub fn inc_cloning(&self) -> T {
        let mut locked = self.lock();
        let prev = locked.clone();
        locked.inc();
        prev
    }
}

impl<T: Inc + Default> Counter<T> {
    /// Resets the counter to its default value.
    #[inline]
    pub fn reset(&self) {
        self.set(T::default());
    }
}

#[cfg(test)]
mod tests {
    use crate::generic::Counter;

    // TODO: Clean up this mess.
    // Maybe move all test helper structs to an extra module.

    #[derive(Default, PartialEq, Eq, Debug)]
    struct PanicOnClone(i32);

    impl Clone for PanicOnClone {
        fn clone(&self) -> Self {
            panic!("PanicOnClone cloned");
        }
    }

    impl crate::generic::Inc for PanicOnClone {
        fn inc(&mut self) {
            self.0.inc();
        }
    }

    #[test]
    fn get_borrowed_doesnt_clone() {
        global_default_counter_2!(COUNTER, PanicOnClone);
        assert_eq!(*COUNTER.get_borrowed(), PanicOnClone(0));
    }

    #[test]
    fn get_mut_borrowed_doesnt_clone() {
        global_counter_2!(COUNTER, PanicOnClone, PanicOnClone(0));
        assert_eq!(*COUNTER.get_mut_borrowed(), PanicOnClone(0));
    }

    #[test]
    fn count_to_five_single_threaded() {
        global_default_counter_2!(COUNTER, u32);
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
        global_default_counter_2!(COUNTER, Bar);
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
        global_default_counter_2!(COUNTER, u32);
        assert_eq!(COUNTER.get_cloned(), 0);

        for _ in 0..50000 {
            COUNTER.inc();
        }

        assert_eq!(COUNTER.get_cloned(), 50000);
    }

    #[test]
    fn count_to_five_seq_threaded() {
        global_default_counter_2!(COUNTER, u32);
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
        global_default_counter_2!(COUNTER, u32);
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
        global_default_counter_2!(COUNTER, u32);
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
        global_default_counter_2!(COUNTER, u32);
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
        global_default_counter_2!(COUNTER, u32);
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
