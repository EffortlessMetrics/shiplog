# shiplog-sync

Synchronization utilities for remote sources.

## Overview

This crate provides functionality for synchronizing data from remote sources.

## Usage

```rust
use shiplog_sync::{SyncConfig, SyncState, SyncResult};

let config = SyncConfig {
    source: "https://api.example.com".to_string(),
    interval_seconds: 3600,
    retry_count: 3,
    last_sync: None,
};

let mut state = SyncState::new(config);
state.start();
// ... sync logic ...
state.complete(100);

let result = SyncResult::success(100);
```
