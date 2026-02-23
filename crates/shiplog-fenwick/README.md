# shiplog-fenwick

Fenwick tree (Binary Indexed Tree) implementation for shiplog.

## Overview

This crate provides Fenwick tree (Binary Indexed Tree) implementations for efficient prefix sum queries and point updates.

## Features

- **FenwickTree**: 1D Fenwick tree for prefix sum queries
  - Point updates: O(log n)
  - Prefix sum queries: O(log n)
  - Range sum queries: O(log n)
  - Lower bound search for prefix sums

- **FenwickTree2D**: 2D Fenwick tree for 2D range sum queries

## Usage

```rust
use shiplog_fenwick::FenwickTree;

// Create from slice
let tree = FenwickTree::from_slice(&[1, 2, 3, 4, 5]);

// Query prefix sum
assert_eq!(tree.sum(2), 6); // 1 + 2 + 3

// Query range sum
assert_eq!(tree.range_sum(1, 3), 9); // 2 + 3 + 4

// Update value
let mut tree = FenwickTree::new(5);
tree.add(0, 10);
assert_eq!(tree.sum(0), 10);
```

## License

MIT OR Apache-2.0
