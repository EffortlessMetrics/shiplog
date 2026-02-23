# shiplog-tracker

Issue tracking utilities for shiplog.

[![Crates.io](https://img.shields.io/crates/v/shiplog-tracker)](https://crates.io/crates/shiplog-tracker)
[![Docs.rs](https://docs.rs/shiplog-tracker/badge.svg)](https://docs.rs/shiplog-tracker)

## Usage

```rust
use shiplog_tracker::{TrackerItem, IssueStatus, Priority};

let item = TrackerItem::new("PROJ-123", "Fix bug", "github")
    .with_status(IssueStatus::InProgress)
    .with_priority(Priority::High);
```

## Features

- `TrackerItem` - Represents a single issue/tracker item
- `TrackerCollection` - Collection of items with filtering utilities
- `IssueStatus` - Common issue statuses (Open, Closed, Merged, etc.)
- `Priority` - Priority levels (Critical, High, Medium, Low, None)
