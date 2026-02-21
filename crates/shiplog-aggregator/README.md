# shiplog-aggregator

Aggregation utilities for shiplog.

[![Rust Version](https://img.shields.io/badge/rust-1.92%2B-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

## Usage

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
shiplog-aggregator = "0.2.1"
```

## Features

- Generic aggregator with sum, min, max, average operations
- Keyed aggregator for grouping and aggregating by key
