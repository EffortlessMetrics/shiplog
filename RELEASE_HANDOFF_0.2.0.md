# Release Handoff: v0.2.0

Date: 2026-02-16
Owner: Release handoff from Codex session
Target: crates.io publish + GitHub tag/release for `v0.2.0`

## Executive Summary

`shiplog` is release-ready for `0.2.0` from a code quality and packaging perspective.

- Workspace is on `0.2.0` (`Cargo.toml` workspace version).
- `CHANGELOG.md` has a `0.2.0` entry dated `2026-02-15`.
- Validation suite passed: format, clippy (`-D warnings`), tests, release build.
- Publish dry-runs were completed for all publishable crates.
- Working tree is clean.

## Current Git State

- Branch: `main`
- Ahead of remote: `main...origin/main [ahead 2]`
- Uncommitted changes at handoff creation: none
- Recent commits:
  - `a906afb` feat: update project metadata and descriptions across multiple Cargo.toml files and GitHub settings
  - `23787ce` feat: update CI workflow and add tests for event handling and coverage reporting
  - `1caf9e4` (origin/main) chore: pre-public cleanup — gitignore OS junk, neutralize token placeholders (#2)

## Tags and Versioning

- Existing local tags: `v0.1.1`, `v0.1.0`
- Missing expected release tag: `v0.2.0`

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
5. `cargo publish --dry-run` for all publishable crates in dependency order
   - Result: pass for all crates

## Dry-Run Packaging Note (Important)

Because `0.2.0` interdependent internal crates are not yet on crates.io, plain downstream dry-runs can fail before upstream crates are published (for example, `shiplog-schema` requiring `shiplog-ids` from crates.io).

To fully preflight every crate before first publish, temporary `patch.crates-io` overrides were supplied at command time during dry-run verification. This does not modify repository files and is safe for preflight.

## Publish Scope

Publishable crates (`shiplog-testkit` is intentionally `publish = false`):

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

Run in this order.

1. Push release commits to remote:

```powershell
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
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0
```

4. Create GitHub release for `v0.2.0` using `CHANGELOG.md` `0.2.0` notes.

## Post-Release Verification

1. Confirm crates exist and resolve from crates.io:
   - `cargo search shiplog-ids`
   - `cargo search shiplog`
2. Confirm GitHub release and tag are visible.
3. Confirm changelog compare links remain valid:
   - `[Unreleased]: .../compare/v0.2.0...HEAD`
   - `[0.2.0]: .../compare/v0.1.1...v0.2.0`

## Rollback / Contingency Guidance

- If a publish fails mid-sequence:
  - Do not republish already-published versions.
  - Continue with remaining crates after fixing root cause.
  - If dependency ordering is the issue, wait until crates.io index catches up and retry.
- If tag/release is wrong:
  - Avoid rewriting published crate versions.
  - Create a follow-up patch release (`0.2.1`) with corrective changes.

## Evidence Pointers

- Version source: `Cargo.toml`
- Release notes: `CHANGELOG.md`
- Publishing guidance: `CLAUDE.md`
- CI release build/publish canary: `.github/workflows/ci.yml`

## Handoff Status

Ready for release execution by maintainer.
