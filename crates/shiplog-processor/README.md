# shiplog-processor

Processor utilities for shiplog.

[![Rust Version](https://img.shields.io/badge/rust-1.92%2B-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

## Usage

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
shiplog-processor = "0.2.1"
```

## Features

- Simple processor for transforming input to output
- Batch processor for processing items in batches
- Stateful processor that maintains state between processing
- Pipeline for chaining multiple processors together
