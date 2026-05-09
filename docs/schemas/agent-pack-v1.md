# Agent Pack v1

`agent-pack.json` is a compact export derived from a validated
`intake.report.json`. It is meant for future local UI/TUI surfaces, agent
helpers, and support/debug flows that need the review-rescue control surface
without parsing Markdown.

The v1 schema lives at:

```text
contracts/schemas/agent-pack.v1.schema.json
```

Export a pack with:

```bash
shiplog report export-agent-pack --latest --output agent-pack.json
shiplog report export-agent-pack --path out/<run>/intake.report.json
```

When `--output` is omitted, the command writes JSON to stdout. The command
validates the source intake report first and does not rewrite packet, ledger,
coverage, workstream, or share artifacts.

## Compatibility

The top-level `schema_version` field is required and must be `1`.

The following top-level fields are stable for v1 consumers:

```text
schema_version
source_report
run
summary
gaps
repairs
fixups
journal_suggestions
share_status
actions
next_commands
artifacts
```

Consumers should treat display strings, command strings, and ordering as
best-effort operator guidance. Use `schema_version`, `source_report`, `run`,
`summary`, `gaps`, `repairs`, `fixups`, `share_status`, `actions`, and
`artifacts` for control flow.

## Source Report

`source_report` points back to the validated `intake.report.json` and Markdown
report:

```text
schema_version
path
markdown_path
```

The source report remains the full durable record. The agent pack is a derived
view that groups the fields most useful for UI and agent consumers.

## Summary And Gaps

`summary` contains counts for included sources, skipped sources, evidence debt,
repair actions, fixups, journal suggestions, share commands, machine actions,
and artifacts.

`gaps` carries:

```text
needs_attention
skipped_sources
evidence_debt
```

These fields describe packet quality and repair state. They must not be used as
productivity metrics or person scores.

## Repairs, Fixups, And Actions

`repairs` mirrors `intake.report.json` repair items, including stable repair
`kind` values when present. `fixups` mirrors top curation actions, including
stable fixup IDs and kinds when present.

`actions` mirrors the machine-readable action list from the intake report:

```text
id
kind
label
command
writes
risk
```

Future UI or agent consumers should show these as suggested actions and require
explicit user confirmation before running any command.

## Share Status

`share_status` contains share commands plus booleans indicating whether manager
or public share commands are available in the source report. This is not a
privacy guarantee. Use `shiplog share verify manager --latest` or
`shiplog share verify public --latest --strict` before producing shareable
output.

## Secrets

The agent pack must not include token values, redaction key material,
passwords, or secret data. It may include environment variable names and
copy-ready commands that contain placeholders such as `export JIRA_TOKEN=...`.
