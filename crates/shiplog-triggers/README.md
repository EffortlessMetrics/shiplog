# shiplog-triggers

Trigger utilities for stream processing in shiplog.

[![Rust Version](https://img.shields.io/badge/rust-1.92%2B-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

## Usage

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
shiplog-triggers = "0.2.1"
```

## Features

- `Trigger` interface
 trait - Core trigger- `CountTrigger` - Fires after a fixed number of elements
- `TimeTrigger` - Fires at fixed time intervals
- `WatermarkTrigger` - Fires when watermark passes a threshold
- `RepeatedTimeTrigger` - Fires repeatedly at fixed intervals after an initial delay
- `NeverTrigger` - Never fires
- `AlwaysTrigger` - Always fires
- `AndTrigger` - Combines triggers with AND logic
- `OrTrigger` - Combines triggers with OR logic
