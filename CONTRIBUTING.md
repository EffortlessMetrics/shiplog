# Contributing to shiplog

Thank you for your interest in contributing. This document covers setup, conventions, and the workflow for getting changes merged.

## Prerequisites

- **Rust 1.95+** (edition 2024) — install via [rustup](https://rustup.rs/)
- **Git**
- **cargo-insta** only when reviewing snapshot changes: `cargo install cargo-insta`
- A `GITHUB_TOKEN` only when explicitly testing GitHub ingestion against live APIs

The project pins its toolchain in `rust-toolchain.toml`, so `rustup` will automatically install the correct version (including `rustfmt` and `clippy`).

## Getting started

```bash
git clone https://github.com/EffortlessMetrics/shiplog-swarm.git
cd shiplog-swarm
cargo build --workspace --locked
cargo xtask ci-small
```

All workspace crates should build and pass tests on a clean checkout.

Normal development targets `shiplog-swarm`. Maintainers who need the release
source can add a second remote named `source` for
`https://github.com/EffortlessMetrics/shiplog.git`; ordinary contributors do
not need the promotion topology.

The shortest contributor path is:

```bash
git clone https://github.com/EffortlessMetrics/shiplog-swarm.git
cd shiplog-swarm
cargo build --workspace --locked
cargo xtask ci-small
```

Then create a branch, make the focused edit, run `cargo xtask ci-small`, push
the branch to your fork, and open a pull request against
`EffortlessMetrics/shiplog-swarm/main`. GitHub CLI is optional.

The public checkout must be acquired first. After that, building and running
the gate do not need a retained checkout credential, provider token, or the
GitHub CLI.

## Project structure

shiplog is a module-first Rust workspace following Clean Architecture (Ports & Adapters).
The supported public package surface is being contracted for 0.7. Internal
implementation seams belong under their owning package as modules unless a spec
and ADR promote them. See [API_SURFACE.md](API_SURFACE.md) before adding or
promoting a package.

| Tier | Examples | Role |
|------|----------|------|
| Stable contracts | `shiplog::ids`, `shiplog::schema`, `shiplog::ports` | Core types and traits, no adapter dependencies |
| Trust surfaces | coverage, redaction, bundle, workstreams, render, cache modules | User-visible evidence, privacy, and output behavior under owner packages |
| Adapters | `shiplog::ingest::*` | External-system or stable-import boundaries |
| Orchestration | `shiplog::engine` | Wires adapters together via ports |
| App | `shiplog` (in `apps/shiplog`) | CLI entrypoint (composition root) |
| Test-only | `shiplog-testkit` | Shared fixtures and BDD helpers, not published |
| Internal modules | cache keys/stats/expiry, redaction policy/projector, output layout, team phases, generic helpers | Module folders under owner crates unless deliberately promoted |

**Key rule:** Adapters depend on ports and schema. Ports and schema never depend on adapters.

## Development workflow

1. Fork `shiplog-swarm` and create a feature branch from `main`.
2. Make your changes.
3. Run the quality gates:

```bash
cargo xtask ci-small
```

On Windows PowerShell, use `pwsh -NoProfile -File scripts/dev-check.ps1`.
On Linux or macOS, use `bash scripts/dev-check.sh`. Both wrappers call the
same Rust-native gate as CI.

To catch formatting and lint issues before they hit CI, install the
optional pre-commit hook once per clone:

```bash
scripts/install-hooks.sh
```

This runs `cargo fmt --all -- --check` and `cargo clippy` before each
commit. Skip it for a single commit with `git commit --no-verify`, or
disable it for a session with `SHIPLOG_SKIP_HOOKS=1`.

4. Open a pull request against `shiplog-swarm/main`.

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

### Module-first boundaries

Start a new boundary as a module under the nearest owning package. Promote it to
a new `shiplog-*` crate only when it passes the public crate test in
`SHIPLOG-SPEC-0004` and has an ADR. Naming conventions follow only after that
promotion is justified.

### Keep it simple

Only make changes that are directly necessary. Do not add speculative error handling, feature flags, or abstractions for hypothetical future requirements.

## Adding a new crate

Crate promotion is exceptional. Use [API_SURFACE.md](API_SURFACE.md) to justify the
boundary in the PR description.

### Ingest adapter

1. **Create the module.** Add `apps/shiplog/src/ingest/<source>.rs` or `apps/shiplog/src/ingest/<source>/` and keep it behind the `shiplog` package unless a support-tier ADR promotes it.

2. **Implement the `Ingestor` trait.** See `apps/shiplog/src/ports.rs` for the trait definition. Your adapter must return a `Vec<EventEnvelope>` and a `CoverageManifest`.

3. **Register in the ingest module.** Export the module from `apps/shiplog/src/ingest/mod.rs`.

4. **Wire into the engine.** Wire it into `shiplog::engine` orchestration.

5. **Wire into the CLI.** Add the new source as a subcommand under `collect` and `refresh` in `apps/shiplog`.

6. **Add tests.** Unit tests in your crate, plus at least one integration test using `shiplog-testkit` fixtures.

7. **Update documentation.** Add the new source to `README.md`, `CLAUDE.md`, and `GEMINI.md`.

### Other crate types

For renderers, prefer an owner module such as `apps/shiplog/src/render/` and
implement the `Renderer` trait there. For utility code, prefer an owner module.
Do not add generic `shiplog-*` crates for data structures, queues, counters,
parsers, normalizers, or helper functions.

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
cargo test -p shiplog
cargo test -p shiplog --test render_md
cargo test -p shiplog --test render_md -- --nocapture  # See stdout
cargo test -p shiplog --test render_md my_test -- --exact  # Single test
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
cargo insta review -p shiplog
```

**Updating snapshots** when you intentionally change output:

```bash
# Unix
INSTA_UPDATE=auto cargo test -p shiplog --test render_md

# PowerShell
$env:INSTA_UPDATE='auto'; cargo test -p shiplog --test render_md
```

Always review snapshot diffs carefully before committing. Snapshot changes should reflect intentional output modifications, not accidental regressions.

### Property-based tests (proptest)

Used for invariants that must never regress (ID determinism, redaction leak detection, coverage slicing). Example:

```bash
cargo test -p shiplog --test redact   # Runs proptest-based leak detection
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
| Publish dry-run | `cargo publish -p shiplog --dry-run` | Ubuntu |

Additional CI workflows run for property tests, BDD tests, fuzzing, mutation testing, and security audits — see `.github/workflows/` for details.

## License

This project is dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE). By contributing, you agree that your contributions will be licensed under the same terms.
