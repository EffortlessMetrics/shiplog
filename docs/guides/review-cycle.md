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

Start with local scaffolding, check setup, then collect enabled sources from
`shiplog.toml`:

```bash
shiplog init
shiplog doctor
shiplog collect multi --last-6-months
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

## Manager-safe packet

The internal profile can render without a redaction key. Manager and public
profiles fail closed: they require an explicit key or `SHIPLOG_REDACT_KEY`.

```bash
export SHIPLOG_REDACT_KEY=replace-with-a-stable-secret
shiplog render --latest --redact-key "$SHIPLOG_REDACT_KEY" --zip --bundle-profile manager
```

Use the same stable key across review cycles if you want deterministic aliases
to stay consistent across packets.

## Public portfolio packet

Public packets are the most restrictive share profile. Use them only when you
expect the output to leave your organization.

```bash
shiplog render --latest --redact-key "$SHIPLOG_REDACT_KEY" --zip --bundle-profile public
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
shiplog runs show --run latest
shiplog cache stats --out ./out
shiplog cache inspect --out ./out --source github
shiplog cache clean --out ./out --source github --older-than 30d --dry-run
```

`cache clean` removes cache entries from known source API databases. It does not
delete packets, ledgers, coverage manifests, bundles, or workstream files.

## Fixture-safe rehearsal

From this repository, you can run a no-network rehearsal against checked-in
fixtures:

```bash
shiplog init --source json --source manual --dry-run
shiplog config validate --config examples/configs/local-git-json-manual.toml
shiplog config explain --config examples/configs/local-git-json-manual.toml
shiplog doctor --config examples/configs/local-git-json-manual.toml
shiplog collect --out ./out/docs-fixture multi --config examples/configs/local-git-json-manual.toml
shiplog runs list --out ./out/docs-fixture
shiplog workstreams list --out ./out/docs-fixture --run latest
shiplog render --out ./out/docs-fixture --latest --mode scaffold
shiplog open packet --out ./out/docs-fixture --latest --print-path
```

That path exercises local git, JSON fixture events, and manual YAML fixture
events without calling live external APIs.
