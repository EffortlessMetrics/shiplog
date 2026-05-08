# Review-cycle guide

This guide is the short path from an empty directory to a defensible review
packet. Use it when you want a practical self-review artifact, not a tour of
every option.

shiplog keeps the workflow simple:

```text
collect -> curate -> render
```

It writes one run directory with the packet, event ledger, coverage manifest,
workstream files, and optional share bundles.

## 5-minute packet

When the review form is already open, use intake first. It creates starter local
files if needed, collects whatever configured sources are usable, records skipped
sources as coverage warnings, renders the packet, and prints the review next
steps. It also prints a readiness summary with good signals, attention items,
and copy-ready next commands. The same checklist is written to
`intake.report.md`, with a structured `intake.report.json` beside it:

```bash
shiplog intake --last-6-months
shiplog open packet --latest
```

If setup is unclear, add `--explain`:

```bash
shiplog intake --last-6-months --explain
```

That prints why each source was included or skipped, such as a missing token,
missing local git repo, or fixture files that were found, and includes repair
hints without printing token values.

For a more deliberate setup pass, initialize local scaffolding, check setup,
then collect enabled sources from `shiplog.toml`:

```bash
shiplog init
shiplog doctor
shiplog collect multi --last-6-months
shiplog review --latest
shiplog runs list
shiplog open packet --latest
```

By default, shiplog writes to `./out`, uses the last six months, renders the
internal packet, and records source gaps in coverage warnings instead of hiding
them. Tokens stay in environment variables such as `GITHUB_TOKEN`,
`GITLAB_TOKEN`, `JIRA_TOKEN`, `LINEAR_API_KEY`, and `SHIPLOG_REDACT_KEY`.

## 30-minute curated packet

After collection, inspect the suggested workstreams and make safe edits through
the CLI. YAML remains the durable format, but you do not need to hand-edit it
for common changes.

```bash
shiplog review --latest
shiplog review fixups --latest
shiplog workstreams list --run latest
shiplog workstreams rename --run latest --from "acme/platform" --to "Platform Reliability"
shiplog workstreams move --run latest --event <event_id> --to "Platform Reliability"
shiplog workstreams receipts --run latest --workstream "Platform Reliability"
shiplog workstreams validate --run latest
shiplog render --latest --mode scaffold
shiplog render --latest --receipt-limit 3 --appendix summary
```

The scaffold mode gives prompts and evidence anchors. It does not write your
performance narrative for you.

Use `shiplog review fixups --latest` when you only want the top curation
actions as existing commands. It is read-only and ranks setup repair,
workstream validation, outcome notes, receipt inspection, and broad-bucket
splits without scoring performance.
Add `--commands-only` when you want a terse copy/paste list with no prose.

## Weekly upkeep

Use weekly review after a refresh or collection run when you want a short,
read-only checkpoint before review season:

```bash
shiplog review weekly
shiplog journal add \
  --title "Debugged customer import failure" \
  --workstream "Customer Reliability" \
  --date 2026-05-08
shiplog journal list
shiplog journal edit --id manual-2026-05-08-debugged-customer-import-failure \
  --impact "Identified the upstream export shape before the next import window"
```

The weekly view shows latest evidence counts, source gaps, evidence debt, and
next commands. It does not write packets or mutate workstreams.

The default internal packet starts with coverage and gaps, then uses a
receipt-summary appendix so you can see what was included, skipped, or
incomplete before refining claims. Rendering with `--bundle-profile manager`
defaults to fewer main receipts with the same summary appendix. Rendering with
`--bundle-profile public` defaults to the lowest density and omits the appendix
unless you opt back in. Use `--appendix full` when you want the dense audit
appendix.

## Capture missing work

Use `journal add` when the important work was not captured by code, ticket, or
review APIs. It appends to `manual_events.yaml` so the next `collect multi`
includes the evidence through the normal manual source path.

```bash
shiplog journal add \
  --date 2026-05-08 \
  --title "Debugged customer import incident" \
  --workstream "Customer Reliability" \
  --impact "Prevented repeat failure before the next import window" \
  --receipt ticket=https://example.invalid/ticket/OPS-123
```

Keep journal entries factual. Use them to capture evidence, context, receipts,
and impact prompts while the details are still fresh.

Use `shiplog journal list` to inspect manual evidence without opening YAML. Add
`--workstream` or repeat `--tag` when you only want one slice.
Use `shiplog journal edit` to correct a rushed title, workstream, impact note,
tags, receipts, or date without hand-editing the file.
When you pass `--tag` or `--receipt`, the supplied values replace that entry's
existing tag or receipt list.

When review finds a broad workstream with receipts but no manual outcome note,
it prints a `shiplog journal add` command with the workstream already filled in.

## Manager-safe packet

The internal profile can render without a redaction key. Manager and public
profiles fail closed: they require an explicit key or `SHIPLOG_REDACT_KEY`.
When a key is missing, the CLI prints the exact env var to set and the profile
to rerun.

```bash
export SHIPLOG_REDACT_KEY=replace-with-a-stable-secret
shiplog share manager --latest --zip
```

Use the same stable key across review cycles if you want deterministic aliases
to stay consistent across packets.

## Public portfolio packet

Public packets are the most restrictive share profile. Use them only when you
expect the output to leave your organization.

```bash
shiplog share public --latest --zip
```

Review the public packet before sharing it. Redaction protects identifiers and
sensitive fields; it does not decide whether a receipt is appropriate to share.

## Multi-source packet

Use `shiplog.toml` for repeatable multi-source collection. Start from one of the
example configs, edit identities and instances, then validate it:

```bash
shiplog config validate --config shiplog.toml
shiplog config explain --config shiplog.toml
shiplog doctor --config shiplog.toml
shiplog collect multi --config shiplog.toml --last-6-months
```

For source fields, defaults, env vars, and versioning rules, see
[docs/config-reference.md](../config-reference.md).

Partial source failures are recorded in the merged coverage manifest. A missing
Jira token, for example, should show up as a skipped source or warning rather
than a silent hole in the packet.

If you do not know the exact Jira account ID or Linear user ID to configure,
ask the provider API before collecting:

```bash
shiplog identify jira --instance company.atlassian.net --auth-user you@example.com
shiplog identify linear
```

## Troubleshooting source gaps

Use these commands when the packet looks smaller than expected:

```bash
shiplog doctor
shiplog review --latest
shiplog review weekly
shiplog review fixups --latest
shiplog review --latest --strict
shiplog runs show --run latest
shiplog runs compare --from review-2025-h2 --to latest
shiplog cache stats --out ./out
shiplog cache inspect --out ./out --source github
shiplog cache clean --out ./out --source github --older-than 30d --dry-run
```

Use `--strict` when you want review rehearsal or CI to fail if the run still
has evidence debt, such as skipped sources, partial coverage, missing receipt
anchors, broad workstreams, or validation errors.

`cache clean` removes cache entries from known source API databases. It does not
delete packets, ledgers, coverage manifests, bundles, or workstream files.

## Fixture-safe rehearsal

From this repository, you can run a no-network rehearsal against checked-in
fixtures:

```bash
shiplog init --source json --source manual --dry-run
shiplog journal add --events ./manual_events.yaml --date 2026-05-08 --title "Documented fixture rehearsal" --workstream "Docs" --dry-run
shiplog config validate --config examples/configs/local-git-json-manual.toml
shiplog config explain --config examples/configs/local-git-json-manual.toml
shiplog doctor --config examples/configs/local-git-json-manual.toml
shiplog collect --out ./out/docs-fixture multi --config examples/configs/local-git-json-manual.toml
shiplog runs list --out ./out/docs-fixture
shiplog review --out ./out/docs-fixture --latest
shiplog workstreams list --out ./out/docs-fixture --run latest
shiplog render --out ./out/docs-fixture --latest --mode scaffold
shiplog open packet --out ./out/docs-fixture --latest --print-path
```

That path exercises local git, JSON fixture events, and manual YAML fixture
events without calling live external APIs.
