# shiplog

[![crates.io](https://img.shields.io/crates/v/shiplog.svg)](https://crates.io/crates/shiplog)
[![docs.rs](https://docs.rs/shiplog/badge.svg)](https://docs.rs/shiplog)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

> Compile your GitHub, GitLab, Jira, Linear, git, JSON, and manual activity into defensible self-review packets -- with receipts.

## Installation

```bash
cargo install shiplog
```

With optional LLM-assisted workstream clustering:

```bash
cargo install shiplog --features llm
```

**Prerequisites:** Rust 1.92+. Set `GITHUB_TOKEN` for GitHub ingestion, `GITLAB_TOKEN` for GitLab ingestion, `JIRA_TOKEN` for Jira ingestion, or `LINEAR_API_KEY` for Linear ingestion.

## Quick start

```bash
# Fast review-deadline path
shiplog intake --last-6-months --explain
shiplog open intake-report --latest
shiplog open packet --latest
```

For repeatable setup:

```bash
shiplog init
shiplog doctor
shiplog collect multi --last-6-months
shiplog review --latest
shiplog render --latest
```

For direct source collection:

```bash
shiplog collect github \
  --me \
  --last-6-months \
  --mode merged \
  --out ./out
```

Curate workstreams without hand-editing YAML:

```bash
shiplog workstreams list --run latest
shiplog workstreams create --run latest --title "Platform Reliability"
shiplog workstreams rename --run latest --from "acme/platform" --to "Platform Reliability"
shiplog workstreams move --run latest --event <event_id> --to "Platform Reliability"
shiplog workstreams split --run latest --from "Platform Reliability" --to "Auth Migration" --matching "auth|oauth|sso" --create
shiplog workstreams receipts --run latest --workstream "Platform Reliability"
shiplog workstreams receipt add --run latest --workstream "Platform Reliability" --event <event_id>
shiplog workstreams receipt remove --run latest --workstream "Platform Reliability" --event <event_id>
shiplog workstreams delete --run latest --workstream "old bucket" --move-to "Platform Reliability"
shiplog workstreams validate --run latest
```

Re-render and share:

```bash
shiplog render --latest
shiplog render --latest --receipt-limit 3 --appendix summary
shiplog render --latest --mode scaffold
shiplog render --latest --mode receipts
shiplog share verify manager --latest
```

Output goes to `out/<run_id>/` containing `packet.md`, `ledger.events.jsonl`, `coverage.manifest.json`, and optional redacted profiles. Share commands also write `profiles/<profile>/share.manifest.json` with the share profile, redaction-key source, coverage status, and SHA-256 checksums for the packet and optional zip. Use `--mode packet` for the default packet, `--mode scaffold` for prompts and evidence anchors, or `--mode receipts` for a dense audit view. Use `--receipt-limit <N>` to cap main-section receipts and `--appendix full|summary|none` to control appendix density; `--receipt-limit 0` shows no main-section receipts. The packet coverage block lists completed sources with event counts, skipped configured sources, and known gaps before the detailed coverage metadata.

## Commands

| Command | Description |
|---------|-------------|
| `init` | Create `shiplog.toml` and `manual_events.yaml` scaffold files |
| `doctor` | Check local config, enabled sources, token env vars, and output safety |
| `intake` | Best-effort review rescue path that collects usable sources, renders a packet, writes an intake report, and prints next actions |
| `config validate/explain/migrate` | Validate `shiplog.toml`, print resolved settings, or add version metadata |
| `journal add/list/edit` | Capture and correct factual manual evidence without hand-editing YAML |
| `cache stats/inspect/clean` | Inspect and safely clean source API cache databases |
| `collect <source>` | Fetch events from a source and generate packet artifacts |
| `collect multi` | Collect enabled sources from `shiplog.toml` into one merged packet |
| `render` | Re-render packet from existing ledger and workstreams |
| `share manager/public` | Render manager/public-safe outputs with fail-closed redaction-key checks |
| `share verify manager/public` | Preflight share readiness without writing share artifacts |
| `refresh <source>` | Re-fetch events while preserving curated `workstreams.yaml` |
| `review` | Inspect coverage, evidence debt, fixups, and next commands without writing artifacts |
| `workstreams list/validate/create/rename/move/split/receipts/receipt/delete` | Inspect, validate, and safely edit workstream curation |
| `runs list/show/compare` | Discover runs and inspect or compare their sources, counts, coverage, and paths |
| `open packet/workstreams/intake-report/out` | Open run artifacts, or print their paths when opening is unavailable |
| `merge` | Merge existing run directories into one packet |
| `import` | Import an existing run directory and re-render |
| `run <source>` | Legacy: collect + render in one shot |

Date-based sources accept `--since/--until`, `--last-6-months`, `--last-quarter`, or `--year <YYYY>`. If omitted, shiplog uses the last six months.

Use `shiplog render --latest` or `--run latest` to re-render the most recent run. `shiplog refresh --run-dir latest` refreshes that run while preserving curation.

Use `shiplog init --source github --source jira --dry-run` to preview a
source-specific scaffold without writing files.
Use `shiplog config validate` for a token-free config and local path check,
`shiplog config explain` to print resolved defaults and enabled source
settings, and `shiplog config migrate` to add `[shiplog] config_version = 1`
to older configs without changing source settings. Copy-adaptable configs live
in the repository `examples/configs/` directory; from the repository root, the
local fixture config can be checked with:

```bash
shiplog config validate --config examples/configs/local-git-json-manual.toml
shiplog config explain --config examples/configs/local-git-json-manual.toml
```

Use `shiplog doctor` before collection to check tokens, output safety,
identity, and source setup.

Use cache commands to inspect or clean source API cache databases without
touching packet outputs:

```bash
shiplog cache stats --out ./out
shiplog cache inspect --out ./out --source github
shiplog cache clean --out ./out --source github
shiplog cache clean --out ./out --source jira --older-than 30d --dry-run
```

`cache clean` removes expired entries by default. `--all` requires `--yes` and
only clears known source cache databases, not packets, ledgers, or workstreams.

GitHub and GitLab accept `--me` to infer the source user from `--token`,
`GITHUB_TOKEN`, or `GITLAB_TOKEN`; use `--user <login>` to pin the identity
explicitly.

## Sources

| Source | Description |
|--------|-------------|
| `github` | PR and review ingestion from GitHub API |
| `gitlab` | Merge request and review-note ingestion from GitLab API |
| `jira` | Issue ingestion from Jira API |
| `linear` | Issue ingestion from Linear API |
| `git` | Local git commit ingestion |
| `json` | Import from canonical JSONL event files |
| `manual` | Ingest non-GitHub work from a YAML events file |

## Key features

- **Receipts-first.** Every claim traces to fetched evidence. Missing data is explicitly flagged, never silently omitted.
- **Coverage tracking.** A coverage manifest documents API query windows, pagination limits, and gaps.
- **Deterministic redaction.** Three profiles (internal/manager/public) with keyed SHA-256 aliasing. Same key = same aliases across runs.
- **User-owned workstreams.** Auto-generated suggestions in `workstreams.suggested.yaml`; your curated `workstreams.yaml` is never overwritten.
- **SQLite API cache.** GitHub, GitLab, Jira, and Linear API responses are cached locally to avoid redundant requests on re-runs.
- **Zip bundles.** Package output as a zip archive with SHA256 checksum manifests using `--zip`.

## Redaction

Provide a key to generate redacted packets:

```bash
shiplog render --latest --redact-key my-stable-secret
```

This produces `profiles/manager/packet.md` (context preserved, details stripped) and `profiles/public/packet.md` (repos and workstreams aliased, sensitive fields removed).

## Links

- [Repository](https://github.com/EffortlessMetrics/shiplog) -- Full README, architecture, and crate descriptions.
- [Review-cycle guide](https://github.com/EffortlessMetrics/shiplog/blob/main/docs/guides/review-cycle.md) -- The fastest path from setup to a curated, share-safe packet.
- [CHANGELOG](https://github.com/EffortlessMetrics/shiplog/blob/main/CHANGELOG.md) -- Release history.
- [ROADMAP](https://github.com/EffortlessMetrics/shiplog/blob/main/ROADMAP.md) -- What is planned and what is out of scope.
- [CONTRIBUTING](https://github.com/EffortlessMetrics/shiplog/blob/main/CONTRIBUTING.md) -- How to contribute.

## License

Dual licensed under [MIT](https://github.com/EffortlessMetrics/shiplog/blob/main/LICENSE-MIT) OR [Apache-2.0](https://github.com/EffortlessMetrics/shiplog/blob/main/LICENSE-APACHE), at your option.
