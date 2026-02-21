# shiplog-retry

Retry/backoff utilities for shiplog operations.

## Overview

This crate provides retry logic with configurable backoff strategies for handling transient failures in network and I/O operations.

## Usage

```rust
use shiplog_retry::{Retry, RetryConfig, RetryStrategy};

fn main() {
    let config = Retry::default_config();
    
    let result = Retry::execute(&config, || {
        // Your operation that might fail
        Ok::<_, ()>("success".to_string())
    });
    
    if result.success {
        println!("Operation succeeded after {} attempts", result.attempts);
    }
}
```

## Features

- **Multiple Retry Strategies**: Fixed, Exponential, and Linear backoff
- **Configurable Delays**: Initial delay, max delay, and multiplier settings
- **Sync and Async Support**: Both synchronous and asynchronous retry operations
- **Default Configuration**: Sensible defaults for quick adoption

## Example: Exponential Backoff

```rust
use shiplog_retry::{Retry, RetryConfig, RetryStrategy};

let config = RetryConfig {
    max_retries: 3,
    initial_delay_ms: 100,
    max_delay_ms: 30000,
    strategy: RetryStrategy::Exponential,
    multiplier: 2.0,
};

let result = Retry::execute(&config, || {
    // Operation with potential failures
    Ok(())
});
```

## License

MIT OR Apache-2.0
