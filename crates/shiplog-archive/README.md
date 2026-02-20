# shiplog-archive

Archive/compress old packets.

## Overview

This crate provides functionality for archiving and compressing old shipping packets.

## Usage

```rust
use shiplog_archive::{ArchiveConfig, ArchiveFormat, ArchiveState};

let config = ArchiveConfig {
    format: ArchiveFormat::Zip,
    retention_days: 90,
    compression_level: Some(6),
};

let mut state = ArchiveState::new(config);
state.start();
// ... archive logic ...
```
