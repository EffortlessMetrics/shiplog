# shiplog-circuitbreaker

Circuit breaker pattern implementation for shiplog.

## Overview

This crate provides a circuit breaker implementation for handling fault tolerance and preventing cascade failures.

## Features

- Configurable failure and success thresholds
- Automatic state transitions (Closed -> Open -> HalfOpen -> Closed)
- Per-circuit state management
- Strict and lenient preset configurations

## Usage

```rust
use shiplog_circuitbreaker::{CircuitBreaker, CircuitBreakerConfig};
use chrono::Duration;

// Create a circuit breaker
let config = CircuitBreakerConfig::strict();
let mut breaker = CircuitBreaker::new(config);

// Check if circuit allows requests
if breaker.is_available("service1") {
    // Make the request
    // ...
    breaker.record_success("service1");
} else {
    // Circuit is open, reject request
}
```

## License

MIT OR Apache-2.0
