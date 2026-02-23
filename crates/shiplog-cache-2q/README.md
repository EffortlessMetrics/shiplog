# shiplog-cache-2q

2Q (Two-Queue) cache implementation for shiplog.

## Overview

This crate provides a 2Q cache algorithm which uses two queues:
- A FIFO queue for first-time insertions (in queue)
- An LRU queue for frequently accessed items (out queue)

The 2Q algorithm is efficient for workloads with "temporal locality" where items accessed once are unlikely to be accessed again.

## Features

- Two-queue algorithm (2Q) for improved hit rates
- Automatic promotion of frequently accessed items
- FIFO eviction for new items, LRU eviction for frequent items
- Simple and lightweight implementation

## Usage

```rust
use shiplog_cache_2q::TwoQCache;

let mut cache = TwoQCache::new(100);

// Insert items
cache.put("key1", "value1");
cache.put("key2", "value2");

// Get an item (promotes to out-queue if accessed frequently)
let value = cache.get(&"key1");

// Cache automatically manages eviction between queues
cache.put("key3", "value3");
```

## Algorithm

1. New items go to the "in" queue (FIFO)
2. When an item is accessed, it's promoted to the "out" queue (LRU)
3. Eviction优先优先 from the in queue, then from the out queue
4. This separates "one-time" items from "frequently accessed" items

## License

MIT OR Apache-2.0
