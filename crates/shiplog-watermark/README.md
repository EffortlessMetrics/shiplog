# shiplog-watermark

Watermark utilities for stream processing in shiplog.

[![Rust Version](https://img.shields.io/badge/rust-1.92%2B-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

## Usage

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
shiplog-watermark = "0.2.1"
```

## Features

- `Watermark` - Core watermark type for tracking event time progress
- `PeriodicWatermarkGenerator` - Generates watermarks with a fixed lag behind the maximum observed timestamp
- `TumblingWatermarkGenerator` - Generates watermarks at fixed time intervals
- `WatermarkTracker` - Maintains a history of watermarks for debugging and analysis
