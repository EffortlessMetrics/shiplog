# Release Handoff: v0.2.1

Date: 2026-02-17
Owner: Release handoff from Codex session
Target: crates.io publish + GitHub tag/release for `v0.2.1`

## Executive Summary

`shiplog` is prepared for `0.2.1` pending final publish/tag steps.

- Workspace and crate versions are set to `0.2.1`.
- `CHANGELOG.md` has a `0.2.1` entry dated `2026-02-17`.
- Release validation commands were run for format, clippy (`-D warnings`), tests, and release build.

## Current Git State

- Branch: `main`
- Ahead of remote: `main...origin/main [ahead 1]` before this release-prep change set
- Expected uncommitted changes from prep:
  - version bumps (`Cargo.toml` files + `Cargo.lock`)
  - `CHANGELOG.md`
  - `RELEASE_HANDOFF_0.2.1.md`

## Tags and Versioning

- Existing local tags: `v0.2.0`, `v0.1.1`, `v0.1.0`
- Missing expected release tag: `v0.2.1`

## What Was Validated

Executed from `h:\Code\Rust\shiplog`:

1. `cargo fmt --all -- --check`
   - Result: pass
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - Result: pass
3. `cargo test --workspace`
   - Result: pass
4. `cargo build --workspace --release`
   - Result: pass
5. `cargo publish -p shiplog-ids --dry-run --allow-dirty` (publish canary)
   - Result: pass

## Dry-Run Packaging Note

For interdependent internal crates at a new version (`0.2.1`), downstream `cargo publish --dry-run` can fail before upstream crates are actually published to crates.io. Use dependency-order publish and retry after index propagation when needed.

## Publish Scope

Publishable crates (`shiplog-testkit` remains `publish = false`):

1. `shiplog-ids`
2. `shiplog-schema`
3. `shiplog-ports`
4. `shiplog-coverage`
5. `shiplog-cache`
6. `shiplog-workstreams`
7. `shiplog-redact`
8. `shiplog-render-json`
9. `shiplog-render-md`
10. `shiplog-bundle`
11. `shiplog-ingest-json`
12. `shiplog-ingest-manual`
13. `shiplog-ingest-github`
14. `shiplog-cluster-llm`
15. `shiplog-engine`
16. `shiplog` (CLI app crate)

## Release Runbook

1. Commit and push release-prep changes:

```powershell
git add Cargo.toml Cargo.lock apps/shiplog/Cargo.toml crates/*/Cargo.toml CHANGELOG.md RELEASE_HANDOFF_0.2.1.md
git commit -m "chore(release): prepare v0.2.1"
git push origin main
```

2. Publish crates in dependency order:

```powershell
cargo publish -p shiplog-ids
cargo publish -p shiplog-schema
cargo publish -p shiplog-ports
cargo publish -p shiplog-coverage
cargo publish -p shiplog-cache
cargo publish -p shiplog-workstreams
cargo publish -p shiplog-redact
cargo publish -p shiplog-render-json
cargo publish -p shiplog-render-md
cargo publish -p shiplog-bundle
cargo publish -p shiplog-ingest-json
cargo publish -p shiplog-ingest-manual
cargo publish -p shiplog-ingest-github
cargo publish -p shiplog-cluster-llm
cargo publish -p shiplog-engine
cargo publish -p shiplog
```

3. Create and push release tag:

```powershell
git tag -a v0.2.1 -m "Release v0.2.1"
git push origin v0.2.1
```

4. Create GitHub release for `v0.2.1` using the `CHANGELOG.md` `0.2.1` notes.

## Post-Release Verification

1. Confirm crates exist and resolve from crates.io:
   - `cargo search shiplog-ids`
   - `cargo search shiplog`
2. Confirm GitHub release and tag are visible.
3. Confirm changelog compare links:
   - `[Unreleased]: .../compare/v0.2.1...HEAD`
   - `[0.2.1]: .../compare/v0.2.0...v0.2.1`

## Evidence Pointers

- Version source: `Cargo.toml`
- Release notes: `CHANGELOG.md`
- Publishing guidance: `CLAUDE.md`
- CI validation baseline: `.github/workflows/ci.yml`
