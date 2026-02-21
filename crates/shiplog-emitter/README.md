# shiplog-emitter

Event emitter pattern utilities for shiplog.

[![Rust Version](https://img.shields.io/badge/rust-1.92%2B-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

## Usage

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
shiplog-emitter = "0.2.1"
```

## Features

- Generic event emitter for publish-subscribe patterns
- Thread-safe shared emitter using Arc and RwLock
- Multiple event handling with `on`, `emit`, and `off` methods
