# shiplog-team

Team aggregation mode for generating team-level shipping summaries from multiple member ledgers.

## Overview

This crate provides functionality to aggregate multiple team members' shiplog ledgers into a single team-level shipping summary packet.

## Features

- Generate team-level packets from multiple member ledgers
- Configurable sections (workstreams, coverage, receipts)
- Member alias support for display names
- Custom template support
- Warning handling for missing or incompatible ledgers
- Version compatibility checking

## Usage

```rust
use shiplog_team::{TeamAggregator, TeamConfig};
use std::collections::HashMap;
use std::path::Path;

let mut aliases = HashMap::new();
aliases.insert("alice".to_string(), "Alice S.".to_string());

let config = TeamConfig {
    members: vec!["alice".to_string(), "bob".to_string()],
    aliases,
    sections: vec!["summary".to_string(), "workstreams".to_string()],
    template: None,
    date_range: None,
};

let aggregator = TeamAggregator::new(config);
// Load member ledgers and aggregate...
```

## CLI Integration

This crate is designed to be integrated with the main shiplog CLI to provide the `team-aggregate` subcommand.
