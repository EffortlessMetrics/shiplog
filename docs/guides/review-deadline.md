# Review-deadline guide

Use this guide when the review form is due soon and your work is scattered
across source systems. The goal is a defensible first packet, not a perfect
archive.

shiplog should help you answer:

```text
what was collected
what failed
what is thin
what context you need to add
what is safe to share
```

It should not invent impact claims or turn receipts into a productivity score.

## 5-minute packet

Start with intake. It is best-effort: one working source is enough to produce a
packet, and missing sources are recorded as skipped-source warnings.

```bash
shiplog intake --last-6-months --explain
shiplog open intake-report --latest
shiplog open packet --latest
```

If your config has a saved review window, use it instead of retyping dates:

```bash
shiplog intake --period review-cycle --explain
shiplog review --period review-cycle
```

Read the top of `intake.report.md` first. It is the durable checklist for the
run after terminal output scrolls away. It shows readiness, included sources,
skipped sources, repair hints, evidence debt, fixups, journal suggestions, share
commands, and artifact paths.

Treat the packet as useful if it has at least one honest source and clear gaps.
Do not wait for every provider token to work before writing down what you can
prove.

## 15-minute cleanup

Use fixups to find the highest-value curation actions.

```bash
shiplog review fixups --latest
shiplog review fixups --latest --commands-only
shiplog review fixups --latest --journal-template
```

The first command explains the issues. The second prints only copy-ready
commands. The third prints only factual `journal add` templates for missing
human context; replace the placeholder description before adding the entry.
Typical fixups are:

- select anchor receipts for a workstream;
- add manual outcome context for a broad workstream;
- split a large miscellaneous bucket;
- rerun a source after adding a token.

Keep fixups factual. If shiplog suggests a journal command, fill in the real
context before you rely on the packet.

## 30-minute manager-safe packet

Manager and public packets fail closed. Set a stable redaction key before
rendering a share profile.

```bash
export SHIPLOG_REDACT_KEY=replace-with-a-stable-secret
shiplog share verify manager --latest
shiplog share manager --latest --zip
```

`share verify` is read-only. Use it before writing a share packet when you want
to check that the run has coverage, visible gaps, and the redaction key needed
for the selected audience.

For a public packet, use the stricter profile and read the output before
publishing it:

```bash
shiplog share verify public --latest --strict
shiplog share public --latest --zip
```

Strict public verification scans the rendered public view for obvious raw URLs
and original names. Redaction reduces accidental disclosure; it does not decide
whether a receipt is appropriate to share or guarantee perfect privacy.

## When a source fails

Do not discard the run. Intake records skipped sources in coverage and in the
intake report.

Look for the `Repair sources` section:

```text
Jira:
  export JIRA_TOKEN=...
  shiplog identify jira --auth-user you@example.com
  shiplog intake --last-6-months --explain
```

The commands show what to set or rerun without printing secret values. Fix one
source at a time, then rerun intake. Reruns create a new run and reuse prior
curated `workstreams.yaml` when available, leaving earlier curation and
`manual_events.yaml` untouched.

## When evidence is thin

Use review to inspect packet quality without writing artifacts:

```bash
shiplog review --latest
shiplog review --latest --strict
```

Evidence debt is about packet quality, not worker quality. Examples include
skipped sources, partial coverage, missing selected receipts, too many selected
receipts, broad or miscellaneous workstreams, thin workstreams, code-only
workstreams, ticket-only workstreams, manual-only workstreams, and workstreams
that need manual outcome context. Each finding should point to an existing
command such as `workstreams receipts`, `workstreams split`, `journal add`,
`journal list`, or `doctor`.

## Add missing context

Use the journal commands instead of hand-editing YAML under deadline pressure.

```bash
shiplog journal add \
  --date 2026-05-08 \
  --title "Debugged customer import incident" \
  --workstream "Customer Reliability" \
  --impact "Identified the upstream export shape before the next import window"

shiplog journal list
shiplog journal edit --id manual-2026-05-08-debugged-customer-import-incident \
  --impact "Identified the upstream export shape before the next import window"
```

Journal entries are manual evidence. Keep them factual and attach receipts when
you have them:

```bash
shiplog journal add \
  --date 2026-05-08 \
  --title "Validated rollback checklist" \
  --workstream "Platform Reliability" \
  --receipt doc=https://example.invalid/rollback-checklist
```

## Safe sharing checklist

Before sending a packet:

```bash
shiplog share verify manager --latest
shiplog share manager --latest
shiplog open packet --latest
```

Check that:

- coverage and skipped sources are visible;
- the selected audience profile is the one you meant to render;
- the packet has the receipts you want to rely on;
- manager/public output used a real redaction key;
- public output does not include receipts you should keep private.

## No-network rehearsal

From this repository, you can rehearse the deadline path against checked-in
fixtures:

```bash
scripts/demo-review-rescue.sh --out ./out/deadline-fixture
```

On Windows PowerShell:

```powershell
pwsh -File .\scripts\demo-review-rescue.ps1 -Out .\out\deadline-fixture
```

The demo script runs the same rescue loop:

```bash
shiplog intake --out ./out/deadline-fixture --config examples/configs/local-git-json-manual.toml --no-open --explain
shiplog open intake-report --out ./out/deadline-fixture --latest --print-path
shiplog review fixups --out ./out/deadline-fixture --latest --commands-only
shiplog share verify manager --out ./out/deadline-fixture --latest --redact-key fixture-key
```

These commands do not call live provider APIs. They exercise local git, JSON,
and manual fixture inputs while preserving the same rescue-loop shape.
