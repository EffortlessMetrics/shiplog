# SHIPLOG-SPEC-0003: Source Identity

Status: proposed
Owner: product/schema
Created: 2026-05-13
Related proposal:
[`SHIPLOG-PROP-0001-user-polish-release`](../proposals/SHIPLOG-PROP-0001-user-polish-release.md)
Related report spec:
[`SHIPLOG-SPEC-0002-intake-report-v1`](SHIPLOG-SPEC-0002-intake-report-v1.md)

## Purpose

This spec defines source identity for intake reports. The goal is that
machine-readable JSON uses stable source keys while human-facing Markdown uses
friendly display labels.

This is the source identity contract used by current intake-report writers:
skipped-source freshness, cross-section joins, and future agent flows join on
`source_key` instead of recreating display-label normalization.

## Historical Problem

Before the source identity work landed, the same conceptual source could appear
under different strings:

```text
source_freshness[].source = github / git / json_import
source_decisions[].source = GitHub / Local git / JSON
included/skipped sources  = display labels
```

Issue
[#223](https://github.com/EffortlessMetrics/shiplog/issues/223) records the
original schema-consistency gap. Issue
[#229](https://github.com/EffortlessMetrics/shiplog/issues/229) records why
skipped-source freshness was deferred until identity was settled.

The failure mode is not cosmetic. A report consumer cannot join freshness and
decision entries reliably without reproducing shiplog's private
normalization/display-label rules.

## Decision

JSON uses canonical machine keys. Markdown uses display labels.

The preferred JSON shape for every source-facing report entry is:

```json
{
  "source_key": "github",
  "source_label": "GitHub"
}
```

`source_key` is the join key. `source_label` is display-only. Markdown renders
`source_label` or computes an equivalent label from `source_key`; it must not
expose raw implementation aliases when a friendly label exists.

## Canonical Source Keys

Canonical source keys are lower-case ASCII identifiers using snake case.

Initial vocabulary:

```text
github
gitlab
jira
linear
manual
json
git
unknown
```

Compatibility aliases that may appear in older reports or adapter-local
receipts:

```text
json_import -> json
jsonimport   -> json
local_git    -> git
localgit     -> git
```

New source keys require:

- schema vocabulary update;
- display-label mapping;
- tests for every report section that emits source identity;
- docs update in this spec or a linked successor spec.

## Source Labels

Display labels are human-facing strings. Initial labels:

```text
github  -> GitHub
gitlab  -> GitLab
jira    -> Jira
linear  -> Linear
manual  -> Manual
json    -> JSON
git     -> Local git
unknown -> Unknown
```

Labels can change for readability without changing the machine key, but a label
change should update Markdown snapshots and user-facing docs when visible.

## Report Sections

The source identity shape applies to:

```text
included_sources[]
skipped_sources[]
source_decisions[]
source_freshness[]
repair_sources[]
```

After implementation, each entry should expose `source_key` and `source_label`.
Legacy `source` may be retained temporarily only if the schema/docs specify the
migration behavior and readers are told whether it is a key or label.

The long-term contract is:

- JSON joins on `source_key`;
- JSON can display `source_label` without recomputing labels;
- Markdown shows friendly labels;
- validators reject unknown `source_key` values unless the schema explicitly
  allows extension keys.

## Freshness And Skipped Sources

Skipped sources now appear in `source_freshness` using the same
`source_key` / `source_label` identity shape as `source_decisions`. A skipped
source entry reports:

```json
{
  "source": "github",
  "source_key": "github",
  "source_label": "GitHub",
  "status": "skipped",
  "cache_hits": 0,
  "cache_misses": 0,
  "reason": "missing GITHUB_TOKEN"
}
```

Current tests prove cold-start skipped sources are present in
`source_freshness`, carry non-empty reasons, and do not duplicate
`source_key` rows.

Do not reintroduce display-label normalization in tests. That would preserve
the original bug this spec removed.

## Acceptance Criteria

The source identity implementation is complete when:

- every source-facing JSON report entry carries `source_key`;
- every source-facing JSON report entry carries `source_label` or has a
  documented compatibility reason not to;
- `source_key` validates against the canonical vocabulary;
- `shiplog report validate` rejects current reports whose `source`,
  `source_key`, and `source_label` disagree;
- `source_decisions` and `source_freshness` join directly on `source_key`;
- tests no longer need report-local normalization helpers to compare source
  sections;
- Markdown still renders friendly labels;
- skipped sources appear in `source_freshness` without naming drift;
- historical v1 reports either keep validating or have a documented migration
  path.

## Proof Mapping

Current evidence:

- [`docs/conventions.md`](../conventions.md#conventions-under-discussion)
  records the inconsistency as an unsettled convention.
- [`apps/shiplog/src/main.rs`](../../apps/shiplog/src/main.rs) contains the
  current private `normalized_source_key` and `display_source_label` helpers.
- [`contracts/schemas/intake-report.v1.schema.json`](../../contracts/schemas/intake-report.v1.schema.json)
  defines the canonical `source_key` vocabulary, while
  `shiplog report validate` enforces the current identity triplet when reports
  include `source_key` / `source_label`.
- [`apps/shiplog/tests/intake_cold_start.rs`](../../apps/shiplog/tests/intake_cold_start.rs)
  currently checks first-run source decisions and freshness.
- [`apps/shiplog/tests/cli_integration.rs`](../../apps/shiplog/tests/cli_integration.rs)
  contains report shape and source-array assertions that must move to the new
  identity contract when behavior changes.

Useful validation commands for the implementation PR:

```bash
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test cli_integration -- intake
cargo test -p shiplog --test cli_integration -- report
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

## Compatibility Notes

Canonical source identity landed as a compatible v1 extension. Current writers
emit `source_key` and `source_label` on source-facing report entries, and retain
`source` as a compatibility alias that mirrors `source_key`. Older v1 reports
may only have `source`, and some historical reports used display labels there.

Readers should join on `source_key` when present, fall back to normalizing
`source` when it is absent, and display `source_label` when present.
`shiplog report validate` preserves that compatibility path by allowing older
reports without `source_key`, while rejecting current reports whose source
identity fields drift apart.

## Non-Goals

This spec does not define:

- freshness statuses;
- cache hit/miss semantics;
- source adapter configuration names;
- provider-specific account identity;
- source opaque ID redaction.

Source opaque IDs remain covered by the protected-fields audit trail and
redaction policy, not by this source-name contract.
