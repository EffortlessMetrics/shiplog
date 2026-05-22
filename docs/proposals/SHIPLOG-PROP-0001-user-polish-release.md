# SHIPLOG-PROP-0001: User-Polish Release

Status: proposed
Owner: product/docs
Created: 2026-05-13
Target release: 0.6.0

## Summary

Shiplog's user-polish lane should make the first-run review-pack experience
executable and receipt-backed. The goal is not more documentation. The goal is
that a first-time user can install shiplog, run one intake command, open the
latest report and packet, understand what worked and what needs repair, and
share the output safely.

The matching repository goal is that a future Codex, Droid, or Claude thread
can read one active goal manifest, follow linked proposal/spec/ADR/plan/issues,
run the right proof commands, and implement the next PR without rediscovering
intent from chat history.

## Problem

Shiplog already has the core review-rescue path:

```bash
shiplog intake --last-6-months --explain
shiplog open intake-report --latest
shiplog open packet --latest
```

The remaining friction is around confidence and continuation. A user should not
have to infer:

- where output went;
- which sources worked;
- which sources were skipped or unavailable;
- whether evidence is fresh or cached;
- whether an artifact is safe to share;
- what to fix next;
- what command to run after the first report.

Agents face a parallel problem. The intent behind first-run polish currently
lives across product docs, guides, schema docs, issues, prior PRs, and chat
history. That makes the lane easy to restart incorrectly: specs can turn into
guides, plans can become behavior contracts, and deferred design choices can be
rediscovered as if they were open questions.

## Target Users

Primary users:

- a first-time shiplog user preparing a self-review, promo packet, brag doc, or
  manager-facing "what shipped?" summary from an empty directory;
- a deadline-pressure user who needs an honest first artifact before they have
  time to curate every source;
- a reviewer or manager reading `intake.report.md` to understand readiness,
  gaps, source state, and share posture.

Repository users:

- Codex, Droid, Claude, or a maintainer implementing the next PR in this lane;
- reviewers checking that report JSON, Markdown, cache semantics, redaction,
  and CLI guidance still match the intended contract.

## Product End State

The lane is done when shiplog can support this first-run path without hidden
interpretation:

```bash
shiplog intake --last-6-months --explain
shiplog open intake-report --latest
shiplog open packet --latest
```

Required product outcomes:

- `shiplog intake --last-6-months --explain` produces a defensible review pack
  from an empty directory.
- The CLI prints concise next-step commands after intake, including
  `shiplog open intake-report --latest` and `shiplog open packet --latest`.
- `shiplog open ... --latest` finds the newest run deterministically or gives a
  useful command to create one.
- `intake.report.md` is readable by a reviewer under time pressure.
- `intake.report.json` is stable and machine-joinable: source identities are
  canonical, freshness is explicit, skipped and unavailable sources have
  reasons, and secret-bearing vocabulary is rejected at the schema layer.
- Freshness remains honest: `fresh`, `cached`, `skipped`, `unavailable`, and
  `stale` are emitted only when the underlying receipt proves that state.
  `stale` specifically requires `CacheLookup::Stale(_)`.
- Sharing remains fail-closed: redaction posture is visible, source opaque IDs
  do not leak across non-internal profiles, and share verification stays
  receipt-backed.

## Repository End State

The source-of-truth stack is used as the control plane between product intent
and agent execution:

```text
docs/proposals/       why / PRD / alternatives / success criteria
docs/specs/           behavior contracts and required proof
docs/adr/             durable architecture decisions
plans/user-polish/    PR-sized sequencing, proof commands, rollback
.shiplog/goals/       current machine-readable agent state
policy/*.toml         proof, exceptions, and enforcement posture
contracts/schemas/    machine-readable artifact contracts
tests                 executable proof
docs/guides/          user how-to
```

Specs define contracts. Plans define PR order. Guides teach users. ADRs record
durable architecture decisions. Policy ledgers, schemas, tests, and CI docs
remain proof and exception surfaces, not duplicated prose.

## Success Criteria

This lane succeeds when the following are true:

- a clean install can reach a first defensible pack with one intake command and
  two open commands;
- the intake terminal footer and `intake.report.md` agree on the next useful
  commands;
- latest-artifact opening is deterministic and Windows-safe;
- report JSON consumers can join source decisions and freshness without
  display-label normalization;
- skipped and unavailable sources carry reasons in both human and machine
  receipts;
- schema validation rejects secret-bearing report field names before values
  exist;
- stale freshness is emitted only when a cache lookup proves stale rows;
- manager/public sharing fails closed without `SHIPLOG_REDACT_KEY` or an
  explicit redaction key;
- the active goal manifest links the current work item to proposal, specs,
  ADRs, plans, issues or PRs, and proof commands;
- each PR in the lane carries scope, expected files, behavior change,
  advisory/blocking status, validation, rollback, and follow-up notes.

## Non-Goals

This proposal does not reopen:

- the protected-fields or `clippy::disallowed_fields` lane;
- source-opaque-ID audits already closed for the current code;
- a broad platform-wide spec system beyond the user-polish lane;
- generated impact narratives or reviewer-ready prose authored by shiplog;
- team aggregation or cross-user rollups;
- hard CI budget enforcement beyond the existing policy-ledger doctrine.

This proposal also does not implement CLI, cache, report, redaction, or
open-latest behavior. Those changes belong after the spec stack and ADRs land.

## Release Target

Target the full lane at `0.6.0`.

Rationale:

- the lane adds user-visible CLI behavior;
- latest-artifact discovery changes the first-run workflow;
- source identity cleanup changes the machine-readable report contract;
- stale/cache work introduces new cache lookup semantics;
- the release thesis is broader than a patch fix.

`0.5.1` is appropriate only for narrow bug fixes or docs-only corrections that
do not change the report contract or first-run behavior. The spec stack can
land before the release cut, but the product lane as a whole should not be
described as a patch release.

## Proposed Artifact Stack

The lane should land in this order:

1. Source-of-truth scaffold: [`docs/proposals/README.md`](README.md),
   [`docs/specs/README.md`](../specs/README.md),
   [`docs/adr/README.md`](../adr/README.md),
   [`plans/user-polish/README.md`](../../plans/user-polish/README.md),
   [`.shiplog/goals/README.md`](../../.shiplog/goals/README.md).
2. This proposal:
   `docs/proposals/SHIPLOG-PROP-0001-user-polish-release.md`.
3. Rapid first-intake spec:
   `docs/specs/SHIPLOG-SPEC-0001-rapid-first-intake.md`.
4. Intake report and source identity specs:
   `docs/specs/SHIPLOG-SPEC-0002-intake-report-v1.md` and
   `docs/specs/SHIPLOG-SPEC-0003-source-identity.md`.
5. ADRs:
   `docs/adr/SHIPLOG-ADR-0001-ingest-output-is-receipt-boundary.md`,
   `docs/adr/SHIPLOG-ADR-0002-machine-source-keys-vs-display-labels.md`,
   `docs/adr/SHIPLOG-ADR-0003-stale-requires-cachelookup.md`.
6. Implementation plan and active goal:
   `plans/user-polish/implementation-plan.md` and
   `.shiplog/goals/active.toml`.

Implementation PRs follow only after the spec stack lands.

## Linked Issues And PRs

- [PR #230](https://github.com/EffortlessMetrics/shiplog/pull/230):
  source-of-truth scaffold.
- [Issue #197](https://github.com/EffortlessMetrics/shiplog/issues/197):
  rapid first-intake product lane umbrella.
- [Issue #199](https://github.com/EffortlessMetrics/shiplog/issues/199):
  fixture-backed cold-start integration test.
- [Issue #201](https://github.com/EffortlessMetrics/shiplog/issues/201):
  cold-start defaults and source-selection probes.
- [Issue #203](https://github.com/EffortlessMetrics/shiplog/issues/203):
  render polish for executive summary, redaction status, and
  where-to-look sections.
- [Issue #223](https://github.com/EffortlessMetrics/shiplog/issues/223):
  inconsistent source naming across intake report sections.
- [Issue #224](https://github.com/EffortlessMetrics/shiplog/issues/224):
  `CacheLookup::{Fresh, Stale, Miss}` for honest stale reporting.
- [Issue #225](https://github.com/EffortlessMetrics/shiplog/issues/225):
  recorded HTTP fixtures for full CLI freshness integration tests;
  implemented by [#245](https://github.com/EffortlessMetrics/shiplog/pull/245).
- [Issue #229](https://github.com/EffortlessMetrics/shiplog/issues/229):
  skipped sources in `source_freshness`.

## Proof Surfaces

Existing proof surfaces to link from future specs:

- [`apps/shiplog/tests/intake_cold_start.rs`](../../apps/shiplog/tests/intake_cold_start.rs)
- [`apps/shiplog/tests/front_door_first_pack_smoke.rs`](../../apps/shiplog/tests/front_door_first_pack_smoke.rs)
- [`docs/product/rapid-first-intake.md`](../product/rapid-first-intake.md)
- [`docs/guides/rapid-first-intake.md`](../guides/rapid-first-intake.md)
- [`docs/schemas/intake-report-v1.md`](../schemas/intake-report-v1.md)
- [`contracts/schemas/intake-report.v1.schema.json`](../../contracts/schemas/intake-report.v1.schema.json)
- [`docs/CLIPPY_PROTECTED_FIELDS.md`](../CLIPPY_PROTECTED_FIELDS.md)
- [`policy/clippy-protected-fields.toml`](../../policy/clippy-protected-fields.toml)
- [`docs/ci/cost-and-verification-policy.md`](../ci/cost-and-verification-policy.md)
- [`docs/ci/skipped-by-policy.md`](../ci/skipped-by-policy.md)

Docs-only PR validation:

```bash
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Behavior PRs add targeted tests from the owning spec and, when practical,
broader workspace checks.

## Alternatives Considered

### Put everything in the guide

Rejected. Guides teach users what to do; they are not stable contracts for
report JSON, cache semantics, source identity, or agent sequencing.

### Put the full PR queue in specs

Rejected. Specs define what must be true. PR order, rollback, and validation
belong in `plans/user-polish/`.

### Implement skipped-source freshness before source identity

Rejected for sequencing. Skipped freshness depends on stable source identity;
otherwise tests need local display-label normalization and the report contract
keeps baking in naming drift.

### Emit `stale` from existing cache behavior

Rejected. At proposal time, `ApiCache::get` could not honestly distinguish
stale-hit from miss. `FreshnessStatus::Stale` became valid only after
`ApiCache::lookup` exposed `CacheLookup::{Fresh, Stale, Miss}`.

### Treat this as a patch release

Rejected for the full lane. Some docs-only or bug-fix PRs could ship in a
patch, but the lane changes the first-run workflow and machine-readable report
semantics. `0.6.0` is the honest release target.

## Exit Criteria

The lane can close when:

- the proposal, specs, ADRs, implementation plan, and active goal manifest have
  landed and link to each other;
- CLI next-step footer behavior is implemented and tested;
- latest artifact opening is implemented and tested;
- report source identities are canonical and schema-validated;
- skipped source freshness is implemented after source identity cleanup;
- stale reporting is unlocked only through `CacheLookup`;
- recorded fixtures prove warm rerun freshness without live network;
- release notes separate user-facing improvements from internal proof work;
- docs, schemas, policy ledgers, and tests agree on the contract.

North star: shiplog's product receipts and development receipts should have the
same shape: claim, source, freshness, proof, gaps, and next action.
