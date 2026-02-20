# shiplog-report

Report generation from workstream data.

## Overview

This crate provides functionality for generating reports from workstream data.

## Usage

```rust
use shiplog_report::{ReportGenerator, ReportFormat};
use chrono::Utc;

let mut generator = ReportGenerator::new();
generator.add_workstream("backend", 50, 85.0);
generator.add_workstream("frontend", 30, 90.0);

let report = generator.generate(
    "Weekly Report".to_string(),
    Utc::now(),
    Utc::now(),
    ReportFormat::Html,
);
```
