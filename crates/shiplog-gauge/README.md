# shiplog-gauge

Gauge utilities for shiplog.

## Overview

This crate provides specialized gauge functionality for tracking point-in-time values:

- [`Gauge`] - Basic gauge with min/max tracking
- [`ClampedGauge`] - Gauge that clamps values within bounds
- [`AverageGauge`] - Gauge that tracks running average
- [`GaugeRegistry`] - Manage multiple gauges

## Usage

```rust
use shiplog_gauge::{Gauge, ClampedGauge, AverageGauge, GaugeRegistry};

// Basic gauge
let mut gauge = Gauge::new("temperature");
gauge.set(72.5);
gauge.inc(0.5);
assert_eq!(gauge.value(), 73.0);

// Clamped gauge
let mut percent = ClampedGauge::new("cpu_percent", 0.0, 100.0);
percent.set(150.0); // clamped to 100.0

// Average gauge
let mut avg = AverageGauge::new("readings");
avg.record(10.0);
avg.record(20.0);
avg.record(30.0);
assert_eq!(avg.average(), 20.0);

// Registry
let mut registry = GaugeRegistry::new();
registry.set("temperature", 72.0);
registry.set("humidity", 65.0);
```

## Features

- Multiple gauge types (basic, clamped, average)
- Min/max tracking
- Gauge registry for management
- Snapshot support
- Serialization

## License

MIT OR Apache-2