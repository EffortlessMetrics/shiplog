# shiplog-quantile

Quantile estimation utilities for shiplog.

## Overview

This crate provides quantile estimation functionality:

- [`TDigest`] - TDigest-based quantile estimator
- [`ReservoirQuantile`] - Reservoir sampling quantile estimator
- [`GKQuantile`] - Greenwald-Khanna quantile estimator
- [`quantile`] - Standalone quantile calculation

## Usage

```rust
use shiplog_quantile::{TDigest, ReservoirQuantile, GKQuantile, quantile};

// TDigest quantile estimation
let mut td = TDigest::new();
for i in 1..=1000 {
    td.add(i as f64);
}
println!("Q50: {}", td.quantile(0.5));
println!("Q95: {}", td.quantile(0.95));
println!("Q99: {}", td.quantile(0.99));

// Reservoir quantile
let mut rq = ReservoirQuantile::new(0.5, 1000);
rq.add(10.0);
rq.add(20.0);
println!("Quantile: {}", rq.quantile());

// GK quantile
let mut gk = GKQuantile::new(0.05);
for i in 1..=100 {
    gk.add(i as f64);
}
println!("Q50: {}", gk.quantile(0.5));

// Standalone
let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
let q = quantile(&values, 0.5);
```

## Features

- TDigest algorithm for accurate quantile estimation
- Reservoir sampling for memory efficiency
- GK algorithm for space-efficient quantile estimation
- Weighted value support (TDigest)

## License

MIT OR Apache-2.0
