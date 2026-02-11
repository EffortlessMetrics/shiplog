# shiplog

**A shipping packet generator for developers.**

`shiplog` compiles a date range of GitHub activity into an editable self-review packet (Markdown), an evidence appendix (JSONL), and a coverage manifest. It is designed to create a report with an audit trail, emphasizing "receipts first".

## Architecture

This project is a modular Rust workspace following **Clean Architecture** principles (Ports & Adapters).

### Workspace Structure

*   **`apps/shiplog`**: The CLI application entry point.
*   **`crates/`**:
    *   **Core Domain:**
        *   `shiplog-schema`: Canonical event model (the data spine).
        *   `shiplog-ports`: Trait definitions (Ingestor, Renderer, Redactor, etc.).
        *   `shiplog-engine`: Orchestration logic (ingest → normalize → cluster → render).
        *   `shiplog-workstreams`: Logic for clustering events into workstreams.
        *   `shiplog-coverage`: Logic for slicing and completeness reporting.
        *   `shiplog-ids`: ID types and helpers.
    *   **Adapters (Infrastructure):**
        *   `shiplog-ingest-github`: GitHub API adapter.
        *   `shiplog-ingest-json`: JSONL import adapter.
        *   `shiplog-ingest-manual`: Manual entry adapter.
        *   `shiplog-render-md`: Markdown renderer (uses `insta` for snapshots).
        *   `shiplog-render-json`: JSON renderer.
        *   `shiplog-bundle`: Zip export functionality.
        *   `shiplog-redact`: Deterministic redaction profiles.
    *   **Testing:**
        *   `shiplog-testkit`: Test utilities.

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

**GitHub Mode:**

```bash
# Requires GITHUB_TOKEN for private repos
cargo run -p shiplog -- run github \
  --user <username> \
  --since YYYY-MM-DD \
  --until YYYY-MM-DD \
  --mode merged \
  --out ./out \
  --include-reviews
```

**JSON Import Mode:**

```bash
cargo run -p shiplog -- run json \
  --events ./examples/fixture/ledger.events.jsonl \
  --coverage ./examples/fixture/coverage.manifest.json \
  --out ./out
```

## Development Conventions

*   **Code Style:** Standard Rust style. Use `cargo fmt` and `cargo clippy`.
*   **Dependency Direction:** Adapters depend on Ports and Schema. Ports and Schema do *not* depend on Adapters.
*   **Testing:**
    *   **Unit Tests:** Located inside each microcrate.
    *   **Snapshot Tests:** Used for rendered outputs (Markdown/JSON). Uses [insta](https://github.com/mitsuhiko/insta).
        *   Update snapshots: `INSTA_UPDATE=auto cargo test -p <crate-name>` (Unix) or `$env:INSTA_UPDATE='auto'; cargo test ...` (PowerShell).
    *   **Property Tests:** Used for invariants (e.g., redaction) using `proptest`.
*   **Redaction:** Deterministic and profile-based. Public packets strip titles/links by default.
*   **Coverage First:** Components must emit receipts. Missing data is explicitly reported in `coverage.manifest.json`.

## Key Commands

*   **Test All:** `cargo test --workspace`
*   **Test Specific Crate:** `cargo test -p <crate-name>`
*   **Format:** `cargo fmt --all`
*   **Lint:** `cargo clippy --workspace --all-targets --all-features -- -D warnings`
