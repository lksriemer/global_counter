# global_counter

[Documentation](https://docs.rs/global_counter/*/global_counter/)

This crate implements global counters, generic and primitive, which build on thoroughly tested synchronization primitives, namely `parking_lot`s Mutex (by default) and the stdlibs atomic types. Faster counters, which trade accuracy for performance, are also available. Refer to the documentation for details.

## Usage

Add the following dependency to your Cargo.toml file:

```toml
[dependencies]
global_counter = "0.2.1"
```

Use the `#[macro_use]` annotation when importing, like this:

```rust
#[macro_use]
extern crate global_counter;
```

If you want to disable using `parking_lot`, and instead use the stdlibs Mutex, disable the default features:

```toml
[dependencies.global_counter]
version = "0.2.1"
default-features = false
```

## Quickstart

### Create a counter

```rust
use global_counter::generic::Counter;
use global_counter::primitive::exact::CounterI16;

// Generic
global_counter!(COUTER_NAME, CountedType, CountedType::default());

// If you feel funny, you can also create a generic global counter without a macro.
// Take a look at the implemtation of the macro, it's simply wrapping in a once_cell::sync::Lazy<...>.

// Primitive
static COUNTER_NAME : CounterI16 = CounterI16::new(0);
```

### Count your counter up

```rust
COUNTER_NAME.inc();
```

### Get the value of your counter

```rust
// Generic
let val_borrowed = COUNTER_NAME.get_borrowed();
// Or
let val = COUNTER_NAME.get_cloned();

// Primitive
let val = COUNTER_NAME.get();

```

## Example - Primitive counter used for indexing into vec from multiple threads

```rust
#[macro_use]
extern crate global_counter;

use global_counter::primitive::exact::CounterUsize;
use std::sync::{Arc, Mutex};

fn main() {
    // This is a primitive counter. Implemented using atomics, more efficient than its generic equivalent.
    // Available for primitive integer types.
    static COUNTER: CounterUsize = CounterUsize::new(0);

    // We want to copy the 'from' arr to the 'to' arr. From multiple threads.
    // Please don't do this in actual code.
    let from = Arc::new(Mutex::new(vec![1, 5, 22, 10000, 43, -4, 39, 1, 2]));
    let to = Arc::new(Mutex::new(vec![0, 0, 0, 0, 0, 0, 0, 0, 0]));

    // 3 elements each in two other threads + 3 elements in this thread.
    // After joining those two threads, all elements will have been copied.
    let to_arc = to.clone();
    let from_arc = from.clone();
    let t1 = std::thread::spawn(move || {
        // '.inc()' increments the counter, returning the previous value.
        let indices = [COUNTER.inc(), COUNTER.inc(), COUNTER.inc()];
        for &i in indices.iter() {
            to_arc.lock().unwrap()[i] = from_arc.lock().unwrap()[i];
        }
    });

    let to_arc = to.clone();
    let from_arc = from.clone();
    let t2 = std::thread::spawn(move || {
        let indices = [COUNTER.inc(), COUNTER.inc(), COUNTER.inc()];
        for &i in indices.iter() {
            to_arc.lock().unwrap()[i] = from_arc.lock().unwrap()[i];
        }
    });

    let indices = [COUNTER.inc(), COUNTER.inc(), COUNTER.inc()];
    for &i in indices.iter() {
        to.lock().unwrap()[i] = from.lock().unwrap()[i];
    }

    t1.join().unwrap();
    t2.join().unwrap();

    assert_eq!(**to.lock().unwrap(), **from.lock().unwrap());
}
```

## Example - No cloning of counted struct

```rust
#[macro_use]
extern crate global_counter;

use global_counter::generic::{Counter, Inc};
use std::collections::LinkedList;
use std::iter::FromIterator;

// Note how this (supposedly) doesn't implement `Clone`.
#[derive(Debug, PartialEq, Eq)]
struct CardinalityCountedList(LinkedList<()>);

// Incrementing to us means just inserting another element.
impl Inc for CardinalityCountedList {
    fn inc(&mut self) {
        self.0.push_back(());
    }
}

// Some helper methods.
impl CardinalityCountedList {
    pub fn with_cardinality(card: usize) -> Self {
        CardinalityCountedList(LinkedList::from_iter(std::iter::repeat(()).take(card)))
    }

    pub fn card(&self) -> usize {
        self.0.len()
    }
}

// We create a new global, thread-safe Counter.
// Could also do this in the main fn or wherever.
global_counter!(
    COUNTER, // Name
    CardinalityCountedList, // Type
    CardinalityCountedList::with_cardinality(0) // Initial value
);

fn main() {
    // Note how we use a borrow, but never clone this LinkedList.
    // Of course, a cloning, convenient API is also available.
    assert_eq!((*COUNTER.get_borrowed()).card(), 0);

    let t1 = std::thread::spawn(move || {
        for _ in 0..(1 << 20) {
            COUNTER.inc();
        }
    });
    let t2 = std::thread::spawn(move || {
        for _ in 0..(1 << 20) {
            COUNTER.inc();
        }
    });

    t1.join().unwrap();

    let card = (*COUNTER.get_borrowed()).card();

    // t1 finished, t2 maybe did something.
    assert!((1 << 20) <= card && card <= (2 << 20));

    t2.join().unwrap();

    // Both threads finished, the counter guarantees `Inc` was executed 2 << 20 times.
    assert_eq!((*COUNTER.get_borrowed()).card(), 2 << 20);
}
```

## Changelog

This library is still being developed. A detailed changelog will be introduced, once a relatively stable state is reached.
Treat every version bump as a breaking change.

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
