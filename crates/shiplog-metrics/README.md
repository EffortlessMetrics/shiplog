# shiplog-metrics

Metrics collection and reporting for shiplog.

## Overview

This crate provides metrics utilities for the shiplog project, including:
- Counter, Gauge, and Histogram metric types
- Metrics collector with statistics
- Metrics reporting with JSON export

## Usage

```rust
use shiplog_metrics::{MetricsCollector, MetricsReport};

// Create a metrics collector
let mut collector = MetricsCollector::new();

// Record metrics
collector.inc_counter("requests_total", 1.0);
collector.set_gauge("active_connections", 42.0);
collector.record_histogram("request_duration_ms", 150.0);

// Get statistics
if let Some(stats) = collector.histogram_stats("request_duration_ms") {
    println!("Mean: {}ms", stats.mean);
}

// Export and report
let report = MetricsReport::new(collector.export());
println!("{}", report.to_json());
```

## License

MIT OR Apache-2.0
