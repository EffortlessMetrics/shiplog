# shiplog-cache-lru

LRU (Least Recently Used) cache implementation for shiplog.

## Overview

This crate provides an in-memory LRU cache with a fixed capacity. When the cache is full, the least recently used item is automatically evicted to make room for new items.

## Features

- Fixed capacity with automatic eviction
- O(1) get and put operations
- Thread-unsafe (use with interior mutability for thread-safe access)
- Simple and lightweight implementation

## Usage

```rust
use shiplog_cache_lru::LruCache;

let mut cache = LruCache::new(3);

// Insert items
cache.put("a", 1);
cache.put("b", 2);
cache.put("c", 3);

// Get an item (updates access order)
let value = cache.get(&"a");

// Cache automatically evicts least recently used item
cache.put("d", 4); // "b" is evicted
```

## License

MIT OR Apache-2.0
