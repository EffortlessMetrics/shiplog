# shiplog-ring

Ring buffer (circular buffer) implementation for shiplog.

## Usage

```rust
use shiplog_ring::RingBuffer;

let mut buffer: RingBuffer<i32> = RingBuffer::new(3);
buffer.push(1);
buffer.push(2);
buffer.push(3);
assert_eq!(buffer.pop(), Some(1));
assert_eq!(buffer.len(), 2);
```
