# shiplog-team-aggregate

Team aggregation microcrate extracted from `shiplog-team`.

This crate exposes deterministic team aggregation primitives:
- `TeamConfig` resolution contracts remain in `shiplog-team-core`.
- `TeamAggregator` for loading and merging member ledgers.
- `TeamAggregateResult` and rendering logic now live in `shiplog-team-render`
  and are re-exported here for compatibility.
- `write_team_outputs` for emitting packet/event/coverage artifacts.
