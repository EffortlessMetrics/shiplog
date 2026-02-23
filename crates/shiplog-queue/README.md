# shiplog-queue

Message queue abstractions for shiplog async task processing.

This crate provides queue implementations for handling asynchronous message processing within the shiplog pipeline, including:

- **MessageQueue**: A thread-safe queue with FIFO ordering and priority support
- **Priority ordering**: Items are dequeued based on priority (higher priority first)
- **Retry tracking**: Built-in support for tracking retry attempts

## Features

- Thread-safe queue operations using RwLock
- Priority-based ordering (higher priority = dequeued first)
- Maximum queue size enforcement
- Retry count tracking for failed items

## Usage

```rust
use shiplog_queue::{MessageQueue, QueueError};

let queue: MessageQueue<String> = MessageQueue::new(100);

// Enqueue items with priority
queue.enqueue("low priority task".to_string(), 1)?;
queue.enqueue("high priority task".to_string(), 10)?;

// Dequeue returns highest priority item first
let item = queue.dequeue().unwrap();
assert_eq!(item.payload, "high priority task");
```

## Priority Queue

The `MessageQueue` implements a priority queue where:
- Higher priority values are dequeued first
- Items with equal priority maintain FIFO order
- Priority can be any integer (positive or negative)
