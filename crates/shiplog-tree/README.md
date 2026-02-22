# shiplog-tree

Generic tree implementations (Binary, AVL, Red-Black) for shiplog.

This crate provides implementations of common tree data structures:

- **BinaryTree**: Basic binary search tree implementation
- **AvlTree**: Self-balancing AVL tree with O(log n) worst-case operations
- **RedBlackTree**: Self-balancing red-black tree with O(log n) worst-case operations

## Usage

```rust
use shiplog_tree::{BinaryTree, AvlTree, RedBlackTree};

// Binary Search Tree
let mut bst = BinaryTree::new();
bst.insert(5);
bst.insert(3);
bst.insert(7);
assert!(bst.search(&5));

// AVL Tree (self-balancing)
let mut avl = AvlTree::new();
avl.insert(5);
avl.insert(3);
avl.insert(7);
assert!(avl.search(&5));

// Red-Black Tree (self-balancing)
let mut rbt = RedBlackTree::new();
rbt.insert(5);
rbt.insert(3);
rbt.insert(7);
assert!(rbt.search(&5));
```

## Features

- Generic type support
- Efficient O(log n) search, insert operations for balanced trees
- No external dependencies beyond Rust standard library
