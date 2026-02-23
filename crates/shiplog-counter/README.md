# shiplog-counter

Counter utilities for shiplog.

## Overview

This crate provides specialized counter functionality:

- [`Counter`] - Basic increment-only counter
- [`WrappingCounter`] - Counter that wraps around at max value
- [`BoundedCounter`] - Counter that stops at max value
- [`DeltaCounter`] - Counter that tracks change since snapshot
- [`CounterRegistry`] - Manage multiple counters

## Usage

```rust
use shiplog_counter::{Counter, WrappingCounter, BoundedCounter, CounterRegistry};

// Basic counter
let mut counter = Counter::new("requests");
counter.inc();
counter.inc_by(5);
assert_eq!(counter.value(), 6);

// Wrapping counter
let mut wrap = WrappingCounter::new("wrap", 100);
wrap.inc(); // wraps at 100

// Bounded counter
let mut bounded = BoundedCounter::new("maxed", 10);
bounded.try_inc(); // returns false when at max

// Registry
let mut registry = CounterRegistry::new();
registry.inc("api_calls");
registry.inc_by("api_calls", 5);
```

## Features

- Multiple counter types (basic, wrapping, bounded, delta)
- Counter registry for management
- Serialization support

## License

MIT OR Apache-2.0
