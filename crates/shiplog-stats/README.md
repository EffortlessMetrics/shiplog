# shiplog-stats

Statistics and analytics on events/workstreams.

## Overview

This crate provides functionality for computing statistics on shipping packets and workstream data.

## Usage

```rust
use shiplog_stats::{StatsAnalyzer, StatsSummary};

let mut analyzer = StatsAnalyzer::new();
analyzer.record_event("commit", "github");
analyzer.record_event("pr", "github");

let summary: StatsSummary = analyzer.generate_summary();
```
