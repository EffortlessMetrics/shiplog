# Copilot instructions for shiplog

Purpose: Provide concise, repo-specific guidance for Copilot sessions so suggestions, code generation, and edits align with this workspace's structure and conventions.

---

## Build, test, and lint (commands)

Build
- Build the entire workspace: `cargo build --workspace`
- Build a single crate: `cargo build -p <crate-name>` (example: `cargo build -p shiplog-engine`)
- Release build: `cargo build --workspace --release`

Run CLI examples (from README)
- GitHub mode (requires token for private repos):
  `cargo run -p shiplog -- run github --user octocat --since 2025-01-01 --until 2026-01-01 --mode merged --out ./out --include-reviews`
- JSON import mode:
  `cargo run -p shiplog -- run json --events ./examples/fixture/ledger.events.jsonl --coverage ./examples/fixture/coverage.manifest.json --out ./out`

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

- crates/shiplog-ids — ID types and helpers
- crates/shiplog-schema — canonical event model (the data spine)
- crates/shiplog-ports — trait definitions (ingestors, renderers)
- crates/shiplog-coverage — slicing and completeness reporting
- crates/shiplog-workstreams — clustering + editable YAML overrides
- crates/shiplog-redact — deterministic redaction profiles
- crates/shiplog-render-md — Markdown renderer (snapshot-tested)
- crates/shiplog-render-json — JSON renderer
- crates/shiplog-bundle — checksums + optional zip export
- crates/shiplog-engine — orchestration (ingest → normalize → cluster → render)
- crates/shiplog-ingest-json — JSONL adapter
- crates/shiplog-ingest-github — GitHub adapter
- crates/shiplog-ingest-manual — manual ingest adapter
- crates/shiplog-testkit — test utilities
- apps/shiplog — CLI entrypoint

Important workspace metadata
- Rust edition: 2024
- Minimum rust-version: 1.92
- Common workspace deps: anyhow, thiserror, serde(+derive), insta, proptest, etc.

Primary runtime/flow: CLI (`apps/shiplog`) drives the engine which wires ingestors (ports -> adapters), normalizes events into the canonical schema, clusters into workstreams, applies deterministic redaction profiles, and renders output formats (Markdown/JSON) with a coverage manifest and optional bundling.

Outputs typically produced under `out/<run_id>/` and include `packet.md`, `ledger.events.jsonl`, `coverage.manifest.json`, and `bundle.manifest.json` (see README examples).

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
- CLI usage: prefer `cargo run -p shiplog -- run <mode> ...` when invoking from the workspace to target the binary directly.

---

## Integration notes
- Copied crucial examples and architecture from README.md (use that file for detailed usage examples and philosophy).
- No CONTRIBUTING.md, CLAUDE.md, AGENTS.md, or other known AI-assistant config files were found in this repository when this file was created; if you add assistant-specific config, include the important rules here.

---

If you want this file expanded to include example `cargo` wrappers, CI snippets, or explicit instructions for running tests in editors/IDEs (VS Code/JetBrains), say which area to expand and it will be added.
