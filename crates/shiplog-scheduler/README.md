# shiplog-scheduler

Task scheduling utilities for shiplog.

## Overview

Provides scheduling primitives for recurring tasks like nightly sync jobs, weekly reports, and periodic backups.

## Features

- Multiple schedule frequencies: daily, weekly, interval-based
- Task tracking with last run / next run metadata
- Simple scheduler for managing multiple tasks

## Usage

```rust
use shiplog_scheduler::{ScheduledTask, ScheduleFrequency, Scheduler};

// Create a daily task
let task = ScheduledTask::new(
    "nightly-sync",
    "Nightly Git Sync",
    ScheduleFrequency::Daily { hour: 2, minute: 0 },
);

// Add mut scheduler = Scheduler to scheduler
let::new();
scheduler.add_task(task);

// Check for due tasks
for task in scheduler.get_due_tasks() {
    println!("Running task: {}", task.name);
}
```

## License

MIT OR Apache-2.0
