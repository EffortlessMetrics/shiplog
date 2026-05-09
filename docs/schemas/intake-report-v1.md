# Intake Report v1

`intake.report.json` is the machine-readable control surface written beside
each `shiplog intake` run. The Markdown report is for humans; this JSON report
is for scripts, future local UI/TUI surfaces, and agents that need to inspect a
run without scraping terminal output.

The v1 schema lives at:

```text
contracts/schemas/intake-report.v1.schema.json
```

## Compatibility

The top-level `schema_version` field is required and must be `1`.

The following top-level fields are stable for v1 consumers:

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

Consumers should treat display strings, paths, command strings, and ordering as
best-effort user-facing guidance. They are stable enough to show to a user, but
not a promise that a later v1 report will use identical wording for every
finding. Use `schema_version`, `run_id`, source arrays, repair arrays, evidence
debt fields, and artifact labels for control flow.

## Readiness

`readiness` is a packet-quality status, not a score for the person whose work is
being reviewed.

Allowed values:

```text
Ready for review
Needs curation
Needs evidence
Needs repair
```

## Secrets

The report must not include token values, redaction keys, passwords, or secret
material. The schema deliberately avoids secret-bearing fields, and tests keep
known secret sentinels out of generated report text. Repair commands should show
environment variable names such as `JIRA_TOKEN`, not their values.

## Source And Repair Fields

`included_sources` records sources that produced a usable result.
`skipped_sources` records configured or attempted sources that did not produce
usable evidence. `source_decisions` explains why intake included or skipped a
source. `repair_sources` carries copy-ready setup and rerun commands for skipped
or unusable sources.

Provider failure kinds are still represented as human-readable `reason` strings
in v1. A later v1-compatible extension may add a classifier only if it remains
additive or is introduced under a new schema version.

## Evidence Debt And Fixups

`evidence_debt` is about packet quality and must not be used for productivity scoring.
Each item includes:

```text
severity
kind
summary
detail
next_step
```

`top_fixups`, `journal_suggestions`, `share_commands`, and `next_commands` are
operator guidance. Commands should be shown as suggestions and should not be run
without user confirmation in future UI or agent surfaces.
