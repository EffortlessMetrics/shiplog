# shiplog-team

Team aggregation mode for generating team-level shipping summaries from multiple member ledgers.

`shiplog-team` is currently the stable public façade. The runtime aggregation
implementation has been extracted to [`shiplog-team-aggregate`](../shiplog-team-aggregate)
to keep a clearer domain boundary and make API surfaces easier to extend.

## Overview

This crate provides functionality to aggregate multiple team members' shiplog ledgers into a single team-level shipping summary packet.

## Features
- Stable config and resolver contracts are exposed through `shiplog-team-core`:
  - `TeamConfig`, `parse_csv_list`, `parse_alias_list`, and `resolve_team_config`
  - keeps `shiplog-team` focused on aggregation, rendering, and output writes

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
    since: None,
    until: None,
};

let aggregator = TeamAggregator::new(config);
// Load member ledgers and aggregate...
```

## CLI Integration

This crate is designed to be integrated with the main shiplog CLI to provide the `team-aggregate` subcommand.
