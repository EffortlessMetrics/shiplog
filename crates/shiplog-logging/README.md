# shiplog-logging

Logging configuration and utilities for shiplog.

## Overview

This crate provides logging utilities for the shiplog project, including:
- Log levels (Error, Warn, Info, Debug, Trace)
- Logging configuration with component-specific levels
- Log entry and collector structures

## Usage

```rust
use shiplog_logging::{LogLevel, LogEntry, LoggingConfig};

// Create a logging config
let config = LoggingConfig::new()
    .with_level(LogLevel::Debug)
    .with_component_level("network", LogLevel::Trace);

// Check if we should log
if config.should_log(LogLevel::Info, None) {
    println!("Logging is enabled for Info level");
}

// Create log entries
let entry = LogEntry::new(LogLevel::Info, "Application started");
let entry_with_component = LogEntry::with_component(LogLevel::Debug, "engine", "Processing data");
```

## License

MIT OR Apache-2.0
