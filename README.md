# global_counter

Sometimes you just want to count something globally, and you really dont want to worry to much about data races, other race conditions, all the fun stuff.

That's what this crate is for. It supplies global counters, which build on thoroughly tested synchronization primitives, namely `parking_lot`s Mutex  for the generic counter and the stdlibs atomic types for the primitive counters.

## Usage

Add the following dependency to your Cargo.toml file:

```toml
[dependencies]
global_counter = "0.1.3"
```

Use the `#[macro_use]` annotation when importing, like this:

```rust
#[macro_use]
extern crate global_counter;
```

## Examples

```rust
#[macro_use]
extern crate global_counter;

use global_counter::generic::Inc;
use std::collections::LinkedList;
use std::iter::FromIterator;

// Note how this (supposedly) doesnt implement `Clone`.
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
// Could also do this in the main fn.
global_counter!(
    COUNTER,
    CardinalityCountedList,
    CardinalityCountedList::with_cardinality(0)
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

```rust
#[macro_use]
extern crate global_counter;

use global_counter::primitive::CounterUsize;
use std::sync::{Arc, Mutex};

fn main() {
    // This is a primitive counter. Implemented using atomics, more efficient than its generic equivalent.
    // Available for primitive integer types.
    static COUNTER: CounterUsize = CounterUsize::new(0);

    // We want to copy the 'from' arr to the 'to' arr. From multiple threads.
    // Please don't do this in actual code.
    let from = Arc::new(Mutex::new(vec![1, 5, 22, 10000, 43, -4, 39, 1, 2]));
    let to = Arc::new(Mutex::new(vec![0, 0, 0, 0, 0, 0, 0, 0, 0]));

    // 3 elemets in two other threads + 3 elements in this thread.
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

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
