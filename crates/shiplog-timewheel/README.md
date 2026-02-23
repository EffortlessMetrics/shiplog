# shiplog-timewheel

Time wheel scheduler for shiplog.

[![Rust Version](https://img.shields.io/badge/rust-1.92%2B-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

## Usage

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
shiplog-timewheel = "0.2.1"
```

## Features

- Hierarchical time wheel scheduler for efficient timer management
- Configurable tick duration and wheel size
- Delay queue for simple deadline-based scheduling
- Task cancellation support
- Statistics tracking
