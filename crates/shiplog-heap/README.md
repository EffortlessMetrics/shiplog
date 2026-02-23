# shiplog-heap

Binary heap utilities for shiplog.

This crate provides binary heap implementations for efficient priority queue operations and heap sort.

## Features

- **BinaryHeap**: Max-heap implementation (largest element at top)
- **BinaryHeapWithOrder**: Configurable min or max heap
- O(log n) insert and remove operations
- O(1) peek operation

## Usage

```rust
use shiplog_heap::{BinaryHeap, HeapOrder, BinaryHeapWithOrder};

// Using default max heap
let mut heap: BinaryHeap<i32> = BinaryHeap::new();
heap.push(3);
heap.push(1);
heap.push(4);
assert_eq!(heap.pop(), Some(4)); // Largest element

// Using min heap
let mut min_heap = BinaryHeapWithOrder::new(HeapOrder::Min);
min_heap.push(3);
min_heap.push(1);
min_heap.push(4);
assert_eq!(min_heap.pop(), Some(1)); // Smallest element
```

## Heap Types

- `BinaryHeap<T>` - Default max-heap (largest element popped first)
- `BinaryHeapWithOrder<T>` - Configurable with `HeapOrder::Min` or `HeapOrder::Max`

## Operations

- `push(item)` - Add item to heap
- `pop()` - Remove and return top element
- `peek()` - View top element without removing
- `len()` - Get number of elements
- `is_empty()` - Check if heap is empty
- `into_sorted_vec()` - Convert to sorted vector
