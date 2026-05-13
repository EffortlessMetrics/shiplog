# SHIPLOG-SPEC-0001: Rapid First Intake

Status: proposed
Owner: product/docs
Created: 2026-05-13
Related proposal:
[`SHIPLOG-PROP-0001-user-polish-release`](../proposals/SHIPLOG-PROP-0001-user-polish-release.md)

## Purpose

This spec defines the first-run intake contract: what must be true when a user
starts in an empty directory and runs shiplog for the first time.

The contract is narrow. It does not define every field in
`intake.report.json`, the detailed source identity model, cache lookup
internals, or the full redaction/share policy. Those are separate specs and
ADRs. This spec owns the cold-start user promise and the proof surfaces that
keep that promise executable.

## Scope

In scope:

- empty-directory intake;
- first-run scaffold behavior;
- required first-run artifacts;
- readiness honesty when evidence is missing;
- terminal and report next-step guidance;
- source decision and freshness visibility at the first-run level;
- proof mapping to existing tests and guides.

Out of scope:

- report v1 schema details beyond fields needed for first-run readiness;
- canonical source key vs display label design;
- stale cache lookup semantics;
- latest-run discovery algorithm;
- manager/public redaction internals;
- PR sequencing and release order.

## User Contract

From a literal empty working directory, the following command is the first-run
entry point:

```bash
shiplog intake --last-6-months --explain
```

The command must not require the user to run `shiplog init`, `shiplog doctor`,
`shiplog collect`, or `shiplog render` first.

After intake completes, shiplog must make these next commands visible either in
terminal output, the generated report, or both:

```bash
shiplog open intake-report --latest
shiplog open packet --latest
```

If the run needs evidence, shiplog must also point at the fastest repair loop:

```bash
shiplog journal add
shiplog intake --last-6-months --explain
```

The `open ... --latest` resolution rules are owned by the later open-latest
spec. This spec only requires that first-run output points users at the
canonical next commands.

## First-Run Behavior

### Empty Directory

An empty-directory run means the current directory starts with no
`shiplog.toml`, no `manual_events.yaml`, and no `out/` directory.

On the first run, shiplog must:

- create a starter `shiplog.toml`;
- create a starter `manual_events.yaml`;
- preserve those files on rerun instead of overwriting user edits;
- write output under `./out/<run_id>/`, where the run directory is sortable and
  does not overwrite prior runs.

### Source Selection

The first run should attempt every source that has enough configuration or
environment to be useful. Sources without tokens, configured identity, or local
inputs must be skipped explicitly rather than silently ignored.

At minimum:

- the scaffolded manual source must be visible as considered;
- skipped provider sources must carry non-empty reasons;
- the user must be able to answer "which sources worked?" and "which sources
  were skipped?" from the generated report.

Canonical source identity is owned by
`SHIPLOG-SPEC-0003-source-identity`. Until that spec lands, this first-run
contract only requires source visibility and non-empty reasons.

### Required Artifacts

Each successful first-run intake must create a run directory containing:

- `packet.md`;
- `intake.report.md`;
- `intake.report.json`;
- `ledger.events.jsonl`;
- `coverage.manifest.json`;
- `workstreams.suggested.yaml`;
- `bundle.manifest.json`.

The report and packet are the first two artifacts a user opens. The ledger,
coverage manifest, suggested workstreams, and bundle manifest are supporting
receipts.

### Readiness

Readiness must be honest. A first run with no collected events is not a broken
run if at least one source succeeded structurally, such as the scaffolded manual
source reading an empty file. It must exit successfully and report:

```text
Needs evidence
```

The report must explain the missing-evidence gap in plain language so a user or
reviewer sees it before treating the packet as complete.

The command exits non-zero only when zero sources succeeded.

### Report Framing

`intake.report.md` must be useful under time pressure. A first-run report must
surface:

- packet readiness;
- redaction profile;
- where to look for the durable receipts;
- source freshness;
- included and skipped sources;
- source decisions with reasons;
- repair guidance;
- next commands.

The report is not a long-form guide. It is a generated receipt and checklist.

### Freshness

The first-run report must include per-source freshness. On a cold start, no
source may report `cached`; the cache starts empty. A source that reads current
local input or successfully fetches live data reports `fresh`.

The detailed freshness vocabulary and stale-cache boundary are owned by
`SHIPLOG-SPEC-0004-freshness-and-cache`. This spec only requires first-run
freshness visibility and forbids pretending cold-start data came from cache.

## Acceptance Criteria

A change preserves this spec when:

- `shiplog intake --last-6-months --explain` works from an empty directory;
- the run creates starter `shiplog.toml` and `manual_events.yaml`;
- the run creates all required artifacts under a new `out/<run_id>/`:
  `packet.md`, `intake.report.md`, `intake.report.json`,
  `ledger.events.jsonl`, `coverage.manifest.json`,
  `workstreams.suggested.yaml`, and `bundle.manifest.json`;
- `readiness` is `Needs evidence` when the run collected zero events but at
  least one source succeeded;
- zero-source-success runs exit non-zero;
- `source_decisions` is populated even when `--explain` is omitted;
- skipped sources carry reasons;
- `source_freshness` is populated;
- no cold-start source reports `cached`;
- the Markdown report contains `Where to Look`, `Source Freshness`, redaction
  profile, readiness, repair, and next-command framing;
- the terminal or report points at `shiplog open intake-report --latest`,
  `shiplog open packet --latest`, and the manual-evidence repair loop when
  evidence is missing.

## Proof Mapping

Current proof:

- [`apps/shiplog/tests/intake_cold_start.rs`](../../apps/shiplog/tests/intake_cold_start.rs)
  pins empty-directory intake, the first-open artifact set
  (`packet.md`, `intake.report.md`, `intake.report.json`) plus
  `coverage.manifest.json`, scaffolded config files, readiness, default window
  behavior, source-decision visibility, source freshness, and the
  zero-source-success exit branch.
- [`apps/shiplog/tests/front_door_first_pack_smoke.rs`](../../apps/shiplog/tests/front_door_first_pack_smoke.rs)
  pins the install-to-first-pack smoke path and verifies the report framing
  elements a first-time user scans first.
- [`docs/product/rapid-first-intake.md`](../product/rapid-first-intake.md)
  explains the product promise and current cold-start behavior.
- [`docs/guides/rapid-first-intake.md`](../guides/rapid-first-intake.md)
  teaches the user-facing command path and how to read the output.

Useful validation commands:

```bash
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test front_door_first_pack_smoke
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Proof gaps to close when behavior changes:

- if the terminal next-step footer changes, add stdout assertions for the
  `open ... --latest` commands and missing-evidence repair loop;
- if required artifact names change, update this spec, the product doc, guide,
  and the cold-start artifact assertions together. The supporting receipt
  artifacts (`ledger.events.jsonl`, `workstreams.suggested.yaml`,
  `bundle.manifest.json`) are documented here and in the guide; add explicit
  test assertions if their first-run presence changes;
- if source identity changes, route the contract through
  `SHIPLOG-SPEC-0003-source-identity` instead of adding local normalization to
  first-run tests.

## Compatibility

This spec does not by itself change a persisted schema. It constrains the
first-run behavior that already writes `intake.report.{md,json}` and the run
bundle artifacts.

Any future change that removes a required artifact, changes readiness meaning,
or changes first-run exit semantics must update this spec and the matching
proof before it lands.

## Related Future Specs

- `SHIPLOG-SPEC-0002-intake-report-v1` owns the detailed JSON/Markdown report
  contract and schema-level secret-vocabulary firewall.
- `SHIPLOG-SPEC-0003-source-identity` owns canonical machine source keys and
  display labels.
- `SHIPLOG-SPEC-0004-freshness-and-cache` owns `fresh`, `cached`, `skipped`,
  `unavailable`, and reserved `stale` semantics.
- `SHIPLOG-SPEC-0005-open-latest` owns deterministic latest-artifact
  discovery.
- `SHIPLOG-SPEC-0007-evidence-repair-loop` owns the full "what now?" repair
  checklist after missing evidence.
