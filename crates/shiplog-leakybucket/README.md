# shiplog-leakybucket

Leaky bucket implementation for shiplog.

## Overview

This crate provides a leaky bucket implementation for rate limiting.

## Features

- Configurable bucket capacity and leak rate
- Per-key leaky buckets
- Automatic leaking based on time
- Strict and lenient preset configurations

## Usage

```rust
use shiplog_leakybucket::{LeakyBucket, LeakyBucketConfig};
use chrono::Duration;

// Create a leaky bucket with capacity of 10, leaking 1 item per second
let config = LeakyBucketConfig::new(10, 1, Duration::seconds(1));
let mut bucket = LeakyBucket::new(config);

// Try to add an item
if bucket.try_add("user123") {
    // Item added successfully
} else {
    // Bucket is full
}
```

## License

MIT OR Apache-2.0
