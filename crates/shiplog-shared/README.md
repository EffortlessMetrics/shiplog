# shiplog-shared

Shared state utilities for shiplog.

## Overview

This crate provides shared state management utilities:

- `SharedState`: A wrapper for safely sharing state across tasks
- `StateGuard`: A guard that provides exclusive access to shared state

## Usage

```rust
use shiplog_shared::SharedState;

let state = SharedState::new(42);
// Get a guard to access the state
let guard = state.read().await;
assert_eq!(*guard, 42);
```
