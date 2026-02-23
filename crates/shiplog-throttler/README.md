# shiplog-throttler

Rate limiting and throttling utilities for shiplog.

## Overview

This crate provides throttling implementations for controlling the rate of operations.

## Features

- Configurable rate limiting with time windows
- Per-key throttling support
- Window reset functionality
- Strict and lenient preset configurations

## Usage

```rust
use shiplog_throttler::{Throttler, ThrottlerConfig};
use chrono::Duration;

// Create a throttler that allows 10 requests per minute
let config = ThrottlerConfig::new(10, Duration::minutes(1));
let mut throttler = Throttler::new(config);

// Try to acquire a token
if throttler.try_acquire("user123") {
    // Request allowed
} else {
    // Rate limited
}
```

## License

MIT OR Apache-2.0
