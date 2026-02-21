# shiplog-sink

Sink utilities for streaming data in shiplog.

[![Rust Version](https://img.shields.io/badge/rust-1.92%2B-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

## Usage

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
shiplog-sink = "0.2.1"
```

## Features

- BufferSink: Collect items into a bounded buffer with optional eviction
- TransformSink: Transform items as they pass through
- FilterSink: Filter items based on a predicate
