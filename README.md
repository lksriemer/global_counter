# global_counter
Sometimes you just want to count something globally, and you really dont want to worry to much about data races, other race conditions, all the fun stuff.

## Usage

Add the following dependency to your Cargo.toml file:

```toml
[dependencies]
global_counter = "0.1.2"
```

And use the `#[macro_use]` annotation when importing:

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

// Note how this doesnt implement `Clone`.
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
    // Note how we use a borrow, but never clone this HashSet.
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

    // Both threads finished, the counter guarantees `Inc` was executed 1 << 20 times.
    assert_eq!((*COUNTER.get_borrowed()).card(), 2 << 20);
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
