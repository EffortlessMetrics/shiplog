# shiplog-bst

Binary Search Tree implementation for shiplog.

This crate provides a Binary Search Tree (BST) implementation with support for:
- Insertion
- Search
- Deletion (with all three cases: leaf, one child, two children)
- Inorder traversal (sorted order)
- Min/max operations
- Height calculation

## Usage

```rust
use shiplog_bst::Bst;

// Create a new BST
let mut bst = Bst::new();

// Insert values
bst.insert(5);
bst.insert(3);
bst.insert(7);
bst.insert(1);
bst.insert(9);

// Search for values
assert!(bst.search(&5));
assert!(!bst.search(&10));

// Get sorted values
let sorted = bst.to_sorted_vec();
assert_eq!(sorted, vec![&1, &3, &5, &7, &9]);

// Get min/max
assert_eq!(bst.min(), Some(&1));
assert_eq!(bst.max(), Some(&9));

// Remove a value
bst.remove(&3);

// Iterate in sorted order
for value in bst.iter() {
    println!("{}", value);
}
```

## Features

- Generic type support with `Ord` trait bound
- O(log n) average case for search, insert, delete operations
- Iterator support for sorted traversal
- No external dependencies beyond Rust standard library
