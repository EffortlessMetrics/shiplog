# Shiplog User-Polish Implementation Plan

Status: done
Owner: Codex
Created: 2026-05-13
Related proposal:
[`SHIPLOG-PROP-0001-user-polish-release`](../../docs/proposals/SHIPLOG-PROP-0001-user-polish-release.md)
Archived goal:
[`2026-05-13-user-polish-0.6.0.toml`](../../.shiplog/goals/archive/2026-05-13-user-polish-0.6.0.toml)

## Purpose

This plan sequences the user-polish implementation PRs after the
source-of-truth stack has landed. Specs define what must be true. ADRs define
durable architectural decisions. This plan defines PR order, proof commands,
rollback, and follow-up.

The lane remained narrow: make the first-run review-pack path obvious,
receipt-backed, and safe to extend.

Target user path:

```bash
shiplog intake --last-6-months --explain
shiplog open intake-report --latest
shiplog open packet --latest
```

## Source Of Truth

Proposal:

- [`SHIPLOG-PROP-0001-user-polish-release`](../../docs/proposals/SHIPLOG-PROP-0001-user-polish-release.md)

Specs:

- [`SHIPLOG-SPEC-0001-rapid-first-intake`](../../docs/specs/SHIPLOG-SPEC-0001-rapid-first-intake.md)
- [`SHIPLOG-SPEC-0002-intake-report-v1`](../../docs/specs/SHIPLOG-SPEC-0002-intake-report-v1.md)
- [`SHIPLOG-SPEC-0003-source-identity`](../../docs/specs/SHIPLOG-SPEC-0003-source-identity.md)

ADRs:

- [`SHIPLOG-ADR-0001-ingest-output-is-receipt-boundary`](../../docs/adr/SHIPLOG-ADR-0001-ingest-output-is-receipt-boundary.md)
- [`SHIPLOG-ADR-0002-machine-source-keys-vs-display-labels`](../../docs/adr/SHIPLOG-ADR-0002-machine-source-keys-vs-display-labels.md)
- [`SHIPLOG-ADR-0003-stale-requires-cachelookup`](../../docs/adr/SHIPLOG-ADR-0003-stale-requires-cachelookup.md)

Related issues:

- [#197](https://github.com/EffortlessMetrics/shiplog/issues/197):
  rapid first-intake lane umbrella.
- [#203](https://github.com/EffortlessMetrics/shiplog/issues/203):
  first-run report render polish.
- [#223](https://github.com/EffortlessMetrics/shiplog/issues/223):
  source naming across report JSON sections.
- [#224](https://github.com/EffortlessMetrics/shiplog/issues/224):
  `CacheLookup::{Fresh, Stale, Miss}`; implemented by
  [#244](https://github.com/EffortlessMetrics/shiplog/pull/244).
- [#225](https://github.com/EffortlessMetrics/shiplog/issues/225):
  recorded HTTP fixtures for warm-rerun freshness; implemented by
  [#245](https://github.com/EffortlessMetrics/shiplog/pull/245).
- [#229](https://github.com/EffortlessMetrics/shiplog/issues/229):
  skipped sources in `source_freshness`.

Scaffold PRs already landed:

- [#230](https://github.com/EffortlessMetrics/shiplog/pull/230):
  source-of-truth scaffold.
- [#231](https://github.com/EffortlessMetrics/shiplog/pull/231):
  user-polish proposal.
- [#232](https://github.com/EffortlessMetrics/shiplog/pull/232):
  rapid first-intake spec.
- [#233](https://github.com/EffortlessMetrics/shiplog/pull/233):
  intake-report and source-identity specs.
- [#234](https://github.com/EffortlessMetrics/shiplog/pull/234):
  receipt, source identity, and stale ADRs.

Implementation and release receipts:

- [#236](https://github.com/EffortlessMetrics/shiplog/pull/236):
  intake next-step footer.
- [#237](https://github.com/EffortlessMetrics/shiplog/pull/237):
  latest artifact opening.
- [#238](https://github.com/EffortlessMetrics/shiplog/pull/238):
  source identity canonicalization.
- [#243](https://github.com/EffortlessMetrics/shiplog/pull/243):
  skipped sources in freshness.
- [#244](https://github.com/EffortlessMetrics/shiplog/pull/244):
  `CacheLookup` and stale freshness.
- [#245](https://github.com/EffortlessMetrics/shiplog/pull/245):
  recorded HTTP warm-rerun fixtures.
- [#246](https://github.com/EffortlessMetrics/shiplog/pull/246):
  user-polish 0.6.0 release prep.
- [#247](https://github.com/EffortlessMetrics/shiplog/pull/247):
  active goal archive after release.

Release receipts:

- [`docs/release/0.6.0-readiness.md`](../../docs/release/0.6.0-readiness.md)
- [`RELEASE_HANDOFF_0.6.0.md`](../../RELEASE_HANDOFF_0.6.0.md)

## Operating Rules

- One implementation behavior change per PR.
- Do not reopen protected-fields or `disallowed_fields`.
- Do not add skipped-source freshness until source identity is canonical.
- Do not emit `stale` until `ApiCache` exposes a lookup result that proves a
  stale hit.
- Keep JSON report contracts schema-backed and Markdown reviewer-friendly.
- Keep guides user-facing; update them only when commands or visible behavior
  change.
- Every PR body must include scope, files expected, behavior change,
  validation, rollback, and follow-up.

## PR Ladder

### PR 1: CLI Next-Step Footer

Title: `feat(cli): print next-step commands after intake`

Status: done

Depends on:

- `SHIPLOG-SPEC-0001-rapid-first-intake`
- `SHIPLOG-SPEC-0002-intake-report-v1`

Scope:

- After a successful intake run, print the run directory and concise next-step
  commands.
- When readiness is `Needs evidence`, print the manual-evidence repair loop.
- Keep the generated report as the durable receipt; the terminal footer is
  guidance.

Expected files:

- `apps/shiplog/src/main.rs`
- `apps/shiplog/tests/intake_cold_start.rs`
- `apps/shiplog/tests/front_door_first_pack_smoke.rs`
- `docs/guides/rapid-first-intake.md`, only if visible command wording changes.

Behavior change:

- Yes, user-visible CLI stdout after `shiplog intake`.

Validation:

```bash
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test front_door_first_pack_smoke
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Rollback:

- Revert the footer rendering and stdout assertions. Generated artifacts should
  remain unchanged unless the PR deliberately adds report next-command fields.

Receipt:

- Landed in [#236](https://github.com/EffortlessMetrics/shiplog/pull/236).
- Release readiness records this as `Intake next-step footer (#236)`.

Follow-up:

- Completed by PR 2.

### PR 2: Latest Artifact Opening

Title: `feat(open): improve latest artifact discovery`

Status: done

Depends on:

- `SHIPLOG-SPEC-0001-rapid-first-intake`

Scope:

- Support:

```bash
shiplog open intake-report --latest
shiplog open packet --latest
shiplog open out --latest
```

- Select the latest run deterministically.
- When no run exists, print the exact intake command that creates one.
- Keep path handling Windows-safe.

Expected files:

- `apps/shiplog/src/main.rs`
- CLI/open tests near existing integration coverage.
- `docs/guides/rapid-first-intake.md`, if command examples change.

Behavior change:

- Yes, CLI open resolution and error messages.

Validation:

```bash
cargo test -p shiplog --test cli_integration -- open
cargo test -p shiplog --test intake_cold_start
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Rollback:

- Revert open-latest resolution and tests. The intake footer can keep pointing
  to commands only if they still work; otherwise revert the footer command text
  in the same rollback.

Receipt:

- Landed in [#237](https://github.com/EffortlessMetrics/shiplog/pull/237).
- Release readiness records this as `Latest artifact opening (#237)`.

Follow-up:

- Included in the 0.6.0 release notes.

### PR 3: Source Identity Canonicalization

Title: `feat(report): canonicalize source identities in intake.report.json`

Status: done

Depends on:

- `SHIPLOG-SPEC-0002-intake-report-v1`
- `SHIPLOG-SPEC-0003-source-identity`
- `SHIPLOG-ADR-0002-machine-source-keys-vs-display-labels`

Scope:

- Add canonical `source_key` and display-only `source_label` to source-facing
  JSON report entries.
- Decide and document v1 compatibility for any retained legacy `source` field.
- Update schema, schema docs, and tests together.
- Preserve Markdown-friendly labels.

Expected files:

- `apps/shiplog/src/main.rs`
- `contracts/schemas/intake-report.v1.schema.json`
- `docs/schemas/intake-report-v1.md`
- `apps/shiplog/tests/intake_cold_start.rs`
- `apps/shiplog/tests/cli_integration.rs`
- `docs/specs/SHIPLOG-SPEC-0003-source-identity.md`, only for compatibility
  notes discovered during implementation.

Behavior change:

- Yes, machine-readable report JSON contract changes.

Validation:

```bash
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test cli_integration -- intake
cargo test -p shiplog --test cli_integration -- report
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Rollback:

- Revert JSON writer, schema/docs, and tests together. Do not leave schema docs
  advertising fields that writers do not emit.

Receipt:

- Landed in [#238](https://github.com/EffortlessMetrics/shiplog/pull/238).
- Release readiness records this as `Source identity canonicalization (#238)`.

Follow-up:

- Completed by PR 4.

### PR 4: Skipped Sources In Freshness

Title: `feat(report): include skipped sources in source_freshness`

Status: done

Depends on:

- `SHIPLOG-ADR-0001-ingest-output-is-receipt-boundary`
- `SHIPLOG-ADR-0002-machine-source-keys-vs-display-labels`
- [Issue #229](https://github.com/EffortlessMetrics/shiplog/issues/229)

Scope:

- Add configured-but-skipped sources to `source_freshness` with
  `status = "skipped"` and a reason.
- Dedupe against adapter successes and configured-source failures by
  `source_key`.
- Do not infer freshness from absence.

Expected files:

- `apps/shiplog/src/main.rs`
- `apps/shiplog/tests/intake_cold_start.rs`
- `apps/shiplog/tests/cli_integration.rs`
- `contracts/schemas/intake-report.v1.schema.json`, only if the shape changes.
- `docs/schemas/intake-report-v1.md`, if schema docs change.

Behavior change:

- Yes, report JSON/Markdown source freshness becomes a more complete per-source
  rollup.

Validation:

```bash
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test cli_integration -- intake
cargo test -p shiplog --test cli_integration -- report
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Rollback:

- Revert skipped freshness population and assertions. Keep source identity
  canonicalization intact.

Receipt:

- Landed in [#243](https://github.com/EffortlessMetrics/shiplog/pull/243).
- Release readiness records this as `Skipped sources in freshness (#243)`.

Follow-up:

- Completed by PR 6.

### PR 5: CacheLookup And Stale

Title: `feat(cache): add CacheLookup for stale freshness`

Status: done

Depends on:

- `SHIPLOG-ADR-0001-ingest-output-is-receipt-boundary`
- `SHIPLOG-ADR-0003-stale-requires-cachelookup`
- [Issue #224](https://github.com/EffortlessMetrics/shiplog/issues/224)

Scope:

- Add a cache lookup result that distinguishes fresh hit, stale hit, and miss.
- Thread stale-hit evidence through the relevant adapter receipt path.
- Emit `stale` only from a proven stale cache lookup.

Expected files:

- `crates/shiplog-cache/src/`
- `crates/shiplog-cache/tests/`
- affected ingest adapter crates.
- `apps/shiplog/src/schema/freshness.rs`, only if docs or counters change.
- focused adapter tests.

Behavior change:

- Yes, cache API and freshness behavior.

Validation:

```bash
cargo test -p shiplog lookup_distinguishes_fresh_stale_and_miss
cargo test -p shiplog expired_cache_entry_records_stale_hit_without_network
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Rollback:

- Revert the cache API, adapter stale path, and tests together. Do not leave
  adapters querying cache internals directly.

Follow-up:

- Recorded HTTP fixtures prove warm reruns without live network in PR 6.
  Stale behavior remains proved by the CacheLookup stale-hit test.

Receipt:

- Landed in [#244](https://github.com/EffortlessMetrics/shiplog/pull/244).
- Release readiness records this as `CacheLookup and stale freshness (#244)`.

### PR 6: Recorded HTTP Fixtures

Title: `test(harness): add recorded HTTP fixtures for full warm-rerun`

Status: done

Depends on:

- [Issue #225](https://github.com/EffortlessMetrics/shiplog/issues/225)

Scope:

- Add recorded fixtures for a full warm-rerun proof without live network.
- Prove run 1 cache miss/fresh and run 2 cache hit/cached for the same logical
  source.
- Keep credentials and source opaque IDs out of fixtures.

Expected files:

- test harness files under the existing fixture/test layout.
- focused integration tests for warm reruns.
- docs or policy notes only if the fixture workflow changes.

Behavior change:

- No product behavior change unless fixture harness support requires a guarded
  test-only path.

Validation:

```bash
cargo test -p shiplog recorded_http_fixtures_prove_full_fresh_then_cached_ingest
cargo test -p shiplog warm_rerun_cache_primitive_round_trips_miss_then_hit
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Rollback:

- Remove fixtures and tests. Do not loosen live-network guards to compensate.

Follow-up:

- Release prep can cite fixture-backed freshness proof.

Receipt:

- Landed in [#245](https://github.com/EffortlessMetrics/shiplog/pull/245).
- Release readiness records this as `Recorded HTTP warm-rerun fixtures (#245)`.

### PR 7: User-Polish Release Prep

Title: `release: prepare user-polish release`

Status: done

Depends on:

- All prior implementation PRs.

Scope:

- Prepare release notes and release proof for the user-polish lane.
- Separate user-facing improvements from internal proof work.
- Confirm docs, schemas, tests, and policy ledgers agree on the final contract.

Expected files:

- `CHANGELOG.md` or release-note surface used by the repo.
- release checklist/docs used by shiplog.
- docs/spec updates only for compatibility notes discovered during release
  proof.

Behavior change:

- No runtime behavior change unless release proof exposes a narrow bug that
  needs its own PR.

Validation:

```bash
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test front_door_first_pack_smoke
cargo test -p shiplog --test cli_integration -- intake
cargo test -p shiplog --test cli_integration -- report
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Rollback:

- Revert release-note/checklist changes. Do not revert implementation PRs from
  the release-prep PR unless the release-prep PR introduced the regression.

Follow-up:

- Completed by [#247](https://github.com/EffortlessMetrics/shiplog/pull/247).
  The archived goal is
  [`.shiplog/goals/archive/2026-05-13-user-polish-0.6.0.toml`](../../.shiplog/goals/archive/2026-05-13-user-polish-0.6.0.toml).

Receipt:

- Release prep landed in [#246](https://github.com/EffortlessMetrics/shiplog/pull/246).
- Release readiness records this as `Release prep (#246)`.

## Stop Conditions

Stop and record a decision receipt before proceeding if:

- implementing a work item requires reopening protected-fields or
  `disallowed_fields`;
- source identity cannot be changed compatibly under report schema v1;
- stale reporting would require guessing from a cache miss;
- latest-run discovery depends on non-deterministic filesystem ordering;
- a proposed proof command needs live network to pass.

## Release Thesis

Shiplog now gets a first-time user from an empty directory to a defensible
review pack, with clear source state, freshness, redaction posture, next-step
commands, latest-artifact opening, and proof-backed docs/tests.

## Closure Receipt

The user-polish lane shipped as `v0.6.0` on 2026-05-13. The release readiness
ledger records each implementation PR as merged, the release handoff records
the release operation, and the active goal was archived by #247.
