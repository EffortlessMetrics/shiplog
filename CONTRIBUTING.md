# Contributing to shiplog

Thank you for your interest in contributing. This document covers setup, conventions, and the workflow for getting changes merged.

## Prerequisites

- **Rust 1.92+** (edition 2024) — install via [rustup](https://rustup.rs/)
- **Git**
- **cargo-insta** for reviewing snapshot tests: `cargo install cargo-insta`
- A `GITHUB_TOKEN` if you plan to test GitHub ingestion against live APIs

The project pins its toolchain in `rust-toolchain.toml`, so `rustup` will automatically install the correct version (including `rustfmt` and `clippy`).

## Getting started

```bash
git clone https://github.com/EffortlessMetrics/shiplog.git
cd shiplog
cargo build --workspace
cargo test --workspace
```

All workspace crates should build and pass tests on a clean checkout.

## Project structure

shiplog is a microcrated Rust workspace following Clean Architecture (Ports & Adapters). The workspace contains 180+ crates organized into tiers:

| Tier | Examples | Role |
|------|----------|------|
| Foundation | `shiplog-ids`, `shiplog-schema`, `shiplog-ports`, `shiplog-coverage` | Core types and traits, no adapter dependencies |
| Adapters | `shiplog-ingest-*`, `shiplog-render-*`, `shiplog-bundle`, `shiplog-cache*`, `shiplog-redact*`, `shiplog-workstreams`, `shiplog-cluster-llm*`, `shiplog-template` | Implement foundation traits |
| Orchestration | `shiplog-engine` | Wires adapters together via ports |
| App | `shiplog` (in `apps/shiplog`) | CLI entrypoint (composition root) |
| Test-only | `shiplog-testkit` | Shared fixtures and BDD helpers, not published |
| Utilities | `shiplog-validate`, `shiplog-filter`, `shiplog-merge`, etc. | Focused single-responsibility support crates |

**Key rule:** Adapters depend on ports and schema. Ports and schema never depend on adapters.

## Development workflow

1. Fork the repository and create a feature branch from `main`.
2. Make your changes.
3. Run the quality gates:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

4. Open a pull request against `main`.

## Code style

### Formatting and linting

```bash
cargo fmt --all            # Auto-format
cargo fmt --all -- --check # Check only (CI mode)
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

CI enforces both. Fix all warnings before opening a PR.

### Error handling

Use `anyhow::Result<T>` with `.context("description")?` for error propagation. Add dynamic context with `.with_context(|| format!(...))`. Do not introduce `thiserror` enums or bare `.unwrap()` in production code.

### Sync core, async at the edges

The core pipeline is synchronous. If you need async (e.g., for a new HTTP-based adapter), isolate it inside the adapter crate. Do not leak async into foundation or orchestration crates.

### Prefer new microcrates

If a new capability is orthogonal to existing crates, create a new `shiplog-<role>` crate rather than enlarging an existing one. Follow the naming convention: `shiplog-ingest-*` for sources, `shiplog-render-*` for output formats, etc.

### Keep it simple

Only make changes that are directly necessary. Do not add speculative error handling, feature flags, or abstractions for hypothetical future requirements.

## Adding a new crate

### Ingest adapter (most common contribution)

1. **Create the crate.** Add `crates/shiplog-ingest-<source>/` with a `Cargo.toml` that depends on `shiplog-ports` and `shiplog-schema`.

2. **Implement the `Ingestor` trait.** See `shiplog-ports/src/lib.rs` for the trait definition. Your adapter must return a `Vec<EventEnvelope>` and a `CoverageManifest`.

3. **Register in the workspace.** Add your crate to the `members` list in the root `Cargo.toml`.

4. **Wire into the engine.** Add your adapter as a dependency of `shiplog-engine` and wire it into the orchestration logic.

5. **Wire into the CLI.** Add the new source as a subcommand under `collect` and `refresh` in `apps/shiplog`.

6. **Add tests.** Unit tests in your crate, plus at least one integration test using `shiplog-testkit` fixtures.

7. **Update documentation.** Add the new source to `README.md`, `CLAUDE.md`, and `GEMINI.md`.

### Other crate types

For renderers (`shiplog-render-*`), follow the same pattern but implement the `Renderer` trait. For utility crates, keep them focused on a single responsibility.

General steps for any new crate:

1. Create `crates/shiplog-<name>/Cargo.toml` inheriting workspace metadata:
   ```toml
   [package]
   name = "shiplog-<name>"
   version.workspace = true
   edition.workspace = true
   rust-version.workspace = true
   license.workspace = true
   ```
2. Add the crate to the root `Cargo.toml` `[workspace] members` list.
3. Depend on workspace crates via `shiplog-<dep> = { path = "../shiplog-<dep>" }`.
4. Use workspace dependencies where available: `anyhow.workspace = true`, `serde.workspace = true`, etc.

## Testing

### Running all tests

```bash
cargo test --workspace
```

### Running tests for a single crate

```bash
cargo test -p shiplog-render-md
cargo test -p shiplog-render-md -- --nocapture  # See stdout
cargo test -p shiplog-render-md my_test -- --exact  # Single test
```

### Unit tests

Place unit tests in the same file as the code, inside a `#[cfg(test)]` module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // ...
    }
}
```

### Integration tests

Place integration tests in `crates/<name>/tests/`. Use `shiplog-testkit` for shared fixtures.

### Snapshot tests (insta)

Snapshot tests use the `insta` crate with YAML format. Used extensively in renderers and serialization.

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

### Property-based tests (proptest)

Used for invariants that must never regress (ID determinism, redaction leak detection, coverage slicing). Example:

```bash
cargo test -p shiplog-redact   # Runs proptest-based leak detection
```

When writing property tests, watch for shrunk failure cases — they reveal the minimal reproducer.

### BDD scenario tests

The project uses a custom Given/When/Then framework via `shiplog-testkit::bdd`. Example:

```rust
use shiplog_testkit::bdd::{Scenario, assert_present, assert_true};

#[test]
fn bdd_my_workflow() {
    Scenario::new("My workflow description")
        .given("some precondition", |ctx| {
            ctx.strings.insert("key".into(), "value".into());
        })
        .when("action is performed", |ctx| {
            let val = assert_present(ctx.string("key"), "key")?;
            ctx.strings.insert("result".into(), val.to_uppercase());
            Ok(())
        })
        .then("expected outcome", |ctx| {
            let result = assert_present(ctx.string("result"), "result")?;
            assert_true(result == "VALUE", "should be uppercase")
        })
        .run()
        .expect("scenario should pass");
}
```

### Fuzz testing

Fuzz harnesses live in `fuzz/` (not part of the default workspace). If you have `cargo-fuzz` installed:

```bash
cargo fuzz run <harness>       # Run a fuzz target
cargo fuzz list                # List available harnesses
```

Fuzz targets cover parsers and file formats (JSONL, YAML, LLM response JSON).

### Mutation testing

```bash
cargo mutants --workspace      # Requires cargo-mutants
```

Focus mutation testing on trust surfaces: redaction, bundle filters, schema serialization, coverage slicing.

## Commit messages

Write clear, concise commit messages in imperative mood ("Add GitLab adapter", not "Added GitLab adapter"). Reference issue numbers where applicable.

When using AI assistants, include the Co-authored-by trailer:

```
feat: add GitLab ingest adapter

Implement MR and review event ingestion from GitLab API.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

## Pull request guidelines

- Create a feature branch from `main` (e.g., `feature/gitlab-adapter`, `fix/redaction-leak`).
- Keep PRs focused on a single concern.
- Include tests for new functionality.
- Ensure CI passes before requesting review.
- Update relevant documentation (`README.md`, `CLAUDE.md`, `GEMINI.md`) if behavior changes.

## CI expectations

CI runs on every pull request and must pass before merging. The pipeline checks:

| Check | Command | Runs on |
|-------|---------|---------|
| Formatting | `cargo fmt --all -- --check` | Ubuntu |
| Linting | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Ubuntu + Windows |
| Tests | `cargo test --workspace` | Ubuntu + Windows |
| Release build | `cargo build --workspace --release` | Ubuntu + Windows |
| Publish dry-run | `cargo publish -p shiplog-ids --dry-run` | Ubuntu |

Additional CI workflows run for property tests, BDD tests, fuzzing, mutation testing, and security audits — see `.github/workflows/` for details.

## License

This project is dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE). By contributing, you agree that your contributions will be licensed under the same terms.
