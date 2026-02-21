# shiplog-priorityqueue

Priority queue implementation for shiplog.

This crate provides a priority queue implementation for handling items with different priority levels.

## Features

- **Priority-based ordering**: Higher priority items are dequeued first
- **FIFO tie-breaking**: Items with same priority maintain insertion order
- **Configurable max size**: Set maximum queue capacity
- **Peek operations**: View without removing
- **O(log n)** insert and remove operations

## Usage

```rust
use shiplog_priorityqueue::{PriorityQueue, PriorityItem};

let mut pq: PriorityQueue<&str> = PriorityQueue::new(10);

// Push items with different priorities
pq.push("low priority", 1).unwrap();
pq.push("high priority", 10).unwrap();
pq.push("medium priority", 5).unwrap();

// Pop returns highest priority first
let item = pq.pop().unwrap();
assert_eq!(item.item, "high priority");
assert_eq!(item.priority, 10);
```

## PriorityItem

The `PriorityItem<T>` struct contains:
- `item`: The actual data being stored
- `priority`: Integer priority value (higher = more important)
- `seq`: Sequence number for FIFO ordering of equal priorities

## Operations

- `push(item, priority)` - Add item with given priority
- `pop()` - Remove and return highest priority item
- `peek()` - View highest priority item
- `peek_mut()` - Mutably view highest priority item
- `len()` - Get number of items
- `is_empty()` - Check if empty
- `is_full()` - Check if at max capacity
- `clear()` - Remove all items
