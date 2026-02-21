# shiplog-joiner

Stream join operations for shiplog.

## Overview

This crate provides join implementations for combining streams of data based on key matching. It supports:

- Inner join: Returns only matching pairs from both streams
- Left outer join: Returns all elements from the left stream, with None for non-matching right elements
- Stream joiner: Generic key-based grouping for streams

## Usage

```rust
use shiplog_joiner::{InnerJoin, LeftJoin, StreamJoiner};

// Inner join example
let left = vec![(1, "apple"), (2, "banana")];
let right = vec![(1, "red"), (2, "yellow")];

let result = InnerJoin::new()
    .with_left_data(left)
    .with_right_data(right)
    .execute();

// Stream joiner with key extraction
let items = vec![1, 2, 3, 4, 5];
let even_joiner = StreamJoiner::new(|&x: &i32| x % 2 == 0);
let groups = even_joiner.add_many(items).group_by_key();
```

## License

MIT OR Apache-2.0
