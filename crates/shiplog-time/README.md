# shiplog-time

Time utilities and helpers for shiplog.

## Overview

This crate provides time-related utilities:
- `TimeRange` - A time range with start and end timestamps
- `DurationHelper` - Helper for creating durations
- `TimePeriod` - Common time periods (Today, ThisWeek, etc.)
- Formatting and parsing functions

## Usage

```rust
use shiplog_time::{TimeRange, TimePeriod, format_timestamp};

let now = chrono::Utc::now();
let range = TimeRange::new(
    now - chrono::Duration::days(7),
    now
);

// Use time period
if let Some(range) = TimePeriod::Last30Days.to_range() {
    println!("Last 30 days: {} to {}", range.start, range.end);
}

// Format timestamp
println!("{}", format_timestamp(now));
```

## Features

- TimeRange for representing time intervals
- TimePeriod enum for common periods (Today, ThisWeek, Last30Days, etc.)
- Duration helpers for easy creation
- ISO 8601 formatting and parsing
- Start/end of day helpers
- Weekday/weekend detection
- Relative time formatting
