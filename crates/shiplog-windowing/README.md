# shiplog-windowing

Windowing utilities for stream processing in shiplog.

[![Rust Version](https://img.shields.io/badge/rust-1.92%2B-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

## Usage

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
shiplog-windowing = "0.2.1"
```

## Features

- `Window` - Core window type for temporal grouping
- `TumblingWindow` - Non-overlapping, consecutive windows
- `SlidingWindow` - Overlapping windows of fixed size
- `SessionWindow` - Windows grouped by gaps in timestamps
- `WindowAssigner` trait - For custom window assignment strategies
- `TumblingWindowAssigner` - Fixed tumbling window assigner
- `SlidingWindowAssigner` - Sliding window assigner
