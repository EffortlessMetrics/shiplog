# shiplog-deque

Double-ended queue implementation for shiplog.

This crate provides a double-ended queue (deque) implementation that allows efficient insertion and removal from both ends.

## Features

- Efficient push/pop operations at both front and back
- O(1) operations for all basic operations
- Iterator support
- Rotation and splitting capabilities

## Usage

```rust
use shiplog_deque::Deque;

let mut deque: Deque<i32> = Deque::new();

// Push to both ends
deque.push_front(1);
deque.push_back(2);

// Pop from both ends
let front = deque.pop_front(); // Some(1)
let back = deque.pop_back();    // Some(2)
```

## Operations

- `push_front(item)` - Add item to the front
- `push_back(item)` - Add item to the back
- `pop_front()` - Remove and return item from front
- `pop_back()` - Remove and return item from back
- `front()` - Peek at front item
- `back()` - Peek at back item
- `rotate_left(n)` - Rotate n positions left
- `rotate_right(n)` - Rotate n positions right
- `split_at(n)` - Split into two deques at index n
