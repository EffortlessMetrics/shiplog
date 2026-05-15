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
shiplog report export-agent-pack --latest --output agent-pack.json
shiplog repair plan --latest
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

Current writers also include optional top-level `actions`, `repair_items`, and
`packet_quality`. Older v1 reports may not have them, so readers should treat
these fields as absent rather than invalid when loading historical reports.

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

Current writers include canonical source identity fields on every source-facing
entry:

```text
source
source_key
source_label
```

`source_key` is the machine join key. It uses the canonical vocabulary:

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

`source_label` is display text for Markdown, terminal output, and reviewer UI.
For v1 compatibility, `source` is retained and mirrors `source_key` in reports
written by current shiplog versions. Older v1 reports may only have `source`,
and may use display labels there. Readers should prefer `source_key` when it is
present, fall back to normalizing `source` when it is absent, and display
`source_label` when it is present.

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

## Repair Items

`repair_items` is the machine-readable repair queue for future local UI/TUI,
agent, and `shiplog repair` surfaces. `shiplog repair plan --latest` renders
this queue from the latest report. Each item is derived from existing intake
report receipts. Readers must not re-query providers or scrape
`intake.report.md` to decide whether a repair is real.

Each item includes:

```text
repair_id
repair_key
kind
reason
action
clears_when
receipt_refs
```

Source-owned repairs also include:

```text
source_key
source_label
```

Known repair item kinds are:

```text
manual_evidence_missing
source_skipped_configuration
source_freshness_stale
source_cached_only
evidence_debt_open
share_redaction_required
artifact_missing_or_unopened
```

`action.kind` uses this vocabulary:

```text
journal_add
configure_source
rerun_intake
open_artifact
no_safe_action
```

`action.command` is optional. When present, it is display-and-copy guidance and
still requires explicit user confirmation. Provider setup guidance may name an
environment variable or config key, but it must not include secret values.

`receipt_refs[].field` points at the report receipt fields that justify the
item. Known receipt fields are:

```text
source_decisions
source_freshness
repair_sources
needs_attention
evidence_debt
top_fixups
journal_suggestions
next_commands
actions
artifacts
```

## Packet Quality

`packet_quality` is the optional review-ready packet quality object. It is
derived from existing report receipts and durable artifacts. Readers must not
re-query providers or scrape `intake.report.md` to recover packet quality.

Current writers populate packet readiness, evidence strength, and conservative
claim candidates when receipts support them. `share_posture` is present as an
array but may be empty until that later implementation slice lands.

When present, `packet_quality` includes:

```text
packet_readiness
evidence_strength
claim_candidates
share_posture
```

`packet_readiness` includes:

```text
status
summary
reasons
next_actions
```

Known packet readiness statuses are:

```text
ready
ready_with_caveats
needs_evidence
needs_context
blocked
```

`evidence_strength` is a receipt-backed list of scoped assessments. It is not a
productivity score, performance rating, ranking, or employee-quality signal.
Each item includes:

```text
scope
status
reason
receipt_refs
```

Known evidence strength statuses are:

```text
strong
partial
manual_only
source_skipped
needs_context
```

`receipt_refs[].field` points at report receipts such as:

```text
included_sources
source_freshness
repair_items
needs_attention
evidence_debt
artifacts
```

Claim candidates are evidence scaffolds, not generated performance-review
prose. Each candidate includes:

```text
claim_id
title
evidence_strength
supporting_receipt_refs
missing_context_prompts
safe_profiles
```

Candidates may also include `supporting_sources`, `supporting_repair_keys`, and
`caveats`. They must not invent impact, beneficiaries, or accomplishments.

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

## Agent Pack Export

Use `shiplog report export-agent-pack` when a downstream tool needs a compact
view of the report's summary, gaps, repairs, fixups, actions, share status, and
artifact paths:

```bash
shiplog report export-agent-pack --latest --output agent-pack.json
```

The export validates `intake.report.json` first and then writes a derived
`agent-pack.json` using the schema documented in
[agent-pack-v1.md](agent-pack-v1.md). It does not rewrite packet, ledger,
coverage, workstream, or share artifacts.
