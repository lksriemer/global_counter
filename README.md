# global_counter
Sometimes you just want to count something globally, and you really dont want to worry to much about data races, other race conditions, all the fun stuff.

## Usage

Add the following dependency to your Cargo.toml file:

```toml
[dependencies]
global_counter = "0.1.0"
```

And use the `#[macro_use]` annotation when importing:

```rust
#[macro_use]
extern crate global_counter;
```

## Examples

TODO: Add examples

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
