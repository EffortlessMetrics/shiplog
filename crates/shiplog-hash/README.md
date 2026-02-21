# shiplog-hash

Hashing utilities for shiplog.

[![Crates.io](https://img.shields.io/crates/v/shiplog-hash)](https://crates.io/crates/shiplog-hash)
[![Docs.rs](https://docs.rs/shiplog-hash/badge.svg)](https://docs.rs/shiplog-hash)

## Usage

```rust
use shiplog_hash::{hash_content, Hash, Checksum};

let hash = hash_content("hello world");
println!("Hash: {}", hash);

let checksum = Checksum::sha256("hello world");
println!("Checksum: {}", checksum);
```

## Features

- `Hash` - SHA-256 hash wrapper with utilities
- `hash_content` - Simple content hashing
- `hash_items` - Hash multiple items with separators
- `ContentHasher` - Incremental hasher for streaming content
- `Checksum` - Checksum type with verification
