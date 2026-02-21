# shiplog-circuit

Circuit breaker pattern implementation for shiplog.

## Overview

This crate provides a circuit breaker implementation for handling fault tolerance:
- `CircuitBreaker` - Main circuit breaker implementation
- `CircuitState` - State enum (Closed, Open, HalfOpen)
- `CircuitBreakerConfig` - Configuration for circuit breaker behavior

## Usage

```rust
use shiplog_circuit::{CircuitBreaker, CircuitBreakerConfig};

let config = CircuitBreakerConfig::strict();
let mut breaker = CircuitBreaker::new(config);

if breaker.is_available("api") {
    // Make request
    match result {
        Ok(_) => breaker.record_success("api"),
        Err(_) => breaker.record_failure("api"),
    }
} else {
    // Circuit is open, use fallback
}
```

## Features

- Three-state circuit breaker (Closed, Open, HalfOpen)
- Configurable failure and success thresholds
- Configurable timeout for automatic recovery
- Per-circuit state management
