# shiplog-writer

Writer utilities for shiplog.

[![Rust Version](https://img.shields.io/badge/rust-1.92%2B-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

## Usage

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
shiplog-writer = "0.2.1"
```

## Features

- BufferedWriter: A buffered writer that batches writes for efficiency
- LineWriter: A writer that writes data line by line with newline characters
- CountingWriter: A wrapper that tracks bytes written
