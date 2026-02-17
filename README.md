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

## Quick start

### 1. Collect events from GitHub

```bash
shiplog collect github \
  --user your-username \
  --since 2025-07-01 \
  --until 2026-01-01 \
  --mode merged \
  --out ./out
```

This fetches merged PRs (and optionally reviews with `--include-reviews`) for the given user and time window. Results go into `out/<run_id>/` with a JSONL event ledger, coverage manifest, and an initial packet.

### 2. Curate workstreams

shiplog generates `workstreams.suggested.yaml` with auto-grouped workstreams based on repository. Copy it to `workstreams.yaml` and edit:

```bash
cp out/<run_id>/workstreams.suggested.yaml out/<run_id>/workstreams.yaml
# Edit workstreams.yaml: rename groups, move PRs between workstreams, add narrative.
```

`workstreams.yaml` is yours. shiplog never overwrites it.

### 3. Re-render the packet

```bash
shiplog render --run-dir out/<run_id>
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

### Sources

| Source | Description |
|--------|-------------|
| `github` | PR and review ingestion from GitHub API (with adaptive slicing and SQLite cache) |
| `json` | Import from canonical JSONL event files |
| `manual` | Ingest non-GitHub work from a YAML events file |

### Examples

```bash
# Refresh receipts while keeping curated workstreams
shiplog refresh github \
  --user your-username \
  --since 2025-07-01 \
  --until 2026-01-01 \
  --run-dir out/20260115_143022 \
  --out ./out

# Import from pre-built JSON artifacts
shiplog collect json \
  --events ./ledger.events.jsonl \
  --coverage ./coverage.manifest.json \
  --out ./out

# Collect manual (non-GitHub) events
shiplog collect manual \
  --events ./manual_events.yaml \
  --user your-username \
  --since 2025-07-01 \
  --until 2026-01-01 \
  --out ./out
```

## Redaction and profiles

Generate redacted packets by providing a key:

```bash
shiplog render --run-dir out/<run_id> --redact-key my-stable-secret
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
shiplog render --run-dir out/<run_id> --redact-key my-stable-secret --zip --bundle-profile manager
```

## Architecture

shiplog is a microcrated Rust workspace following clean architecture (ports and adapters).

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
  | - json       |  | - redact     |  | - render-json  |
  | - manual     |  | - cluster-llm|  | - bundle       |
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

### Workspace crates

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
| `shiplog-ingest-json` | Import from canonical JSON artifacts |
| `shiplog-ingest-manual` | YAML-based manual event ingestion |
| `shiplog-workstreams` | Workstream clustering and user-curated YAML workflow |
| `shiplog-cluster-llm` | Optional LLM-assisted semantic clustering |
| `shiplog-redact` | Deterministic HMAC-SHA256 redaction across three profiles |
| `shiplog-render-md` | Markdown packet renderer |
| `shiplog-render-json` | JSON output renderer |
| `shiplog-bundle` | Zip archives with SHA256 checksum manifests |
| `shiplog-testkit` | Shared test fixtures (not published) |

## LLM clustering

By default, shiplog clusters events by repository. With the `llm` feature, you can enable semantic clustering via an OpenAI-compatible API:

```bash
# Install with LLM support
cargo install shiplog --features llm

# Use LLM clustering during collection
shiplog collect github \
  --user your-username \
  --since 2025-07-01 \
  --until 2026-01-01 \
  --out ./out \
  --llm-cluster \
  --llm-api-key $SHIPLOG_LLM_API_KEY
```

LLM clustering is feature-gated and off by default. It falls back to repository-based clustering on failure. See `--llm-api-endpoint` and `--llm-model` for endpoint configuration.

## Documentation

- [CHANGELOG](CHANGELOG.md) -- Release history and migration notes.
- [ROADMAP](ROADMAP.md) -- What is planned, what is next, and what is out of scope.
- [CONTRIBUTING](CONTRIBUTING.md) -- Setup, conventions, and how to submit changes.
- [docs.rs/shiplog](https://docs.rs/shiplog) -- API documentation for all published crates.

## License

Dual licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE), at your option.
