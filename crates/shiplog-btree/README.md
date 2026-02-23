# shiplog-btree

B-Tree implementation for shiplog.

This crate provides a B-Tree implementation - a self-balancing tree data structure that maintains sorted data and allows efficient insertion, deletion, and search operations.

## Usage

```rust
use shiplog_btree::BTree;

// Create a new B-Tree with minimum degree 3
let mut tree: BTree<i32> = BTree::new(3);

// Insert values
tree.insert(5);
tree.insert(3);
tree.insert(7);
tree.insert(1);
tree.insert(9);

// Search for values
assert!(tree.search(&5));
assert!(!tree.search(&10));

// Remove a value
tree.remove(&3);

// Get all values in sorted order
let sorted = tree.to_vec();
assert_eq!(sorted, vec![&1, &5, &7, &9]);

// Iterate in sorted order
for value in tree.iter() {
    println!("{}", value);
}
```

## Features

- Generic type support with `Ord` trait bound
- O(log n) time complexity for search, insert, delete operations
- Configurable minimum degree
- Optimized for disk-based storage and cache efficiency
- Iterator support for sorted traversal

## B-Tree Properties

For a B-Tree of minimum degree `t`:
- Each node has at most `2t - 1` keys
- Each node (except root) has at least `t - 1` keys
- Internal nodes have at least `t` children
- All leaves appear at the same level

## Performance

B-Trees are particularly efficient for:
- Systems that read and write large blocks of data
- Database indexing
- File systems
- Cache-efficient access patterns
