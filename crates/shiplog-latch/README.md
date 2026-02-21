# shiplog-latch

Latch and barrier utilities for synchronization in shiplog.

## Overview

This crate provides synchronization primitives for coordinating multiple tasks:

- `CountDownLatch`: A latch that decrements until it reaches zero
- `Barrier`: A barrier that blocks until all parties have reached it

## Usage

```rust
use shiplog_latch::CountDownLatch;

let latch = CountDownLatch::new(3);
// Decrement the latch
latch.count_down();
// Wait for the latch to reach zero
latch.wait();
```
