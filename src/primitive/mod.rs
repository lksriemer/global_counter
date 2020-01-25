/// This module contains exact primitive counters, implemented using atomics.
pub mod exact;

/// This module contains more performant hybrid counters, implemented using thread-locals and atomics.
///
/// These counters rely on the assumption that thread-locals are faster than global atomics, which they are- on most systems.
pub mod fast;
