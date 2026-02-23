# shiplog-union

Union operations for combining streams in shiplog.

## Overview

This crate provides union implementations for combining multiple streams of data. It supports:

- Union All: Include all elements (allow duplicates)
- Union Distinct: Include only unique elements
- Keep First: Keep first occurrence of each element
- Keep Last: Keep last occurrence of each element
- Interleaved Merger: Round-robin merge of streams
- Chained Merger: Sequential append of streams

## Usage

```rust
use shiplog_union::{StreamUnion, UnionMode, InterleavedMerger, ChainedMerger};

// Basic union with distinct mode
let stream1 = vec![1, 2, 3];
let stream2 = vec![3, 4, 5];

let result = StreamUnion::new()
    .add_stream(stream1)
    .add_stream(stream2)
    .with_mode(UnionMode::Distinct)
    .execute();

// Interleaved merger - round robin
let result = InterleavedMerger::new()
    .add_stream(vec![1, 2, 3])
    .add_stream(vec![4, 5, 6])
    .execute();

// Chained merger - sequential
let result = ChainedMerger::new()
    .add_stream(vec![1, 2, 3])
    .add_stream(vec![4, 5, 6])
    .execute();
```

## License

MIT OR Apache-2.0
