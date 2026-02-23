# shiplog-histogram

Histogram utilities for shiplog.

## Overview

This crate provides histogram functionality for metrics collection:

- [`Histogram`] - Fixed bucket histogram (exponential buckets)
- [`LinearHistogram`] - Linear histogram with evenly spaced buckets

## Usage

```rust
use shiplog_histogram::{Histogram, LinearHistogram};

// Fixed bucket histogram
let mut hist = Histogram::new("request_latency");
hist.record(0.005);
hist.record(0.025);
hist.record(0.1);

println!("Count: {}", hist.count());
println!("Mean: {}", hist.mean());
println!("Min: {}", hist.min());
println!("Max: {}", hist.max());

// Linear histogram
let mut linear = LinearHistogram::new("response_size", 0.0, 1000.0, 10);
linear.record(100.0);
linear.record(500.0);
```

## Features

- Fixed exponential bucket histograms
- Linear histograms with custom ranges
- Bucket-based data collection
- Cumulative count queries
- Mean, min, max calculations

## License

MIT OR Apache-2.0
