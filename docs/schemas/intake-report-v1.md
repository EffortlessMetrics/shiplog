# Intake Report v1

`intake.report.json` is the machine-readable control surface written beside
each `shiplog intake` run. The Markdown report is for humans; this JSON report
is for scripts, future local UI/TUI surfaces, and agents that need to inspect a
run without scraping terminal output.

The v1 schema lives at:

```text
contracts/schemas/intake-report.v1.schema.json
```

Validate a generated report and its referenced artifacts with:

```bash
shiplog report validate --latest
shiplog report validate --path out/<run>/intake.report.json
shiplog report summarize --latest
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

Current writers also include the optional top-level `actions` array. Older v1
reports may not have it, so readers should treat it as absent rather than
invalid when loading historical reports.

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

Current writers include a v1-compatible `repair_sources[].kind` classifier for
source repair loops. Older v1 reports may not have this optional field, so
readers should fall back to the human-readable `reason` when it is absent.
Known repair kinds are:

```text
missing_token
missing_identity
invalid_filter
bad_instance_url
auth_rejected
rate_limited
network_timeout
partial_results
cache_replay
local_source_unavailable
missing_file
setup_required
```

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

Current writers include stable `top_fixups[].id` and `top_fixups[].kind`
fields for future TUI and agent flows. Older v1 reports may not have these
optional fields, so readers should fall back to `title` and `command` when they
are absent. Known fixup kinds are:

```text
validate_workstreams
repair_sources
split_misc_workstream
manual_context
select_receipts
trim_receipts
split_broad_workstream
ticket_context
code_context
manual_only_workstream
thin_workstream
```

`top_fixups`, `journal_suggestions`, `share_commands`, and `next_commands` are
operator guidance. Commands should be shown as suggestions and should not be run
without user confirmation in future UI or agent surfaces.

## Machine Actions

`actions` is the machine-readable action list for future local UI/TUI and
agent surfaces. It deduplicates the most useful repair, fixup, share, and next
commands into objects with:

```text
id
kind
label
command
writes
risk
```

Known action kinds are:

```text
repair_source
fixup
share_manager
share_public
next_command
```

Known risk values are:

```text
low
medium
high
```

`writes` identifies commands that may create or modify Shiplog artifacts. UI and
agent consumers should still require user confirmation before running any action.
