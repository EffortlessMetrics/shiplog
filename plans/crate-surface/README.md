# Crate-Surface Plan Lane

This directory owns implementation sequencing for the 0.7.0 crate-surface
contraction lane.

The lane is intentionally narrow: reduce shiplog's forward public crates.io
surface to intentional contracts while preserving the 0.6.0 first-run product
behavior.

Target public surface:

```text
shiplog
```

JSON schemas under `contracts/schemas/` are the public machine contract. Rust
schema types are internal unless a future ADR promotes a typed Rust API.

## What Belongs Here

Plans sequence PRs. They should include:

- dependency order;
- scoped PR titles;
- expected file surfaces;
- behavior-change posture;
- proof commands;
- rollback notes;
- follow-up and stop conditions.

Plans link to proposals, specs, ADRs, release audits, policy ledgers, and goal
manifests. They do not define behavior or support policy themselves.

## Current Plan

- [`implementation-plan.md`](./implementation-plan.md) sequences the remaining
  0.7 contraction work.
- [`.shiplog/goals/active.toml`](../../.shiplog/goals/active.toml) records the
  current agent-execution state for this lane.

