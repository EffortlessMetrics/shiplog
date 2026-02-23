# shiplog-uuid

UUID generation and utilities for shiplog.

[![Crates.io](https://img.shields.io/crates/v/shiplog-uuid)](https://crates.io/crates/shiplog-uuid)
[![Docs.rs](https://docs.rs/shiplog-uuid/badge.svg)](https://docs.rs/shiplog-uuid)

## Usage

```rust
use shiplog_uuid::{Uuid, generate_id};

let uuid = Uuid::new();
println!("Generated UUID: {}", uuid);

let id = generate_id("event");
println!("Generated ID: {}", id");
```

## Features

- `Uuid` - UUID wrapper type with utilities
- `UuidVersion` - UUID version detection
- `generate_id` - Simple timestamp-based ID generation
- `parse_uuid` - Parse UUID strings into components
