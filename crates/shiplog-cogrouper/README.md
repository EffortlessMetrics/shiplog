# shiplog-cogrouper

Co-grouping for stream operations in shiplog.

## Overview

This crate provides co-grouping implementations for organizing multiple streams of data by a common key. It supports:

- Co-grouping multiple streams by key
- Finding common keys across streams (intersection)
- Finding all unique keys across streams (union)
- Stream co-grouper with custom key extraction

## Usage

```rust
use shiplog_cogrouper::{CoGrouper, StreamCoGrouper};

// Basic co-grouping
let stream1 = vec![(1, "a"), (1, "b"), (2, "c")];
let stream2 = vec![(1, "x"), (2, "y"), (2, "z")];

let result = CoGrouper::new()
    .add_stream(stream1)
    .add_stream(stream2)
    .execute();

// Find common keys
let common = CoGrouper::new()
    .add_stream(stream1)
    .add_stream(stream2)
    .common_keys();

// Stream co-grouper with key extraction
let items = vec![1, 2, 3, 4];
let cogroup = StreamCoGrouper::new(|x: &i32| x % 2)
    .add_stream(items)
    .execute();
```

## License

MIT OR Apache-2.0
