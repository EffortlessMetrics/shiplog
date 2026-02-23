# shiplog-ttl

TTL (time-to-live) cache utilities for shiplog.

## Overview

This crate provides utilities for managing time-to-live entries in caches, including:
- `TtlEntry<T>` - A value with an associated expiration time
- `TtlCache<K, V>` - A simple in-memory TTL cache
- Helper functions for creating TTL durations

## Usage

```rust
use shiplog_ttl::{TtlCache, ttl_from_secs};

let mut cache = TtlCache::new(ttl_from_secs(3600));
cache.insert("key", "value");

// Get value if not expired
if let Some(val) = cache.get(&"key") {
    println!("Got: {}", val);
}

// Cleanup expired entries
let cleaned = cache.cleanup();
```

## Features

- Simple in-memory TTL cache
- Expiration checking
- Automatic cleanup of expired entries
- Configurable TTL per entry
