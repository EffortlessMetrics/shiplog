# Guided setup and doctor guide

Use this guide before a first intake, after changing `shiplog.toml`, or when a
repair/share command looks blocked by setup. The goal is to make setup state
explicit before you spend time on an intake run or try a repair command that
cannot succeed.

This is the canonical front door for new setups. Use the repair and share
guides after doctor has told you whether the next command is safe.

For the current operational proof ledger, see the
[`Setup-readiness dogfood matrix`](../product/setup-readiness-dogfood-matrix.md).

The front-door loop is:

```bash
shiplog init --guided
shiplog doctor --setup
shiplog sources status
shiplog doctor --setup --json
shiplog intake --last-6-months --explain
shiplog repair plan --latest
shiplog share explain manager --latest
```

`doctor --setup`, `sources status`, and `share explain` are read-only surfaces.
They do not query providers, render share packets, mutate provider records, or
write profile artifacts. `init --guided`, `intake`, `journal add`, and
`share manager|public` write local files.

## Start from an empty directory

Create the local setup files:

```bash
shiplog init --guided
```

Guided init is non-interactive. It writes:

- `shiplog.toml`
- `manual_events.yaml`

When the current directory is a Git repository, guided init enables local Git.
When `ledger.events.jsonl` and `coverage.manifest.json` already exist, it
enables JSON import. Token-backed providers are scaffolded but left disabled
until you configure credentials and identity fields.

Read setup state before intake:

```bash
shiplog doctor --setup
```

Read source-only state when you do not want share or credential noise:

```bash
shiplog sources status
```

Read the same setup state as JSON when an agent or script needs to decide the
next safe command without scraping terminal text:

```bash
shiplog doctor --setup --json
```

If doctor reports `Needs setup`, read the blocked/unavailable groups first.
That state is useful. It means Shiplog found setup work before a later command
failed or produced a caveated packet.

## Read the doctor output

`shiplog doctor --setup` groups setup facts:

- `Blocked`: malformed local files, unsupported config, missing redaction setup
  for share rendering, or internally inconsistent setup.
- `Unavailable`: missing local files or credentials for enabled sources.
- `Ready`: setup pieces that are usable.
- `Disabled`: optional sources or profiles that are intentionally not active.
- `Unknown`: setup state that cannot be proven without network access or
  reading a secret value.

Each item names the reason and, when possible, a next action. The action label
says whether the command is read-only or writes.

`--json` emits the same setup model with stable keys, status strings, reasons,
receipt refs, and `writes` flags. It still does not query providers, render
share packets, mutate config, or print secret values. A `Needs setup` or
`Blocked` result remains a non-zero command result after the JSON is printed, so
agents should parse stdout and treat the exit status as the stop signal.
The schema is documented in
[`Setup Readiness v1`](../schemas/setup-readiness-v1.md).

Examples:

```text
Manual journal [blocked; enabled]: manual_events.yaml malformed
  Next (read-only): shiplog doctor --setup

GitHub token [unavailable; enabled]: GITHUB_TOKEN not set
  Next (read-only): set GITHUB_TOKEN

Manager share [blocked; enabled]: SHIPLOG_REDACT_KEY not set
  Next (read-only): set SHIPLOG_REDACT_KEY
```

Do not treat a missing optional provider token as fatal. Decide whether to set
the token, disable the source, or continue local-only.

## Local-only mode

Use this when you want a packet from local Git plus local manual evidence.

```bash
shiplog init --guided
shiplog doctor --setup
shiplog sources status
shiplog intake --last-6-months --explain
```

Expected setup:

- Local Git is `ready` when the current directory is a readable Git repository.
- Manual journal is `ready` when `manual_events.yaml` has the current schema.
- GitHub, GitLab, Jira, and Linear are disabled until configured.

If local Git is blocked, `sources status` tells you whether the path is missing,
not a directory, or not a readable Git repo. Fix that before relying on packet
quality.

## Manual-only mode

Use this when provider tokens are unavailable or the review needs human context
first.

```bash
shiplog init --guided
shiplog doctor --setup
shiplog intake --last-6-months --explain
shiplog repair plan --latest
```

If the first intake has no manual events, the repair plan should route through
the report-derived journal flow:

```bash
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
```

If `manual_events.yaml` is malformed, do not run `journal add --from-repair`
yet. Run:

```bash
shiplog doctor --setup
```

Fix the manual journal schema, rerun doctor, then resume the repair loop.

## Token-backed GitHub mode

Use this when you want GitHub receipts in the packet.

In `shiplog.toml`, enable GitHub and configure either `user` or `me = true`.
Then set the token in your shell:

```bash
export GITHUB_TOKEN=...
shiplog sources status --source github
shiplog intake --last-6-months --explain
```

On PowerShell:

```powershell
$env:GITHUB_TOKEN='...'
shiplog sources status --source github
shiplog intake --last-6-months --explain
```

`sources status` only checks setup state. It does not call the GitHub API. A
present token means the credential prerequisite exists; intake still owns the
actual evidence fetch and receipts.

## Manager-share-ready mode

Manager share rendering is blocked until redaction is configured.

```bash
shiplog doctor --setup
shiplog share explain manager --latest
```

If doctor says `SHIPLOG_REDACT_KEY not set`, set a stable key before rendering:

```bash
export SHIPLOG_REDACT_KEY=replace-with-a-stable-secret
shiplog doctor --setup
shiplog share explain manager --latest
shiplog share verify manager --latest
shiplog share manager --latest
```

On PowerShell:

```powershell
$env:SHIPLOG_REDACT_KEY='replace-with-a-stable-secret'
shiplog doctor --setup
shiplog share explain manager --latest
shiplog share verify manager --latest
shiplog share manager --latest
```

Use `share explain manager` before rendering. It tells you what the manager
profile includes, removes, blocks, or caveats without writing
`profiles/manager/packet.md` or `share.manifest.json`.

## Public-share-cautious mode

Public share needs the strictest posture. Treat doctor as setup readiness and
`share explain public` as share posture.

```bash
shiplog doctor --setup
shiplog share explain public --latest
shiplog share verify public --latest --strict
```

Doctor can tell you that the redaction key is missing. It cannot prove the
public packet is safe before a public profile has been rendered and strictly
verified. After rendering, inspect the packet before sharing outside your
organization.

## What doctor is not

Doctor is not a dry-run intake.

It may inspect:

- `shiplog.toml`
- `manual_events.yaml`
- local Git paths
- environment-variable presence
- share redaction setup

It must not:

- query provider APIs by default
- mutate provider records
- render share artifacts
- scrape `packet.md` as a machine source
- generate performance-review prose

The boundary is:

```text
doctor explains setup readiness
intake produces evidence receipts
repair consumes intake receipts
share explain consumes report and share receipts
```

When in doubt, run the read-only command first:

```bash
shiplog doctor --setup
shiplog sources status
shiplog share explain manager --latest
```
