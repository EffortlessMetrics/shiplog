# shiplog

A **shipping packet generator** for developers.

It turns a date range of GitHub activity into:

- an **editable self-review packet** (Markdown)
- an **evidence appendix** (JSONL)
- a **coverage manifest** that tells you what was captured, and what might be missing

This is not a productivity scoreboard.
It is a **report compiler with an audit trail**.

## What it produces

Given a user + date range, `shiplog` generates a folder like:

```
out/<run_id>/
  packet.md
  workstreams.yaml              # user-curated (yours to edit)
  workstreams.suggested.yaml    # auto-generated suggestions
  ledger.events.jsonl
  coverage.manifest.json
  bundle.manifest.json
  profiles/
    manager/packet.md
    public/packet.md
```

Key design choice: **receipts first**.
If the tool cannot prove it fetched something, it says so in `coverage.manifest.json`.

## Quick start

### Recommended workflow: collect, edit, render

```bash
# 1. Collect events and generate workstream suggestions
export GITHUB_TOKEN="ghp_..."   # optional for public repos

cargo run -p shiplog -- collect github \
  --user octocat \
  --since 2025-01-01 \
  --until 2026-01-01 \
  --mode merged \
  --out ./out \
  --include-reviews

# 2. Edit workstreams: rename workstreams.suggested.yaml → workstreams.yaml
#    and curate the narrative to match your story.

# 3. Re-render the packet from your curated workstreams (no re-fetch)
cargo run -p shiplog -- render --out ./out
```

### Refresh: re-fetch events, keep your curation

```bash
cargo run -p shiplog -- refresh github \
  --user octocat \
  --since 2025-01-01 \
  --until 2026-01-01 \
  --mode merged \
  --out ./out \
  --include-reviews
```

### Import a pre-built ledger

Use the `import` subcommand to consume output from another system or a previous
shiplog run and re-render it:

```bash
cargo run -p shiplog -- import \
  --dir ./path/to/existing/run \
  --out ./out
```

`import` reads `ledger.events.jsonl` and `coverage.manifest.json` from the
given directory, re-clusters into workstreams, and renders a fresh packet.
Pass `--regen` to ignore imported workstreams and re-cluster from scratch.

### JSON import mode

```bash
cargo run -p shiplog -- collect json \
  --events ./examples/fixture/ledger.events.jsonl \
  --coverage ./examples/fixture/coverage.manifest.json \
  --out ./out
```

### One-shot mode (legacy)

`run` combines collect + render in a single step:

```bash
cargo run -p shiplog -- run github \
  --user octocat \
  --since 2025-01-01 \
  --until 2026-01-01 \
  --mode merged \
  --out ./out \
  --include-reviews
```

### Redaction

Pass a redaction key to generate privacy-filtered packets (manager/public profiles):

```bash
cargo run -p shiplog -- collect github ... --redact-key "my-secret"
# or via environment variable:
export SHIPLOG_REDACT_KEY="my-secret"
```

## CLI Reference

| Flag | Subcommands | Description |
|------|-------------|-------------|
| `--mode merged\|created` | `collect`, `refresh` | Which PR lens to ingest |
| `--include-reviews` | `collect`, `refresh` | Include review events |
| `--no-details` | `collect`, `refresh`, `run` | Omit verbose details |
| `--throttle-ms <N>` | `collect`, `refresh` | Rate-limit API calls (ms) |
| `--api-base <URL>` | `collect`, `refresh` | GitHub Enterprise Server API base |
| `--regen` | `collect`, `import` | Regenerate `workstreams.suggested.yaml` |
| `--redact-key <KEY>` | all | HMAC key for redacted profiles (or `SHIPLOG_REDACT_KEY` env var) |
| `--run-dir <PATH>` | `refresh` | Explicit run directory (overrides auto-detection) |
| `--zip` | `collect`, `render`, `refresh`, `import`, `run` | Write a zip archive next to the run folder |
| `--bundle-profile internal\|manager\|public` | `collect`, `render`, `refresh`, `import`, `run` | Scope bundle/zip to a redaction profile |
| `--llm-cluster` | `collect`, `refresh`, `import`, `run` | Use LLM-assisted workstream clustering |
| `--llm-api-endpoint <URL>` | `collect`, `refresh`, `import`, `run` | LLM endpoint (default: OpenAI) |
| `--llm-model <NAME>` | `collect`, `refresh`, `import`, `run` | LLM model name (default: `gpt-4o-mini`) |
| `--llm-api-key <KEY>` | `collect`, `refresh`, `import`, `run` | LLM API key (or `SHIPLOG_LLM_API_KEY` env var) |

### LLM Clustering

By default, workstreams are clustered by repository name. Pass `--llm-cluster`
to send event summaries to an OpenAI-compatible endpoint for semantic clustering:

```bash
export SHIPLOG_LLM_API_KEY="sk-..."
cargo run -p shiplog -- collect github ... --llm-cluster
```

The LLM feature must be enabled at build time:
`cargo build -p shiplog --features llm`
(or `cargo install shiplog --features llm`).
Without it, `--llm-cluster` prints a clear error and exits.

### Redaction alias cache

When a redaction key is set, the redactor writes
`redaction.aliases.json` to the run directory for alias stability across runs.
This file is **excluded** from bundle manifests and zip archives to prevent
plaintext-to-alias mappings from leaking.

## Why this exists

Your devs do not need more PRs.
They need fewer surprises per PR.

Performance reviews are the same shape: a year's worth of work, compressed into a handful
of workstreams with enough receipts that a reviewer can trust it without spelunking.

GitHub has the artifacts. It does not build the packet.

`shiplog` does.

## Architecture

This is a heavily microcrated Rust workspace with Clean Architecture boundaries:

- **crates/shiplog-ids**: type-safe stable ID generation (SHA256-based)
- **crates/shiplog-schema**: canonical event model (the data spine)
- **crates/shiplog-ports**: traits (ingestors, renderers, redactors, clusterers)
- **crates/shiplog-ingest-\***: adapters (GitHub, JSONL, manual YAML events)
- **crates/shiplog-engine**: orchestration (ingest → normalize → cluster → render)
- **crates/shiplog-workstreams**: clustering + editable YAML overrides
- **crates/shiplog-redact**: deterministic redaction profiles (internal/manager/public)
- **crates/shiplog-coverage**: slicing + completeness reporting
- **crates/shiplog-cache**: SQLite-backed API response caching (reduces GitHub API calls)
- **crates/shiplog-render-\***: outputs (Markdown, JSON)
- **crates/shiplog-bundle**: checksums + optional zip export
- **crates/shiplog-testkit**: shared test fixtures
- **apps/shiplog**: CLI

## Crate stability

| Tier | Crates | Contract |
|------|--------|----------|
| **Stable** | `shiplog-schema`, `shiplog-ports`, `shiplog-ids` | SemVer-strict. Safe to depend on. |
| **Supported** | All other `shiplog-*` crates | SemVer, but internals may shift between minors. |
| **Dev-only** | `shiplog-testkit` | Not published. No stability guarantee. |

## Testing posture

- Unit tests in microcrates.
- Snapshot tests for rendered packets (`insta`).
- Property tests for redaction invariants (`proptest`).
- Fuzz harness scaffolding lives in `fuzz/` (not part of the workspace by default).

## Safety and trust

- Outputs include a **coverage manifest**.
- Redaction is deterministic and profile-based.
- Public packets strip titles and links by default. You should never leak private repo details.

## License

Dual-licensed under MIT or Apache-2.0.
