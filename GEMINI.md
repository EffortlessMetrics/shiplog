# shiplog

**A shipping packet generator for developers.**

`shiplog` compiles a date range of GitHub activity into an editable self-review packet (Markdown), an evidence appendix (JSONL), and a coverage manifest. It is designed to create a report with an audit trail, emphasizing "receipts first".

## Architecture

This project is a modular Rust workspace following **Clean Architecture** principles (Ports & Adapters).

### Workspace Structure

*   **`apps/shiplog`**: The CLI application entry point (subcommands: `collect`, `render`, `refresh`, `import`, `run`).
*   **`crates/`**:
    *   **Core Domain:**
        *   `shiplog-schema`: Canonical event model (the data spine).
        *   `shiplog-ports`: Trait definitions (Ingestor, Renderer, Redactor, WorkstreamClusterer).
        *   `shiplog-engine`: Orchestration logic (ingest → normalize → cluster → render).
        *   `shiplog-workstreams`: Logic for clustering events into workstreams.
        *   `shiplog-coverage`: Logic for slicing and completeness reporting.
        *   `shiplog-ids`: Type-safe stable ID generation (SHA256-based).
        *   `shiplog-redact`: Deterministic HMAC-SHA256 redaction (internal/manager/public profiles).
    *   **Adapters (Infrastructure):**
        *   `shiplog-ingest-github`: GitHub API adapter (adaptive date slicing, SQLite caching).
        *   `shiplog-ingest-json`: JSONL import adapter.
        *   `shiplog-ingest-manual`: Manual entry adapter (YAML-based non-GitHub events).
        *   `shiplog-render-md`: Markdown renderer (uses `insta` for snapshots).
        *   `shiplog-render-json`: JSON renderer.
        *   `shiplog-bundle`: Zip export functionality.
        *   `shiplog-cache`: SQLite-backed API response caching (TTL-based, reduces GitHub API calls).
    *   **Clustering:**
        *   `shiplog-cluster-llm`: Optional LLM-assisted workstream clustering (feature-gated in CLI).
    *   **Testing:**
        *   `shiplog-testkit`: Shared test fixtures and utilities.

**Key rule:** Adapters depend on ports and schema, never the reverse.

## Building and Running

### Build

```bash
# Build workspace
cargo build --workspace

# Release build
cargo build --workspace --release
```

### Run CLI

Use `cargo run -p shiplog` to run the CLI.

**Recommended workflow — collect, edit, render:**

```bash
# 1. Collect events and generate workstream suggestions
export GITHUB_TOKEN="ghp_..."   # optional for public repos

cargo run -p shiplog -- collect github \
  --user <username> \
  --since YYYY-MM-DD \
  --until YYYY-MM-DD \
  --mode merged \
  --out ./out \
  --include-reviews

# 2. Edit: rename workstreams.suggested.yaml → workstreams.yaml and curate

# 3. Re-render from curated workstreams (no re-fetch)
cargo run -p shiplog -- render --out ./out
```

**Refresh events while preserving curation:**

```bash
cargo run -p shiplog -- refresh github \
  --user <username> \
  --since YYYY-MM-DD \
  --until YYYY-MM-DD \
  --mode merged \
  --out ./out \
  --include-reviews
```

**JSON Import Mode:**

```bash
cargo run -p shiplog -- collect json \
  --events ./examples/fixture/ledger.events.jsonl \
  --coverage ./examples/fixture/coverage.manifest.json \
  --out ./out
```

**Legacy one-shot mode** (`run` = collect + render):

```bash
cargo run -p shiplog -- run github \
  --user <username> \
  --since YYYY-MM-DD \
  --until YYYY-MM-DD \
  --mode merged \
  --out ./out \
  --include-reviews
```

### Key CLI flags

Flags available on `collect`/`refresh` github subcommands:

*   `--mode merged|created` — which PR lens to ingest
*   `--include-reviews` — include review events where available
*   `--no-details` — omit verbose details in packet
*   `--throttle-ms <N>` — rate-limit API calls (milliseconds)
*   `--api-base <URL>` — GitHub Enterprise Server API base
*   `--regen` — regenerate `workstreams.suggested.yaml`; never overwrites `workstreams.yaml`
*   `--run-dir <PATH>` — explicit run directory for `refresh` (overrides auto-detection)
*   `--zip` — write a zip archive next to the run folder
*   `--redact-key` or `SHIPLOG_REDACT_KEY` env var — controls generation of manager/public packets
*   `--llm-cluster` — use LLM-assisted workstream clustering instead of repo-based
*   `--llm-api-endpoint <URL>` — LLM endpoint (default: OpenAI-compatible)
*   `--llm-model <NAME>` — LLM model name (default: `gpt-4o-mini`)
*   `--llm-api-key <KEY>` — LLM API key (or `SHIPLOG_LLM_API_KEY` env var)

### Output directory structure

Outputs go under `out/<run_id>/`:

*   `packet.md`
*   `ledger.events.jsonl`
*   `coverage.manifest.json`
*   `workstreams.yaml`
*   `profiles/{manager,public}/packet.md` (redacted)
*   `bundle.manifest.json`

## Development Conventions

*   **Code Style:** Standard Rust style. Use `cargo fmt` and `cargo clippy`.
*   **Dependency Direction:** Adapters depend on Ports and Schema. Ports and Schema do *not* depend on Adapters.
*   **Testing:**
    *   **Unit Tests:** Located inside each microcrate.
    *   **Snapshot Tests:** Used for rendered outputs (Markdown/JSON). Uses [insta](https://github.com/mitsuhiko/insta).
        *   Update snapshots: `INSTA_UPDATE=auto cargo test -p <crate-name>` (Unix) or `$env:INSTA_UPDATE='auto'; cargo test ...` (PowerShell).
    *   **Property Tests:** Used for invariants (e.g., redaction) using `proptest`.
    *   **BDD Tests:** Scenario-driven integration tests via `shiplog-testkit::bdd`.
    *   **Shared Fixtures:** Use `shiplog-testkit` to avoid cross-crate duplication.
*   **Redaction:** Deterministic and profile-based (internal/manager/public). Public packets strip titles/links by default.
*   **Coverage First:** Components must emit receipts. Missing data is explicitly reported in `coverage.manifest.json`.

### Error handling

*   Use `anyhow::Result<T>` with `.context("description")?` for error propagation throughout.
*   Add contextual messages with `.with_context(|| format!(...))` for dynamic info.
*   Do not introduce `thiserror` enums or bare `.unwrap()` where `anyhow` context is expected.

### Runtime

*   GitHub ingest currently uses `reqwest::blocking`. If introducing async, isolate it inside adapters; don't leak it into core crates.

## Key Commands

*   **Test All:** `cargo test --workspace`
*   **Test Specific Crate:** `cargo test -p <crate-name>`
*   **Test Single Test:** `cargo test -p <crate-name> <test_name> -- --exact --nocapture`
*   **Format:** `cargo fmt --all`
*   **Format Check:** `cargo fmt --all -- --check`
*   **Lint:** `cargo clippy --workspace --all-targets --all-features -- -D warnings`
*   **Mutation Testing:** `cargo mutants --workspace`
