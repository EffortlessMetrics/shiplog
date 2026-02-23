# shiplog-meter

Metering utilities for shiplog.

## Overview

This crate provides utilities for measuring rates, throughput, and timing:

- [`MeterData`] - Track event counts and calculate rates
- [`MeterRegistry`] - Manage multiple meters
- [`TimingContext`] - Measure operation duration
- [`TimingRecorder`] - Collect and analyze timing data

## Usage

```rust
use shiplog_meter::{MeterRegistry, TimingRecorder};

// Track rates
let mut registry = MeterRegistry::new();
registry.record("api_requests");
registry.record_many("api_requests", 5);
let rate = registry.get("api_requests").unwrap().rate_per_second();

// Record timings
let mut recorder = TimingRecorder::new();
recorder.record("query", 0.1);
recorder.record("query", 0.2);
let stats = recorder.stats("query").unwrap();
```

## Features

- Rate calculation (per second, minute, hour)
- Timing statistics (mean, min, max, percentiles)
- Multiple meter management
- Serialization support

## License

MIT OR Apache-2.0
