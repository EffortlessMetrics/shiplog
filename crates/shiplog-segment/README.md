# shiplog-segment

Segment and tree utilities for shiplog.

## Overview

This crate provides utilities for working with segment trees, interval data structures, and binary search trees.

## Features

- **SegmentTree**: A segment tree for range queries with O(log n) updates and queries
- **Interval**: An interval representation [start, end) with overlap detection
- **BinarySearchTree**: A basic BST implementation for ordered data
- **TreeNode**: A binary tree node for building custom tree structures

## Usage

```rust
use shiplog_segment::{Interval, SegmentTree, BinarySearchTree};

// Interval usage
let interval = Interval::new(0, 10);
assert!(interval.contains(5));
assert!(interval.overlaps(&Interval::new(5, 15)));

// Segment tree usage
let tree = SegmentTree::from_slice(&[1, 2, 3, 4]);
let result = tree.query(0, 4);

// BST usage
let mut bst = BinarySearchTree::new();
bst.insert(5);
bst.insert(3);
bst.insert(7);
assert!(bst.search(&5));
```

## License

MIT OR Apache-2.0
