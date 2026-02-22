# shiplog-cache-ttl

TTL (Time-To-Live) based cache implementation for shiplog.

## Overview

This crate provides an in-memory cache with time-based expiration. Each cache entry has a TTL after which it automatically expires and becomes unavailable.

## Features

- Time-based expiration with configurable TTL
- Custom TTL per entry
- Automatic cleanup of expired entries
- Simple and lightweight implementation

## Usage

```rust
use shiplog_cache_ttl::TtlCache;

// Create cache with 1 hour default TTL
let mut cache = TtlCache::new(3600);

// Insert items (uses default TTL)
cache.put("key", "value");

// Insert with custom TTL (in seconds)
cache.put_with_ttl("short_lived", "data", 60);

// Check if key exists and hasn't expired
if let Some(value) = cache.get(&"key") {
    println!("Got: {}", value);
}

// Clean up expired entries
cache.cleanup_expired();
```

## License

MIT OR Apache-2.0
