# shiplog-skiplist

Skip list implementation for shiplog.

This crate provides a Skip List implementation - a probabilistic data structure that allows O(log n) average time complexity for search, insert, and delete operations.

## Usage

```rust
use shiplog_skiplist::SkipList;

// Create a new skip list
let mut list: SkipList<i32> = SkipList::new();

// Insert values
list.insert(5);
list.insert(3);
list.insert(7);

// Search for values
assert!(list.search(&5));
assert!(!list.search(&10));

// Remove a value
list.remove(&3);

// Get all values in sorted order
let values: Vec<_> = list.iter().collect();
```

## Features

- Generic type support with `Ord` trait bound
- O(log n) average case for search, insert, delete operations
- Probabilistic balancing (no rotations needed)
- Iterator support for sorted traversal
- Configurable maximum level

## Performance

Skip lists provide:
- Average case: O(log n) for search, insert, delete
- Worst case: O(n) (but extremely unlikely with proper random seed)
- Space: O(n) expected
