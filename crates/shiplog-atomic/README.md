# shiplog-atomic

Atomic counter utilities for shiplog.

## Overview

This crate provides atomic counter primitives:

- `Counter`: A simple atomic counter
- `AtomicU64`, `AtomicI64`, `AtomicUsize`: Generic atomic wrappers
- `AtomicFlag`: An atomic boolean flag

## Usage

```rust
use shiplog_atomic::Counter;

let counter = Counter::new(0);
counter.increment();
assert_eq!(counter.get(), 1);
```
