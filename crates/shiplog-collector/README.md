# shiplog-collector

Collector utilities for shiplog.

[![Rust Version](https://img.shields.io/badge/rust-1.92%2B-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

## Usage

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
shiplog-collector = "0.2.1"
```

## Features

- Batch collector for gathering items and flushing in batches
- Conditional collector that collects until a predicate is satisfied
