# shiplog

[![CI](https://github.com/EffortlessMetrics/shiplog/actions/workflows/ci.yml/badge.svg)](https://github.com/EffortlessMetrics/shiplog/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/shiplog.svg)](https://crates.io/crates/shiplog)
[![docs.rs](https://docs.rs/shiplog/badge.svg)](https://docs.rs/shiplog)
[![MSRV](https://img.shields.io/badge/MSRV-1.92-blue.svg)](https://blog.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

> Compile your GitHub activity into defensible self-review packets -- with receipts.

## Why shiplog?

Performance reviews ask "what did you ship?" shiplog answers that question with evidence.

**Receipts-first.** Every claim in a shiplog packet traces back to fetched data -- merged PRs, reviews, manual entries. Missing data is explicitly flagged in a coverage manifest, never silently omitted.

**Coverage-first.** shiplog tracks what it queried and what might be incomplete. The coverage manifest documents API query windows, pagination limits hit, and gaps -- so you know exactly what the packet does and does not cover.

**Safe sharing.** Three deterministic redaction profiles (internal, manager, public) let you share packets at the right level of detail. Same key + same input = same aliases across runs, powered by HMAC-SHA256.

shiplog is not an analytics dashboard. It is not AI-generated narrative. It produces evidence you curate and a packet you can defend.

## Who is this for?

- **Individual contributors** preparing self-reviews, promo packets, or brag documents with concrete shipping receipts.
- **Tech leads** compiling structured proof of what their team shipped during a review cycle.
- **Anyone** who wants a repeatable, auditable record of their GitHub activity over a time window.

## Features

- 🔍 **Multi-source ingestion** — GitHub API, GitLab API, Jira API, Linear API, local git commits, canonical JSONL, and manual YAML events
- 📊 **Automatic workstream clustering** — repo-based by default, optional LLM-assisted semantic grouping
- 🔒 **Deterministic HMAC-SHA256 redaction** — three profiles (internal / manager / public) with stable aliases
- ✅ **Coverage-first design** — every claim backed by receipts; gaps explicitly flagged in the coverage manifest
- 📦 **Zip bundling with checksums** — SHA256 manifest for integrity verification
- 🏗️ **Module-first clean architecture** — public crates for contracts, trust surfaces, adapters, and optional dependency boundaries

## Installation

### From crates.io (recommended)

```bash
cargo install shiplog
```

With optional LLM-assisted workstream clustering:

```bash
cargo install shiplog --features llm
```

### From source

```bash
git clone https://github.com/EffortlessMetrics/shiplog.git
cd shiplog
cargo install --path apps/shiplog
```

Developers working on shiplog itself can run directly:

```bash
cargo run -p shiplog -- <subcommand>
```

### Prerequisites

- Rust 1.92+
- A `GITHUB_TOKEN` environment variable for GitHub ingestion
- A `GITLAB_TOKEN` environment variable for GitLab ingestion
- A `JIRA_TOKEN` environment variable for Jira ingestion
- A `LINEAR_API_KEY` environment variable for Linear ingestion

## Quick start

shiplog follows a **collect → curate → render** workflow. You fetch raw events once, organize them into workstreams, then re-render as many times as you like without hitting the API again.

### 1. Collect events from GitHub

```bash
shiplog collect github \
  --user your-username \
  --last-6-months \
  --mode merged \
  --out ./out
```

This fetches merged PRs (and optionally reviews with `--include-reviews`) for the given user and time window. If you omit a date window, shiplog defaults to the last six months. Results go into `out/<run_id>/` with a JSONL event ledger, coverage manifest, and an initial packet.

### 2. Curate workstreams

shiplog generates `workstreams.suggested.yaml` with auto-grouped workstreams based on repository. Copy it to `workstreams.yaml` and edit:

```bash
cp out/<run_id>/workstreams.suggested.yaml out/<run_id>/workstreams.yaml
# Edit workstreams.yaml: rename groups, move PRs between workstreams, add narrative.
```

`workstreams.yaml` is yours. shiplog never overwrites it.

### 3. Re-render the packet

```bash
shiplog render --latest
```

This regenerates `packet.md` using your curated workstreams while preserving the original ledger and coverage data. Add `--redact-key <KEY>` to also generate manager and public profile packets.

## Output layout

```text
out/<run_id>/
  packet.md                       # Main self-review packet (internal profile)
  workstreams.yaml                # User-curated workstream definitions
  workstreams.suggested.yaml      # Auto-generated workstream suggestions
  ledger.events.jsonl             # Canonical, append-only event log
  coverage.manifest.json          # What was queried, completeness, gaps
  bundle.manifest.json            # File checksums for integrity verification
  profiles/
    manager/packet.md             # Redacted: keeps context, strips sensitive details
    public/packet.md              # Redacted: aliases repos/workstreams, strips fields
```

## Commands

| Command | Description |
|---------|-------------|
| `collect <source>` | Fetch events from a source and generate packet artifacts |
| `render` | Re-render packet from existing ledger and workstreams |
| `refresh <source>` | Re-fetch events while preserving curated `workstreams.yaml` |
| `import` | Import an existing run directory and re-render |
| `run <source>` | Legacy: collect + render in one shot |

Date-based sources accept explicit dates or a preset:

```bash
--since 2025-07-01 --until 2026-01-01
--last-6-months
--last-quarter
--year 2025
```

Explicit `--since/--until` dates take precedence. If no date window is provided,
shiplog uses `--last-6-months`.

Use `shiplog render --latest` or `--run latest` to re-render the most recent
run. `shiplog refresh --run-dir latest` refreshes the same run while preserving
curation.

### Sources

| Source | Description |
|--------|-------------|
| `github` | PR and review ingestion from GitHub API (with adaptive slicing and SQLite cache) |
| `gitlab` | Merge request and review-note ingestion from GitLab API (supports self-hosted instances) |
| `jira` | Issue ingestion from Jira API; `--user` is the assignee JQL value, and `--auth-user` is available when Basic Auth uses a different email/username |
| `linear` | Issue ingestion from Linear API; `--user-id` is the Linear user UUID, with upstream date/status/project filtering |
| `git` | Local git commit ingestion for `collect git`, `refresh git`, and `run git` |
| `json` | Import from canonical JSONL event files |
| `manual` | Ingest non-GitHub work from a YAML events file |

### Examples

```bash
# Refresh receipts while keeping curated workstreams
shiplog refresh github \
  --user your-username \
  --last-6-months \
  --run-dir out/20260115_143022 \
  --out ./out

# Import from pre-built JSON artifacts
shiplog collect json \
  --events ./ledger.events.jsonl \
  --coverage ./coverage.manifest.json \
  --out ./out

# Collect commits from a local git repository
shiplog collect git \
  --repo . \
  --year 2025 \
  --author you@example.com \
  --out ./out

# Refresh local git receipts while preserving curated workstreams
shiplog refresh git \
  --repo . \
  --year 2025 \
  --run-dir out/20260115_143022 \
  --out ./out

# Collect merge requests from GitLab
shiplog collect gitlab \
  --user your-gitlab-username \
  --last-6-months \
  --state merged \
  --instance gitlab.com \
  --include-reviews \
  --out ./out

# Collect assigned issues from Jira
shiplog collect jira \
  --user 712020:account-id \
  --auth-user you@example.com \
  --last-quarter \
  --status done \
  --instance company.atlassian.net \
  --out ./out

# Collect assigned issues from Linear
shiplog collect linear \
  --user-id 9cfb482a-81e3-4154-b5b9-2c805e70a02d \
  --last-quarter \
  --status done \
  --project OPS \
  --out ./out

# Collect manual (non-GitHub) events
shiplog collect manual \
  --events ./manual_events.yaml \
  --user your-username \
  --year 2025 \
  --out ./out
```

## Redaction and profiles

Generate redacted packets by providing a key:

```bash
shiplog render --latest --redact-key my-stable-secret
```

The key drives deterministic HMAC-SHA256 aliasing. Same key + same input = same aliases across runs.

| Field | Internal | Manager | Public |
|-------|----------|---------|--------|
| PR titles | Visible | Visible | Stripped |
| Repository names | Visible | Visible | Aliased (e.g., `repo-a7f3`) |
| URLs | Visible | Visible | Stripped |
| Workstream names | Visible | Visible | Aliased |
| Descriptions/details | Visible | Stripped | Stripped |
| Event counts and dates | Visible | Visible | Visible |

Bundle a specific profile as a zip:

```bash
shiplog render --latest --redact-key my-stable-secret --zip --bundle-profile manager
```

## Architecture

shiplog is a module-first Rust workspace following clean architecture (ports and adapters).
Public crates represent contracts, trust surfaces, real adapters, and optional dependency
boundaries; internal implementation seams live under their owning crates. See
[API_SURFACE.md](API_SURFACE.md) for the package boundary doctrine.

### Data flow

```text
GitHub API ─→ Ingestor ─→ Normalizer ─→ Clusterer ─→ Renderer ─→ Output
    │              │            │             │            │
    └── Cache      └── Schema   └── Workstreams └── Redact  └── Bundle
```

Events flow left-to-right through well-defined ports. Product boundaries are tested at the
crate level; smaller implementation seams stay as owner modules unless they earn an
external contract. Adapters depend on ports and schema, never the reverse.

### Crate map

```text
                    +------------------+
                    |   shiplog (CLI)  |
                    +--------+---------+
                             |
                    +--------v---------+
                    |  shiplog-engine   |
                    |  (orchestration)  |
                    +--------+---------+
                             |
          +------------------+------------------+
          |                  |                  |
  +-------v------+  +-------v------+  +--------v-------+
  | Ingest       |  | Process      |  | Output         |
  | - github     |  | - workstreams|  | - render-md    |
  | - git        |  | - redact     |  | - render-json  |
  | - json       |  | - cluster-llm|  | - bundle       |
  | - manual     |  | - team/merge |  |                |
  +--------------+  +--------------+  +----------------+
          |                  |                  |
          +------------------+------------------+
                             |
                    +--------v---------+
                    |   Foundation     |
                    | ports, schema,   |
                    | ids, coverage,   |
                    | cache            |
                    +------------------+
```

### Public Surface

| Crate | Role |
|-------|------|
| `shiplog` | CLI entrypoint (clap) |
| `shiplog-engine` | Orchestration: ingest, cluster, redact, render |
| `shiplog-ports` | Trait definitions: `Ingestor`, `Renderer`, `Redactor`, `WorkstreamClusterer` |
| `shiplog-schema` | Canonical event model, `EventKind`, manifests |
| `shiplog-ids` | Deterministic SHA256-based `EventId`, `RunId`, `WorkstreamId` |
| `shiplog-coverage` | Time windows, completeness tracking |
| `shiplog-cache` | SQLite-backed API response cache |
| `shiplog-ingest-github` | GitHub PR and review ingestion with adaptive slicing |
| `shiplog-ingest-git` | Local git commit ingestion for `collect git`, `refresh git`, and `run git` |
| `shiplog-ingest-json` | Import from canonical JSON artifacts |
| `shiplog-ingest-manual` | YAML-based manual event ingestion |
| `shiplog-ingest-gitlab` | GitLab MR and review ingestion adapter |
| `shiplog-ingest-jira` | Jira issue ingestion adapter |
| `shiplog-ingest-linear` | Linear issue ingestion adapter |
| `shiplog-workstreams` | Workstream clustering and user-curated YAML workflow |
| `shiplog-merge` | Multi-source merge helpers used by the engine |
| `shiplog-cluster-llm` | Optional LLM-assisted semantic clustering |
| `shiplog-team` | Team aggregation support |
| `shiplog-redact` | Deterministic HMAC-SHA256 redaction across three profiles |
| `shiplog-render-md` | Markdown packet renderer |
| `shiplog-render-json` | JSON output renderer |
| `shiplog-bundle` | Zip archives with SHA256 checksum manifests |
| `shiplog-testkit` | Shared test fixtures (not published) |

Internal implementation seams live under owner crates as modules. New boundaries
start as modules and are promoted to public crates only when they become stable
contracts, trust surfaces, real adapters, or heavy optional boundaries.
`publish = false` is reserved for dev-only packages.

## LLM clustering

By default, shiplog clusters events by repository. With the `llm` feature, you can enable semantic clustering via an OpenAI-compatible API:

```bash
# Install with LLM support
cargo install shiplog --features llm

# Use LLM clustering during collection
shiplog collect github \
  --user your-username \
  --last-6-months \
  --out ./out \
  --llm-cluster \
  --llm-api-key $SHIPLOG_LLM_API_KEY
```

LLM clustering is feature-gated and off by default. It falls back to repository-based clustering on failure. See `--llm-api-endpoint` and `--llm-model` for endpoint configuration.

## Documentation

- [CHANGELOG](CHANGELOG.md) -- Release history and migration notes.
- [API_SURFACE](API_SURFACE.md) -- Public crate boundary doctrine.
- [ROADMAP](ROADMAP.md) -- What is planned, what is next, and what is out of scope.
- [CONTRIBUTING](CONTRIBUTING.md) -- Setup, conventions, and how to submit changes.
- [docs.rs/shiplog](https://docs.rs/shiplog) -- API documentation for all published crates.

## For contributors

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions, coding conventions, and how to submit changes.

```bash
# Quick dev loop
cargo fmt --all && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace
```

## License

Dual licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE), at your option.
