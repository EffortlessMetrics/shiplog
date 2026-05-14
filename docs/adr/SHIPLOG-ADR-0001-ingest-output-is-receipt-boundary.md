# SHIPLOG-ADR-0001: IngestOutput Is The Receipt Boundary

Status: accepted
Date: 2026-05-13
Related proposal:
[`SHIPLOG-PROP-0001-user-polish-release`](../proposals/SHIPLOG-PROP-0001-user-polish-release.md)
Related specs:
[`SHIPLOG-SPEC-0001-rapid-first-intake`](../specs/SHIPLOG-SPEC-0001-rapid-first-intake.md),
[`SHIPLOG-SPEC-0002-intake-report-v1`](../specs/SHIPLOG-SPEC-0002-intake-report-v1.md)

## Context

Shiplog intake is becoming a receipt compiler for professional evidence. The
report layer must tell users and agents which source contributed evidence,
whether it was fresh or cached, why it was skipped or unavailable, and what to
repair next.

Freshness and cache attribution cannot be inferred safely from a global CLI
cache snapshot. A cache may serve several providers, several calls, and several
source contributions during one intake run. Only the adapter knows whether a
given source contribution came from live input, valid cache, or no usable data.

The schema crate already records this boundary in
[`SourceFreshness`](../../apps/shiplog/src/schema/freshness.rs):
adapter receipts are populated locally, then the CLI/report layer merges those
receipts with skipped or unavailable source decisions for
`intake.report.{md,json}`.

## Decision

`IngestOutput` is the immutable adapter receipt boundary for intake.

Adapters own source-local freshness and cache attribution. Each adapter emits
the `SourceFreshness` receipts it can prove for the source contributions it
handled.

The CLI/report layer may:

- render adapter receipts;
- merge adapter receipts with configured-source failures;
- add skipped or unavailable entries when it has explicit source-decision
  evidence;
- validate and serialize the final report.

The CLI/report layer must not infer source freshness from global cache state,
wall-clock timing, or command-level pre/post cache snapshots.

## Consequences

- `IngestOutput.freshness` remains the adapter-to-CLI receipt handoff.
- A source can be marked `fresh` or `cached` only when an adapter reports that
  state for that source contribution.
- A source can be marked `unavailable` when a configured source failed and the
  report layer has the failure reason.
- A source can be marked `skipped` only from explicit source-decision evidence,
  not from absence.
- Future adapters that aggregate multiple source contributions must emit one
  freshness receipt per logical source contribution.
- Tests should assert receipt flow through `IngestOutput` rather than recreating
  cache attribution in the CLI.

## Alternatives Considered

### Infer Freshness In The CLI

Rejected. CLI-level cache snapshots are not source-attributed. They can prove
that some cache entry changed, but not which source contribution should receive
the freshness label.

### Let The Report Layer Normalize Missing Receipts

Rejected as the default. Absence of a receipt is not a receipt. The report layer
may create skipped or unavailable entries only when another explicit source
decision or failure proves that state.

### Make The Cache Layer The Report Authority

Rejected. The cache can know whether a row is present or valid, but it does not
know whether an adapter used the row as evidence, fetched fresh data afterward,
or discarded the value.

## Affected Specs, Plans, Tests, And Schemas

- [`SHIPLOG-SPEC-0001-rapid-first-intake`](../specs/SHIPLOG-SPEC-0001-rapid-first-intake.md)
  requires honest readiness and source state in the first-run report.
- [`SHIPLOG-SPEC-0002-intake-report-v1`](../specs/SHIPLOG-SPEC-0002-intake-report-v1.md)
  defines `intake.report.json` and `intake.report.md` as the machine and human
  report surfaces.
- [`SourceFreshness`](../../apps/shiplog/src/schema/freshness.rs) is the
  source-local freshness receipt structure.
- [`apps/shiplog/tests/intake_cold_start.rs`](../../apps/shiplog/tests/intake_cold_start.rs)
  and
  [`apps/shiplog/tests/front_door_first_pack_smoke.rs`](../../apps/shiplog/tests/front_door_first_pack_smoke.rs)
  prove current first-run report behavior.
- The user-polish implementation plan should sequence behavior changes so
  receipt ownership remains adapter-local.
