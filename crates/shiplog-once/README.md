# shiplog-once

One-time initialization utilities for shiplog.

## Overview

This crate provides one-time initialization primitives:

- `OnceCell`: A cell that can be written to exactly once
- `Lazy`: A lazily initialized value

## Usage

```rust
use shiplog_once::OnceCell;

let cell = OnceCell::new();
let value = cell.get_or_init(|| 42);
assert_eq!(*value, 42);

// Second call returns the same value
let value2 = cell.get_or_init(|| 0);
assert_eq!(*value2, 42);
```
