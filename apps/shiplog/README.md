# shiplog

[![crates.io](https://img.shields.io/crates/v/shiplog.svg)](https://crates.io/crates/shiplog)
[![docs.rs](https://docs.rs/shiplog/badge.svg)](https://docs.rs/shiplog)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

> Compile your GitHub activity into defensible self-review packets -- with receipts.

## Installation

```bash
cargo install shiplog
```

With optional LLM-assisted workstream clustering:

```bash
cargo install shiplog --features llm
```

**Prerequisites:** Rust 1.92+, a `GITHUB_TOKEN` environment variable for GitHub ingestion.

## Quick start

```bash
# 1. Collect events from GitHub
shiplog collect github \
  --user your-username \
  --since 2025-07-01 \
  --until 2026-01-01 \
  --mode merged \
  --out ./out

# 2. Curate workstreams
#    Copy workstreams.suggested.yaml to workstreams.yaml and edit.
#    shiplog never overwrites workstreams.yaml.
cp out/<run_id>/workstreams.suggested.yaml out/<run_id>/workstreams.yaml

# 3. Re-render the packet with your curated workstreams
shiplog render --run-dir out/<run_id>
```

Output goes to `out/<run_id>/` containing `packet.md`, `ledger.events.jsonl`, `coverage.manifest.json`, and optional redacted profiles.

## Commands

| Command | Description |
|---------|-------------|
| `collect <source>` | Fetch events from a source and generate packet artifacts |
| `render` | Re-render packet from existing ledger and workstreams |
| `refresh <source>` | Re-fetch events while preserving curated `workstreams.yaml` |
| `import` | Import an existing run directory and re-render |
| `run <source>` | Legacy: collect + render in one shot |

## Sources

| Source | Description |
|--------|-------------|
| `github` | PR and review ingestion from GitHub API |
| `json` | Import from canonical JSONL event files |
| `manual` | Ingest non-GitHub work from a YAML events file |

## Key features

- **Receipts-first.** Every claim traces to fetched evidence. Missing data is explicitly flagged, never silently omitted.
- **Coverage tracking.** A coverage manifest documents API query windows, pagination limits, and gaps.
- **Deterministic redaction.** Three profiles (internal/manager/public) with HMAC-SHA256 aliasing. Same key = same aliases across runs.
- **User-owned workstreams.** Auto-generated suggestions in `workstreams.suggested.yaml`; your curated `workstreams.yaml` is never overwritten.
- **SQLite API cache.** GitHub API responses are cached locally to avoid redundant requests on re-runs.
- **Zip bundles.** Package output as a zip archive with SHA256 checksum manifests using `--zip`.

## Redaction

Provide a key to generate redacted packets:

```bash
shiplog render --run-dir out/<run_id> --redact-key my-stable-secret
```

This produces `profiles/manager/packet.md` (context preserved, details stripped) and `profiles/public/packet.md` (repos and workstreams aliased, sensitive fields removed).

## Links

- [Repository](https://github.com/EffortlessMetrics/shiplog) -- Full README, architecture, and crate descriptions.
- [CHANGELOG](https://github.com/EffortlessMetrics/shiplog/blob/main/CHANGELOG.md) -- Release history.
- [ROADMAP](https://github.com/EffortlessMetrics/shiplog/blob/main/ROADMAP.md) -- What is planned and what is out of scope.
- [CONTRIBUTING](https://github.com/EffortlessMetrics/shiplog/blob/main/CONTRIBUTING.md) -- How to contribute.

## License

Dual licensed under [MIT](https://github.com/EffortlessMetrics/shiplog/blob/main/LICENSE-MIT) OR [Apache-2.0](https://github.com/EffortlessMetrics/shiplog/blob/main/LICENSE-APACHE), at your option.
