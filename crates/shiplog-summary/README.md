# shiplog-summary

Summary statistics utilities for shiplog.

## Overview

This crate provides summary statistics functionality:

- [`Summary`] - Running summary with min, max, mean, variance, std_dev
- [`RunningStats`] - Online statistics calculator
- [`DescriptiveStats`] - Full descriptive statistics
- [`mean`], [`variance`], [`std_dev`] - Standalone functions

## Usage

```rust
use shiplog_summary::{Summary, RunningStats, DescriptiveStats, mean, variance};

// Using Summary
let mut s = Summary::new();
s.add(1.0);
s.add(2.0);
s.add(3.0);

println!("Count: {}", s.count());
println!("Sum: {}", s.sum());
println!("Mean: {}", s.mean());
println!("Min: {}", s.min());
println!("Max: {}", s.max());
println!("Std Dev: {}", s.std_dev());

// Using RunningStats
let mut rs = RunningStats::new();
rs.push(10.0);
rs.push(20.0);
rs.push(30.0);
println!("Mean: {}", rs.mean());
println!("Std Dev: {}", rs.std_dev());

// Using DescriptiveStats
let mut ds = DescriptiveStats::new();
ds.extend(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
println!("Median: {}", ds.median());
println!("Variance: {}", ds.variance());

// Standalone functions
let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
println!("Mean: {}", mean(&values));
println!("Variance: {}", variance(&values));
```

## Features

- Running statistics with Welford's algorithm
- Full descriptive statistics
- Sample and population variance/std_dev
- Median calculation
- Serialization support

## License

MIT OR Apache-2.0
