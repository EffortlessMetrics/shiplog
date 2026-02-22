# shiplog-delayqueue

Delayed queue implementation for shiplog.

[![Rust Version](https://img.shields.io/badge/rust-1.92%2B-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

## Usage

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
shiplog-delayqueue = "0.2.1"
```

## Features

- Binary heap-based delay queue for efficient deadline management
- Items are returned when their deadline has passed
- Support for item removal by ID
- Updateable delay queue for changing item deadlines
- Statistics tracking
