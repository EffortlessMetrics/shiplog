# shiplog-percentile

Percentile calculation utilities for shiplog.

## Overview

This crate provides percentile calculation functionality:

- [`Percentile`] - Single percentile estimator
- [`PercentileSet`] - Multiple percentile calculator
- [`StreamingPercentile`] - Memory-efficient streaming percentile
- [`calculate_percentile`] - Standalone percentile calculation

## Usage

```rust
use shiplog_percentile::{Percentile, PercentileSet, calculate_percentile};

// Single percentile
let mut p = Percentile::new("p99", 0.99);
for i in 1..=1000 {
    p.record(i as f64);
}
println!("P99: {}", p.get());

// Percentile set with common percentiles
let mut ps = PercentileSet::new("latency");
ps.record_many(vec![1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0]);
println!("Median: {}", ps.median());
println!("P95: {}", ps.p95());
println!("P99: {}", ps.p99());

// Standalone calculation
let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
let p50 = calculate_percentile(&values, 50.0);
```

## Features

- Single and multi-percentile calculations
- Streaming percentile with fixed memory
- Common percentiles (50, 90, 95, 99)
- Custom percentile support

## License

MIT OR Apache-2.0
