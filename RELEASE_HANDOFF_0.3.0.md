# Release Handoff: v0.3.0

Date: 2026-05-07
Owner: Release handoff from Codex session
Target: crates.io publish + GitHub tag/release for `v0.3.0`

## Executive Summary

`shiplog` is prepared for a v0.3.0 release-readiness PR. This release turns the
post-v0.2.1 product work into a coherent installable surface:

- GitLab, Jira, Linear, local git, JSON, manual, and GitHub sources are
  CLI-visible.
- `collect multi`, `merge`, runs/open/cache commands, config validation and
  migration, and workstream curation commands are part of the product path.
- Packet rendering includes coverage/gap summaries, evidence anchors, claim
  prompts, render modes, receipt limits, and appendix controls.
- Manager/public share profiles fail closed without a real redaction key.
- Workspace package versions and normal workspace dependency requirements are
  aligned at `0.3.0`.

## Current Git State

- Base branch: `main`
- Release-readiness branch: `release/v0.3.0-readiness`
- Expected uncommitted changes from prep:
  - version bumps (`Cargo.toml` files + `Cargo.lock`)
  - `CHANGELOG.md`
  - `docs/CURRENT_STATE.md`
  - `plans/release-matrix-v0.3.0.md`
  - `plans/publish-order.md`
  - `scripts/package-version-audit.sh`
  - `scripts/package-proof.sh`
  - `.github/workflows/release.yml`
  - `RELEASE_HANDOFF_0.3.0.md`

## Tags and Versioning

- Existing release tags: `v0.2.1`, `v0.2.0`, `v0.1.1`, `v0.1.0`
- Expected new release tag: `v0.3.0`
- Workspace and workspace-package versions: `0.3.0`
- Dev-only `shiplog-testkit` is version-aligned but remains `publish = false`.
- `shiplog-fuzz` remains a non-publishable fuzz harness package at `0.0.0`.

## Publish Scope

Publishable crates, in order:

1. `shiplog-ids`
2. `shiplog-schema`
3. `shiplog-ports`
4. `shiplog-coverage`
5. `shiplog-cache`
6. `shiplog-redact`
7. `shiplog-bundle`
8. `shiplog-workstreams`
9. `shiplog-merge`
10. `shiplog-render-md`
11. `shiplog-render-json`
12. `shiplog-ingest-json`
13. `shiplog-ingest-manual`
14. `shiplog-ingest-git`
15. `shiplog-ingest-github`
16. `shiplog-ingest-gitlab`
17. `shiplog-ingest-jira`
18. `shiplog-ingest-linear`
19. `shiplog-cluster-llm`
20. `shiplog-team`
21. `shiplog-engine`
22. `shiplog`

Non-publishable:

- `shiplog-testkit`
- `shiplog-fuzz`

## Required Validation

Run these before merging the release-readiness PR:

```bash
cargo fmt --all -- --check
git diff --check
cargo metadata --format-version 1 --no-deps
scripts/package-boundary-audit.sh
scripts/package-version-audit.sh
cargo test -p shiplog --test docs_commands -- --test-threads=4
cargo test -p shiplog --no-default-features -- --test-threads=4
cargo check --manifest-path fuzz/Cargo.toml --bins
cargo deny check
cargo package -p shiplog --list --allow-dirty
```

For full release proof before publishing:

```bash
scripts/package-proof.sh
```

During real publication, dry-run and publish one crate at a time in dependency
order. A full `scripts/publish-dry-run.sh` pass is expected to complete only
after every upstream `0.3.0` dependency is visible in the crates.io index.

## Release Runbook

1. Merge the v0.3.0 readiness PR after local validation and hosted checks pass.
2. Sync `main`:

```bash
git switch main
git pull --ff-only origin main
```

3. Run final package proof:

```bash
scripts/package-proof.sh
```

4. Publish crates in the order listed above. For each crate:

```bash
cargo publish -p <package> --dry-run
cargo publish -p <package>
```

Retry downstream dry-runs after crates.io index propagation when needed. After
the final crate is published, `scripts/publish-dry-run.sh` should complete.
5. Create and push the tag:

```bash
git tag -a v0.3.0 -m "Release v0.3.0"
git push origin v0.3.0
```

6. Let the release workflow create the draft GitHub release and binary assets.
7. Verify release assets include Linux x86_64, macOS x86_64, macOS arm64,
   Windows x86_64, and `SHA256SUMS.txt`.

## Post-Release Verification

```bash
cargo install shiplog --version 0.3.0 --locked
shiplog --version
shiplog init --dry-run
shiplog collect --help
shiplog collect multi --help
shiplog render --help
```

Confirm:

- crates.io resolves `shiplog 0.3.0`.
- The GitHub draft release has checksums and all platform artifacts.
- `CHANGELOG.md` compare links are correct:
  - `[Unreleased]: .../compare/v0.3.0...HEAD`
  - `[0.3.0]: .../compare/v0.2.1...v0.3.0`

## Known Release Notes

- This release is a CLI/product surface release, not another package-boundary
  migration.
- LLM clustering remains optional and off by default.
- Manager/public packets and bundles require explicit redaction keys.
