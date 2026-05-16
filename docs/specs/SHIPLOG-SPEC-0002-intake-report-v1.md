# SHIPLOG-SPEC-0002: Intake Report v1

Status: proposed
Owner: product/schema
Created: 2026-05-13
Related proposal:
[`SHIPLOG-PROP-0001-user-polish-release`](../proposals/SHIPLOG-PROP-0001-user-polish-release.md)
Related first-run spec:
[`SHIPLOG-SPEC-0001-rapid-first-intake`](SHIPLOG-SPEC-0001-rapid-first-intake.md)

## Purpose

This spec defines the contract for the two durable intake report artifacts:

```text
intake.report.json
intake.report.md
```

The JSON report is the machine-readable receipt. The Markdown report is the
human-readable checklist. Both are generated from the same intake result, but
they do not have the same authority: tools consume JSON, reviewers read
Markdown.

## Scope

In scope:

- v1 JSON report field stability and schema versioning;
- Markdown report section order and purpose;
- the schema-level secret-vocabulary firewall;
- the boundary between human prose and machine receipts;
- what kinds of changes require schema updates or compatibility notes;
- proof surfaces for report validation.

Out of scope:

- source key vocabulary and display labels, owned by
  [`SHIPLOG-SPEC-0003-source-identity`](SHIPLOG-SPEC-0003-source-identity.md);
- detailed freshness semantics, owned by the future freshness/cache spec;
- latest-run resolution, owned by the future open-latest spec;
- redaction/share profile internals, owned by the future redaction/share spec.

## JSON Report Contract

`intake.report.json` is the control surface for scripts, local UI/TUI surfaces,
and agents. It must validate against:

```text
contracts/schemas/intake-report.v1.schema.json
```

The schema requires these top-level fields today:

```text
schema_version
run_id
readiness
config_path
out_dir
run_dir
packet_path
period
window
reports
included_sources
skipped_sources
source_decisions
source_freshness
repair_sources
curation_notes
good
needs_attention
evidence_debt
top_fixups
journal_suggestions
share_commands
next_commands
artifacts
```

Current writers may include optional `actions`. Historical v1 readers must
treat absent `actions` as compatible.

Consumers may use these fields for control flow:

- `schema_version`;
- `run_id`;
- `readiness`;
- `period` and `window`;
- source arrays and repair arrays;
- evidence debt and fixup fields;
- artifact labels and paths;
- `actions`, when present.

Consumers must not depend on exact human prose in summary strings, command
strings, or Markdown section wording.

## Schema Versioning

`schema_version = 1` remains valid while changes are backward compatible for
existing v1 consumers.

Backward-compatible v1 changes include:

- adding an optional field with a clear absence behavior;
- adding an enum value only when the value was already reserved and documented;
- tightening docs around a field without changing its JSON shape;
- adding Markdown-only sections that do not change JSON.

Changes that require a schema update and compatibility note include:

- adding a required JSON field;
- removing or renaming a JSON field;
- changing a field type;
- changing an enum value's meaning;
- changing source identity shape in a way that affects joins;
- moving command/action semantics from human guidance into machine-required
  behavior.

If the report remains `schema_version = 1` while a compatibility-sensitive
field changes, the spec and schema docs must say how older reports are read.

## Source Arrays

The report has four source-facing arrays:

```text
included_sources
skipped_sources
source_decisions
source_freshness
```

Their identity contract is owned by
[`SHIPLOG-SPEC-0003-source-identity`](SHIPLOG-SPEC-0003-source-identity.md).
This spec requires only that source arrays remain present and structured enough
for validators and agents to inspect without scraping Markdown.

Until source identity is canonicalized, downstream code must treat display
strings as user-facing labels, not stable join keys.

## Freshness

`source_freshness` is required in v1. It reports per-source freshness state and
cache counts for the run.

Allowed status values are defined by the schema today:

```text
fresh
cached
stale
skipped
unavailable
```

`stale` may be emitted only when the cache API proves a stale hit. A report
writer must not infer `stale` from a miss, because a miss can mean an absent
row, purged row, expired row hidden by a legacy lookup, or different key.

## Markdown Report Contract

`intake.report.md` is the reviewer checklist. It must let a human answer:

- what run is this?
- is the packet ready?
- which report/packet should I open?
- what evidence source state should I trust?
- what sources were included or skipped?
- what needs repair?
- what should I run next?
- what can I share, and what needs redaction first?

The Markdown report should preserve this section order unless a PR explains why
a different order is better for a time-pressured reviewer:

1. Header and readiness summary.
2. Redaction profile.
3. Where to Look.
4. Source Freshness.
5. Included Sources.
6. Skipped Sources.
7. Source Decisions.
8. Repair Sources.
9. Evidence Debt.
10. Good / Needs Attention.
11. Top Fixups / Journal Suggestions.
12. Share Explain Commands.
13. Share Commands.
14. Next Commands.
15. Artifacts.

The Markdown report may use friendly labels, prose, and reordered details that
help a reviewer scan the result. It must not be the only source of a machine
contract.

## Human Prose Vs Machine Receipt Boundary

The JSON report owns stable structure. The Markdown report owns readable
presentation.

Use JSON for:

- joins across source arrays;
- readiness state;
- artifact discovery;
- repair/fixup/action categorization;
- validation and downstream agent decisions.

Use Markdown for:

- labels and explanations;
- reviewer-facing section order;
- brief next-step prose;
- display of copy-ready commands.

Do not make an agent scrape Markdown when the same fact can be represented in
JSON.

## Secret-Vocabulary Firewall

The report schema rejects secret-bearing field names before values exist. This
is a named pattern in shiplog:

```text
propertyNames regex as schema-level vocabulary gate
```

The canonical explanation lives in
[`docs/conventions.md`](../conventions.md#propertynames-regex-as-schema-level-vocabulary-gate).
The intake-report schema implements it through `$defs/non_secret_field_name`
and applies it recursively through `$defs/object_base`.

The firewall is vocabulary-level, not content-level:

- it rejects field names such as `github_token`, `api_key`, `password`, or
  `credential`;
- it does not inspect values;
- value-level secret prevention remains the job of redaction, sentinel tests,
  and scanners.

New nested objects in the report schema must inherit the same `propertyNames`
gate unless a PR records why the object is exempt.

## Proof Mapping

Current proof:

- [`contracts/schemas/intake-report.v1.schema.json`](../../contracts/schemas/intake-report.v1.schema.json)
  is the machine schema.
- [`docs/schemas/intake-report-v1.md`](../schemas/intake-report-v1.md)
  documents reader compatibility and stable fields.
- [`docs/conventions.md`](../conventions.md#propertynames-regex-as-schema-level-vocabulary-gate)
  records the secret-vocabulary firewall doctrine.
- [`apps/shiplog/tests/cli_integration.rs`](../../apps/shiplog/tests/cli_integration.rs)
  covers schema field allowlists, secret-bearing field rejection, report
  validation, source arrays, repair classifiers, actions, and agent-pack export.
- [`apps/shiplog/tests/intake_cold_start.rs`](../../apps/shiplog/tests/intake_cold_start.rs)
  covers first-run report existence, readiness, source decisions, and source
  freshness.
- [`apps/shiplog/tests/front_door_first_pack_smoke.rs`](../../apps/shiplog/tests/front_door_first_pack_smoke.rs)
  covers Markdown framing sections for the first-pack path.

Useful validation commands:

```bash
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test front_door_first_pack_smoke
cargo test -p shiplog --test cli_integration -- report
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

## Compatibility Notes

This spec does not change the v1 schema. It records the contract that later
implementation PRs must preserve or deliberately migrate.

The source identity spec may require a report JSON migration. That migration
must update the schema, schema docs, validator behavior, and tests in the same
PR as the behavior change.
