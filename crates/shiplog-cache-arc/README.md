# shiplog-cache-arc

Arc-based thread-safe cache implementation for shiplog.

## Overview

This crate provides a thread-safe cache implementation using `Arc` (Atomically Reference Counted) for sharing cache data across threads. It uses a `RwLock` for concurrent read access.

## Features

- Thread-safe cache using Arc and RwLock
- Bounded cache with automatic eviction
- Cloneable cache handles
- Minimal locking overhead for reads

## Usage

```rust
use shiplog_cache_arc::{ArcCache, BoundedArcCache};

// Basic thread-safe cache
let cache = ArcCache::new();
cache.put("key", "value");
let value = cache.get(&"key");

// Bounded cache with max size
let bounded = BoundedArcCache::new(100);
bounded.put("a", 1);
bounded.put("b", 2);
```

## License

MIT OR Apache-2.0
