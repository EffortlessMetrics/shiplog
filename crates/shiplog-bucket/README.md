# shiplog-bucket

Token bucket implementation for shiplog.

## Overview

This crate provides a token bucket implementation for rate limiting.

## Features

- Configurable bucket capacity and refill rate
- Per-key token buckets
- Automatic token refill based on time
- Strict and lenient preset configurations

## Usage

```rust
use shiplog_bucket::{TokenBucket, TokenBucketConfig};
use chrono::Duration;

// Create a token bucket with capacity of 10, refilling 1 token per second
let config = TokenBucketConfig::new(10, 1, Duration::seconds(1));
let mut bucket = TokenBucket::new(config);

// Try to consume tokens
if bucket.try_consume("user123", 1) {
    // Token consumed successfully
} else {
    // No tokens available
}
```

## License

MIT OR Apache-2.0
