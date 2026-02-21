# shiplog-team-aggregate

Team aggregation microcrate extracted from `shiplog-team`.

This crate exposes deterministic team aggregation primitives:
- `TeamConfig` resolution contracts remain in `shiplog-team-core`.
- `TeamAggregator` for loading and merging member ledgers.
- `TeamAggregateResult` for run summaries and event coverage.
- `write_team_outputs` for emitting packet/event/coverage artifacts.

