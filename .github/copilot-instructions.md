# Copilot instructions for shiplog

Purpose: Provide concise, repo-specific guidance for Copilot sessions so suggestions, code generation, and edits align with this workspace's structure and conventions.

---

## Build, test, and lint (commands)

Build
- Build the entire workspace: `cargo build --workspace`
- Build a single crate: `cargo build -p <crate-name>` (example: `cargo build -p shiplog-engine`)
- Release build: `cargo build --workspace --release`

Run CLI examples (from README)
- Preferred workflow — collect, then render:
  `cargo run -p shiplog -- collect github --user octocat --since 2025-01-01 --until 2026-01-01 --mode merged --out ./out --include-reviews`
  Then edit `workstreams.suggested.yaml` → `workstreams.yaml`, then:
  `cargo run -p shiplog -- render --out ./out`
- Refresh events while preserving curated workstreams:
  `cargo run -p shiplog -- refresh github --user octocat --since 2025-01-01 --until 2026-01-01 --mode merged --out ./out --include-reviews`
- JSON import mode:
  `cargo run -p shiplog -- collect json --events ./examples/fixture/ledger.events.jsonl --coverage ./examples/fixture/coverage.manifest.json --out ./out`
- Legacy one-shot mode (`run` = collect + render):
  `cargo run -p shiplog -- run github --user octocat --since 2025-01-01 --until 2026-01-01 --mode merged --out ./out --include-reviews`

Testing
- Run all tests in the workspace: `cargo test --workspace`
- Run tests for a single crate: `cargo test -p <crate-name>`
- Run a single test by name (unit or integration): `cargo test -p <crate-name> <test_name>`
  - To run an exact match: `cargo test -p <crate-name> <test_name> -- --exact`
  - To see test output: `cargo test -p <crate-name> <test_name> -- --nocapture`
- Run a specific integration test file (in `tests/`): `cargo test -p <crate-name> --test <integration_file_name>`
- Snapshot tests (insta): update snapshots via environment variables when intentionally changing outputs:
  - Unix/macOS: `INSTA_UPDATE=auto cargo test -p <crate-name>`
  - PowerShell: `$env:INSTA_UPDATE='auto'; cargo test -p <crate-name>`

Lint & format
- Format: `cargo fmt --all` (check-only: `cargo fmt --all -- --check`)
- Lint (Clippy): `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Ensure toolchain components installed: `rustup component add rustfmt clippy`

Fuzzing
- Fuzz harness scaffolding lives in `fuzz/` (not part of the workspace by default). If `cargo-fuzz` is configured locally, run harnesses with `cargo fuzz run <harness>`.

---

## High-level architecture (big picture)

This is a microcrated Rust workspace following Clean Architecture boundaries. Key components (from Cargo.toml workspace members):

- crates/shiplog-ids — ID types and helpers (SHA256-based deterministic IDs)
- crates/shiplog-schema — canonical event model (the data spine)
- crates/shiplog-ports — trait definitions (Ingestor, Renderer, Redactor, WorkstreamClusterer)
- crates/shiplog-coverage — slicing and completeness reporting
- crates/shiplog-workstreams — clustering + editable YAML overrides
- crates/shiplog-redact — deterministic HMAC-SHA256 redaction (internal/manager/public profiles)
- crates/shiplog-render-md — Markdown renderer (snapshot-tested with insta)
- crates/shiplog-render-json — JSON renderer
- crates/shiplog-bundle — checksums + optional zip export
- crates/shiplog-engine — orchestration (ingest → normalize → cluster → render)
- crates/shiplog-ingest-json — JSONL adapter
- crates/shiplog-ingest-github — GitHub adapter (adaptive date slicing, SQLite caching)
- crates/shiplog-ingest-manual — manual ingest adapter (YAML-based non-GitHub events)
- crates/shiplog-cache — SQLite-backed API response caching (TTL-based, reduces GitHub API calls)
- crates/shiplog-testkit — shared test fixtures and utilities
- apps/shiplog — CLI entrypoint (subcommands: collect, render, refresh, run)

Important workspace metadata
- Rust edition: 2024
- Minimum rust-version: 1.92
- Common workspace deps: anyhow, thiserror, serde(+derive), insta, proptest, etc.

Primary runtime/flow: CLI (`apps/shiplog`) drives the engine which wires ingestors (ports -> adapters), normalizes events into the canonical schema, clusters into workstreams, applies deterministic redaction profiles, and renders output formats (Markdown/JSON) with a coverage manifest and optional bundling.

CLI workflow: `collect` fetches events and generates `workstreams.suggested.yaml` → user edits into `workstreams.yaml` → `render` regenerates the packet without re-fetching. `refresh` re-fetches events while preserving curated workstreams. `run` is the legacy one-shot mode.

Outputs typically produced under `out/<run_id>/` and include `packet.md`, `workstreams.yaml`, `workstreams.suggested.yaml`, `ledger.events.jsonl`, `coverage.manifest.json`, and `bundle.manifest.json` (see README examples).

---

## Key conventions and patterns (repo-specific)

- Crate naming: crates use the `shiplog-*` prefix; role is implied by the suffix (schema, ports, ingest-*, render-*, engine).
- Ports & adapters: `crates/shiplog-ports` defines traits; ingest/render crates are adapters that implement those traits (dependency direction: adapters depend on ports and schema, not vice versa).
- Single responsibility microcrates: most logic is split into focused crates (schema, engine, adapters, renderers) — prefer adding small crates for new orthogonal responsibilities.
- Testing conventions:
  - Unit tests live inside each microcrate.
  - Snapshot tests use `insta` for rendered outputs (serialize to YAML/JSON as checked-in snapshots).
  - Property tests use `proptest` for invariants (redaction, etc.).
  - Fuzz harnesses live in `fuzz/` and are not part of the cargo workspace by default.
- Redaction and safety:
  - Redaction is deterministic and profile-based (see `shiplog-redact`).
  - Public packets strip titles and links by default — do not assume private data is safe unless `coverage.manifest.json` shows receipts.
- Coverage-first design: components emit receipts and a coverage manifest; missing receipts are explicitly reported rather than silently omitted.
- Snapshot updates: be explicit when updating insta snapshots (`INSTA_UPDATE`), and prefer small, reviewed snapshot changes.
- CLI usage: prefer `cargo run -p shiplog -- <subcommand> ...` when invoking from the workspace. Use `collect`/`render`/`refresh` for the recommended workflow; `run` is legacy.

---

## Integration notes
- Copied crucial examples and architecture from README.md (use that file for detailed usage examples and philosophy).
- CLAUDE.md and GEMINI.md provide additional AI-assistant guidance. Keep all AI config files in sync when making architectural or workflow changes.

---

If you want this file expanded to include example `cargo` wrappers, CI snippets, or explicit instructions for running tests in editors/IDEs (VS Code/JetBrains), say which area to expand and it will be added.
