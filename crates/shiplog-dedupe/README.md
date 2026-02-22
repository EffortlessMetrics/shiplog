# shiplog-dedupe

Deduplication utilities for shiplog.

[![Crates.io](https://img.shields.io/crates/v/shiplog-dedupe)](https://crates.io/crates/shiplog-dedupe)
[![Docs.rs](https://docs.rs/shiplog-dedupe/badge.svg)](https://docs.rs/shiplog-dedupe)

## Usage

```rust
use shiplog_dedupe::{dedupe_strings, dedupe_by_key, Deduplicator, DedupConfig};

let strings = vec!["hello", "world", "hello"];
let unique = dedupe_strings(&strings);
assert_eq!(unique.len(), 2);
```

## Features

- `DedupKey` - Keys for deduplication with content hashing
- `DedupConfig` - Configuration for deduplication behavior
- `Deduplicator` - Generic deduplicator with custom key functions
- `dedupe_strings` - Simple string deduplication
- `dedupe_by_key` - Deduplicate items by extracted key
