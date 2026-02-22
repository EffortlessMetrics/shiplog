# shiplog-rate

Rate limiting utilities for shiplog.

## Overview

This crate provides rate limiting utilities including:
- `TokenBucket` - Token bucket rate limiter
- `SlidingWindow` - Sliding window rate limiter
- `RateLimitConfig` - Configuration for rate limiters

## Usage

```rust
use shiplog_rate::{TokenBucket, RateLimitConfig};

let config = RateLimitConfig::per_second(10);
let mut limiter = TokenBucket::new(config);

if limiter.try_consume("user1") {
    // Process request
} else {
    // Rate limited
}
```

## Features

- Token bucket algorithm
- Sliding window algorithm
- Per-key rate limiting
- Pre-built configurations for common APIs (GitHub, etc.)
