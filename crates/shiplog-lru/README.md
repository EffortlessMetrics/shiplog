# shiplog-lru

LRU (Least Recently Used) cache implementation for shiplog.

## Overview

This crate provides an in-memory LRU cache implementation that evicts the least recently used items when the capacity is reached.

## Usage

```rust
use shiplog_lru::LruCache;

let mut cache = LruCache::new(3);
cache.insert(1, "one");
cache.insert(2, "two");
cache.insert(3, "three");

// Access makes it recently used
assert_eq!(cache.get(&1), Some(&"one"));

// Inserting a new item evicts LRU
cache.insert(4, "four");
assert!(!cache.contains(&2)); // Evicted!
```

## Features

- Fixed capacity with automatic eviction
- O(1) get and insert operations
- Thread-unsafe (use interior mutability for thread safety)

## License

MIT OR Apache-2.0
