# Contributing to shiplog

Thank you for your interest in contributing. This document covers setup, conventions, and the workflow for getting changes merged.

## Prerequisites

- **Rust 1.92+** (edition 2024)
- **cargo-insta** for reviewing snapshot tests: `cargo install cargo-insta`
- A `GITHUB_TOKEN` if you plan to test GitHub ingestion against live APIs

## Getting started

```bash
git clone https://github.com/EffortlessMetrics/shiplog.git
cd shiplog
cargo build --workspace
cargo test --workspace
```

All 17 workspace crates should build and pass tests on a clean checkout.

## Project structure

shiplog is a microcrated Rust workspace organized into tiers:

| Tier | Crates | Role |
|------|--------|------|
| Foundation | `shiplog-ids`, `shiplog-schema`, `shiplog-ports`, `shiplog-coverage` | Core types and traits, no adapter dependencies |
| Adapters | `shiplog-ingest-*`, `shiplog-render-*`, `shiplog-bundle`, `shiplog-cache`, `shiplog-redact`, `shiplog-workstreams`, `shiplog-cluster-llm` | Implement foundation traits |
| Orchestration | `shiplog-engine` | Wires adapters together via ports |
| App | `shiplog` (in `apps/shiplog`) | CLI entrypoint |
| Test-only | `shiplog-testkit` | Shared fixtures, not published |

**Key rule:** Adapters depend on ports and schema. Ports and schema never depend on adapters.

## Development workflow

1. Fork the repository and create a feature branch from `main`.
2. Make your changes.
3. Run formatting, linting, and tests:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

4. Open a pull request against `main`.

## Code conventions

### Error handling

Use `anyhow::Result<T>` with `.context("description")?` for error propagation. Add dynamic context with `.with_context(|| format!(...))`. Do not introduce `thiserror` enums or bare `.unwrap()` in production code.

### Sync core, async at the edges

The core pipeline is synchronous. If you need async (e.g., for a new HTTP-based adapter), isolate it inside the adapter crate. Do not leak async into foundation or orchestration crates.

### Prefer new microcrates

If a new capability is orthogonal to existing crates, create a new `shiplog-<role>` crate rather than enlarging an existing one. Follow the naming convention: `shiplog-ingest-*` for sources, `shiplog-render-*` for output formats, etc.

### Keep it simple

Only make changes that are directly necessary. Do not add speculative error handling, feature flags, or abstractions for hypothetical future requirements.

## Adding an ingest adapter

Ingest adapters are the most common type of contribution. Here is a step-by-step recipe:

1. **Create the crate.** Add `crates/shiplog-ingest-<source>/` with a `Cargo.toml` that depends on `shiplog-ports` and `shiplog-schema`.

2. **Implement the `Ingestor` trait.** See `shiplog-ports/src/lib.rs` for the trait definition. Your adapter must return a `Vec<EventEnvelope>` and a `CoverageManifest`.

3. **Register in the workspace.** Add your crate to the `members` list in the root `Cargo.toml`.

4. **Wire into the engine.** Add your adapter as a dependency of `shiplog-engine` and wire it into the orchestration logic.

5. **Wire into the CLI.** Add the new source as a subcommand under `collect` and `refresh` in `apps/shiplog`.

6. **Add tests.** Unit tests in your crate, plus at least one integration test using `shiplog-testkit` fixtures.

7. **Update documentation.** Add the new source to `README.md` and `CLAUDE.md`.

## Snapshot tests

Snapshot tests use the `insta` crate with YAML format, primarily in `shiplog-render-md`.

**Reviewing snapshots:**

```bash
cargo insta review -p shiplog-render-md
```

**Updating snapshots** when you intentionally change output:

```bash
# Unix
INSTA_UPDATE=auto cargo test -p shiplog-render-md

# PowerShell
$env:INSTA_UPDATE='auto'; cargo test -p shiplog-render-md
```

Always review snapshot diffs carefully before committing. Snapshot changes should reflect intentional output modifications, not accidental regressions.

## Property-based tests

`shiplog-redact` uses `proptest` for redaction leak detection. If you modify redaction logic, run these tests and watch for shrunk failure cases:

```bash
cargo test -p shiplog-redact
```

## Commit messages

Write clear, concise commit messages. Use imperative mood ("Add GitLab adapter", not "Added GitLab adapter"). Reference issue numbers where applicable.

## Pull request guidelines

- Keep PRs focused on a single concern.
- Include tests for new functionality.
- Ensure CI passes (fmt, clippy, tests).
- Update relevant documentation if behavior changes.

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project: [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
