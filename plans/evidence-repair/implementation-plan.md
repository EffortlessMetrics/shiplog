# Evidence Repair Loop Implementation Plan

Status: active
Owner: product/cli
Created: 2026-05-14

Related proposal:
[`SHIPLOG-PROP-0002-evidence-repair-loop`](../../docs/proposals/SHIPLOG-PROP-0002-evidence-repair-loop.md)
Related spec:
[`SHIPLOG-SPEC-0005-evidence-repair-loop`](../../docs/specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md)
Related ADR:
[`SHIPLOG-ADR-0006-repair-actions-are-receipt-derived`](../../docs/adr/SHIPLOG-ADR-0006-repair-actions-are-receipt-derived.md)
Active goal:
[`active.toml`](../../.shiplog/goals/active.toml)

## Purpose

This plan sequences the Evidence Repair Loop after the 0.7.0 crate-surface cut.
The lane should make Shiplog better by turning trustworthy first-run report
receipts into safe, specific repair actions.

Target user path:

```bash
shiplog intake --last-6-months --explain
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog open packet --latest
```

## Source Of Truth

Proposal:

- [`SHIPLOG-PROP-0002-evidence-repair-loop`](../../docs/proposals/SHIPLOG-PROP-0002-evidence-repair-loop.md)

Specs:

- [`SHIPLOG-SPEC-0002-intake-report-v1`](../../docs/specs/SHIPLOG-SPEC-0002-intake-report-v1.md)
- [`SHIPLOG-SPEC-0003-source-identity`](../../docs/specs/SHIPLOG-SPEC-0003-source-identity.md)
- [`SHIPLOG-SPEC-0005-evidence-repair-loop`](../../docs/specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md)

ADRs:

- [`SHIPLOG-ADR-0001-ingest-output-is-receipt-boundary`](../../docs/adr/SHIPLOG-ADR-0001-ingest-output-is-receipt-boundary.md)
- [`SHIPLOG-ADR-0002-machine-source-keys-vs-display-labels`](../../docs/adr/SHIPLOG-ADR-0002-machine-source-keys-vs-display-labels.md)
- [`SHIPLOG-ADR-0003-stale-requires-cachelookup`](../../docs/adr/SHIPLOG-ADR-0003-stale-requires-cachelookup.md)
- [`SHIPLOG-ADR-0006-repair-actions-are-receipt-derived`](../../docs/adr/SHIPLOG-ADR-0006-repair-actions-are-receipt-derived.md)

Release checkpoint:

- [`v0.7.0`](https://github.com/EffortlessMetrics/shiplog/releases/tag/v0.7.0)
  shipped the crate-surface contraction and made `shiplog` the intentional
  public package surface.

Landed setup PRs:

- [#292](https://github.com/EffortlessMetrics/shiplog/pull/292):
  archived the crate-surface lane and started this quality lane.
- [#293](https://github.com/EffortlessMetrics/shiplog/pull/293):
  added the Evidence Repair Loop spec.
- [#294](https://github.com/EffortlessMetrics/shiplog/pull/294):
  added the receipt-derived repair-actions ADR.

## Operating Rules

- Repair actions are derived from `intake.report.json`; do not re-query
  providers or rediscover source health.
- Do not scrape `intake.report.md` for machine behavior.
- Keep report JSON compatibility explicit. Optional fields require schema docs;
  required fields require compatibility notes and may require a schema version
  change.
- Do not print secret values, private provider IDs, or destructive commands in
  repair actions.
- Do not mutate GitHub, GitLab, Jira, Linear, or other providers in this lane.
- Keep each PR reviewable: one behavior surface, its schema/docs, and its tests.
- Preserve the existing first-run path while adding repair-loop behavior.
- On Windows, prefer serial Cargo validation when tests touch local servers or
  file locks.

Every behavior PR body should include:

- scope;
- expected files;
- behavior change;
- validation;
- rollback;
- follow-up.

## PR Ladder

### PR 1: Report Repair Item Model

Title: `feat(report): add receipt-derived repair items`

Status: landed in [#296](https://github.com/EffortlessMetrics/shiplog/pull/296)

Depends on:

- `SHIPLOG-SPEC-0005-evidence-repair-loop`
- `SHIPLOG-ADR-0006-repair-actions-are-receipt-derived`

Scope:

- Add an optional `repair_items` array to `intake.report.json` v1.
- Add Rust model types for repair items, actions, receipt refs, and stable
  repair keys.
- Generate initial repair items from existing report receipts only.
- Cover at least manual evidence missing, skipped configuration, stale/cached
  source guidance, evidence debt, share redaction, and artifact guidance where
  current receipts support them.
- Update schema docs and compatibility notes.

Expected files:

- `apps/shiplog/src/schema/**`
- `apps/shiplog/src/report/**` or nearest report owner module
- `apps/shiplog/tests/intake_cold_start.rs`
- `apps/shiplog/tests/cli_integration.rs`
- `contracts/schemas/intake-report.v1.schema.json`
- `docs/schemas/intake-report-v1.md`
- `.shiplog/goals/active.toml`

Behavior change:

- Yes, `intake.report.json` may include optional `repair_items`.
- Existing reports without `repair_items` remain valid.

Validation:

```bash
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test cli_integration -- intake
cargo test -p shiplog --test cli_integration -- report
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
cargo xtask check-executable-files --mode blocking-allowlist
git diff --check
```

Rollback:

- Revert writer/model/schema/docs/tests together. Do not leave docs or schema
  advertising fields that writers do not emit.

Follow-up:

- `repair plan --latest` should render these items without reclassifying
  sources.

### PR 2: Repair Plan CLI

Title: `feat(repair): render latest repair plan`

Status: landed in [#297](https://github.com/EffortlessMetrics/shiplog/pull/297)

Depends on:

- report repair item model;
- latest-run resolution already used by `shiplog open`.

Scope:

- Add `shiplog repair plan --latest`.
- Read the latest `intake.report.json`.
- Render repair items in deterministic, copyable order.
- Handle no latest report, old compatible reports without `repair_items`,
  invalid report JSON, and empty repair queues.
- Do not query providers or inspect Markdown.

Expected files:

- `apps/shiplog/src/main.rs`
- repair command owner module, if extracted
- `apps/shiplog/tests/cli_integration.rs`
- `README.md` or `docs/guides/**`, only if visible command docs are added
- `.shiplog/goals/active.toml`

Behavior change:

- Yes, new CLI command.

Validation:

```bash
cargo test -p shiplog --test cli_integration -- repair
cargo test -p shiplog --test intake_cold_start
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Rollback:

- Revert the command, help text, and tests. Keep report model if PR 1 remains
  useful and compatible.

Follow-up:

- Journal repair can resolve `repair_id` through the same latest-report loader.

### PR 3: Journal From Repair

Title: `feat(journal): add from-repair flow`

Status: landed in [#298](https://github.com/EffortlessMetrics/shiplog/pull/298)

Depends on:

- `repair plan --latest`;
- repair item action kind `journal_add`.

Scope:

- Add `shiplog journal add --from-repair <repair_id>`.
- Resolve the ID from the latest report.
- Act only on `journal_add` repair items.
- Fail usefully for unknown IDs, non-journal repair IDs, no latest report, and
  reports without repair items.
- Avoid destructive overwrites of existing journal content.

Expected files:

- `apps/shiplog/src/main.rs`
- journal command owner module
- `apps/shiplog/tests/cli_integration.rs`
- `apps/shiplog/tests/front_door_first_pack_smoke.rs`, if the product path is
  extended here
- `.shiplog/goals/active.toml`

Behavior change:

- Yes, new journal repair command.

Validation:

```bash
cargo test -p shiplog --test cli_integration -- journal
cargo test -p shiplog --test cli_integration -- repair
cargo test -p shiplog --test intake_cold_start
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Rollback:

- Revert journal command behavior and tests. Keep `repair plan` if it remains
  valid without journal mutation.

Follow-up:

- End-to-end proof should show journal repair improving a rerun packet.

### PR 4: Repair Diff

Title: `feat(repair): diff latest repair states`

Status: landed in [#299](https://github.com/EffortlessMetrics/shiplog/pull/299)

Depends on:

- stable `repair_key` values in report repair items.

Scope:

- Add `shiplog repair diff --latest` or the final chosen equivalent command.
- Compare latest two compatible reports by `repair_key`.
- Show cleared, new, still-open, and changed repair items.
- Handle missing second report, old reports, and empty queues.

Expected files:

- `apps/shiplog/src/main.rs`
- repair command owner module
- `apps/shiplog/tests/cli_integration.rs`
- `.shiplog/goals/active.toml`

Behavior change:

- Yes, new repair diff command.

Validation:

```bash
cargo test -p shiplog --test cli_integration -- repair
cargo test -p shiplog --test front_door_first_pack_smoke
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Rollback:

- Revert diff command and tests. Keep repair IDs/keys if used by earlier
  commands.

Follow-up:

- Worked guide should teach how to interpret cleared and still-open items.

### PR 5: End-To-End Product Proof And Guide

Title: `test(product): prove repair loop improves first packet`

Status: landed in [#300](https://github.com/EffortlessMetrics/shiplog/pull/300)

Depends on:

- report repair items;
- repair plan;
- journal repair;
- repair diff.

Scope:

- Add or extend a product-level test proving:
  1. cold first run produces a repair plan;
  2. manual evidence repair can be added from a repair ID;
  3. rerun intake changes the repair state;
  4. packet usefulness improves without provider mutation.
- Add a short guide for the repair loop.
- Update README pointers only if the command surface is ready for ordinary
  users.

Expected files:

- `apps/shiplog/tests/front_door_first_pack_smoke.rs`
- `apps/shiplog/tests/intake_cold_start.rs`
- `docs/guides/**`
- `README.md`, if needed
- `.shiplog/goals/active.toml`

Behavior change:

- No new behavior beyond earlier PRs; this PR proves and documents the complete
  loop.

Validation:

```bash
cargo test -p shiplog --test front_door_first_pack_smoke
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test cli_integration -- repair
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
cargo xtask check-executable-files --mode blocking-allowlist
git diff --check
```

Rollback:

- Revert guide and product proof if the previous behavior PRs still stand.
  Revert earlier behavior only if the proof exposes a contract flaw.

Follow-up:

- Prepare release notes for the repair-loop release.

### PR 6: Repair Loop Release Prep

Title: `release: prepare evidence repair loop release`

Status: landed in [#301](https://github.com/EffortlessMetrics/shiplog/pull/301)

Scope:

- Freeze changelog/release notes for the repair-loop release.
- Confirm `cargo install shiplog` behavior and release-install smoke still pass.
- Validate schema docs, guide links, policy gates, and product proof.
- Archive or roll forward the active goal only after the release ships.

Expected files:

- `CHANGELOG.md`
- `docs/release/**`
- `RELEASE_HANDOFF_0.8.0.md`
- `.shiplog/goals/active.toml`
- `policy/publish-allowlist.toml`, if the release target marker changes
- Cargo version files and `Cargo.lock`, if the release-prep PR bumps the
  release target

Validation:

```bash
cargo test -p shiplog --test front_door_first_pack_smoke
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test cli_integration -- repair
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo install --path apps/shiplog --locked --root target/release-install-local --force
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
cargo xtask check-executable-files --mode blocking-allowlist
git diff --check
```

Rollback:

- Revert release-prep docs and version changes. Do not revert landed
  repair-loop behavior unless validation proves a behavior blocker.

## Stop Conditions

Stop and ask before proceeding when:

- a repair action would require provider mutation;
- a schema change cannot stay backward-compatible under v1;
- a repair item would need secret values or private provider IDs to be useful;
- a repair command needs to inspect Markdown to work;
- the implementation discovers that existing report receipts cannot justify a
  proposed repair kind.
