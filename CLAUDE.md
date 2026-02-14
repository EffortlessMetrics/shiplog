# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build, Test, and Lint Commands

```bash
cargo build --workspace                  # Build all crates
cargo build -p <crate-name>             # Build a single crate (e.g., shiplog-engine)
cargo test --workspace                   # Run all tests
cargo test -p <crate-name>              # Test a single crate
cargo test -p <crate-name> <test_name>  # Run a specific test
cargo test -p <crate-name> <test_name> -- --exact --nocapture  # Exact match with output
cargo fmt --all -- --check               # Check formatting
cargo clippy --workspace --all-targets --all-features -- -D warnings  # Lint
cargo mutants --workspace                # Mutation testing
```

Snapshot tests use `insta` (YAML format). Update snapshots when intentionally changing outputs:
- PowerShell: `$env:INSTA_UPDATE='auto'; cargo test -p <crate-name>`
- Unix: `INSTA_UPDATE=auto cargo test -p <crate-name>`

Run the CLI: `cargo run -p shiplog -- <subcommand>`. Preferred workflow: `collect` (fetch events) → edit `workstreams.suggested.yaml` into `workstreams.yaml` → `render` (regenerate packet). `refresh` re-fetches events while preserving curated workstreams. `run` is legacy (collect + render in one shot).

Key CLI flags (on `collect`/`refresh` github subcommands):
- `--mode merged|created` (which PR lens to ingest)
- `--include-reviews` (include review events where available)
- `--no-details` (omit verbose details in packet)
- `--throttle-ms <N>` (rate-limit API calls)
- `--api-base <URL>` (GitHub Enterprise Server API base)
- `--regen` (regenerate `workstreams.suggested.yaml`; never overwrites `workstreams.yaml`)
- Redaction: `--redact-key` or `SHIPLOG_REDACT_KEY` controls generation of manager/public packets

## Architecture

Microcrated Rust workspace (edition 2024, MSRV 1.92) following **Clean Architecture / ports-and-adapters**. The CLI (`apps/shiplog`) drives `shiplog-engine`, which orchestrates: ingest → cluster → redact → render.

### Dependency layers (top → bottom)

```
apps/shiplog (CLI, clap)
  └─ shiplog-engine (orchestration)
       ├─ Ingest adapters: shiplog-ingest-github, shiplog-ingest-json, shiplog-ingest-manual
       ├─ shiplog-workstreams (clustering + user-curated YAML)
       ├─ shiplog-redact (deterministic HMAC-SHA256 aliasing, 3 profiles)
       ├─ shiplog-render-md, shiplog-render-json
       └─ shiplog-bundle (zip + SHA256 checksums)
  Shared foundations:
       shiplog-ports (trait definitions: Ingestor, Renderer, Redactor, WorkstreamClusterer)
       shiplog-schema (canonical event model, EventKind, manifests)
       shiplog-ids (deterministic SHA256-based EventId, RunId, WorkstreamId)
       shiplog-coverage (time windows, completeness tracking)
       shiplog-cache (SQLite-backed API response cache, rusqlite bundled)
       shiplog-testkit (fixture builders for tests)
```

**Key rule:** Adapters depend on ports and schema, never the reverse.

### Core design principles

- **Receipts-first:** Every claim must trace to fetched evidence. Missing data is explicitly reported in `coverage.manifest.json`, never silently omitted.
- **User-owned workstreams:** `workstreams.yaml` is user-curated and never overwritten; auto-generated suggestions go to `workstreams.suggested.yaml`.
- **Deterministic redaction:** Three profiles (internal/manager/public). Same input + same key = same alias across runs via HMAC-SHA256.
- **Immutable event ledger:** `ledger.events.jsonl` is the canonical, append-only event log.

### Error handling

- `anyhow::Result<T>` with `.context("description")?` for error propagation throughout.
- Add contextual messages with `.with_context(|| format!(...))` for dynamic info.
- Do not introduce `thiserror` enums or bare `.unwrap()` where `anyhow` context is expected.

### Runtime

- GitHub ingest currently uses `reqwest::blocking`. If introducing async, isolate it inside adapters; don't leak it into core crates.

### Output directory structure

Outputs go under `out/<run_id>/`: `packet.md`, `ledger.events.jsonl`, `coverage.manifest.json`, `workstreams.yaml`, `profiles/{manager,public}/packet.md` (redacted), `bundle.manifest.json`.

### Testing conventions

- Unit tests live inside each microcrate's source files.
- Snapshot tests (`insta`, YAML format) in `shiplog-render-md` — review snapshot diffs carefully.
- Property-based tests (`proptest`) in `shiplog-redact` for redaction leak detection.
- Shared fixtures via `shiplog-testkit::fixtures` to avoid cross-crate duplication.
- BDD-style test infrastructure in `shiplog-testkit::bdd` for scenario-driven integration tests.
- Fuzz harnesses in `fuzz/` (not part of workspace; requires `cargo-fuzz`).

### Crate naming convention

Prefix `shiplog-` with suffix indicating role: `-schema`, `-ports`, `-ingest-*`, `-render-*`, `-engine`. New orthogonal responsibilities should become new microcrates rather than enlarging existing ones.
