# shiplog-split

Stream splitting utilities for shiplog.

## Overview

This crate provides stream splitting implementations for dividing streams of data into multiple output streams. It supports:

- Split by predicate: Separate elements based on a condition
- Split into N: Divide elements into roughly equal partitions
- Partition by function: Group by partition index function
- Group by key: Group elements by key extractor
- Round-robin split: Distribute elements evenly across streams
- Take/Skip: Take or skip first N elements
- Split at: Split at a specific index

## Usage

```rust
use shiplog_split::{StreamSplitter, RoundRobinSplitter};

// Split by predicate
let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

let result = StreamSplitter::new()
    .with_data(data)
    .split_by(|x| x % 2 == 0);

// Split into N partitions
let result = StreamSplitter::new()
    .with_data(data)
    .split_into_n(3);

// Group by key
let items = vec![1, 2, 3, 4, 5];
let groups = StreamSplitter::new()
    .with_data(items)
    .group_by_key(|x| x % 2);

// Round-robin split
let result = RoundRobinSplitter::new(3)
    .with_data(data)
    .execute();
```

## License

MIT OR Apache-2.0
