# shiplog 0.7.0 - Release Handoff

**Tag:** `v0.7.0`
**Date:** 2026-05-14

> Crate-surface contraction release.

This is the short-form handoff. The full readiness ledger is at
[`docs/release/0.7.0-readiness.md`](docs/release/0.7.0-readiness.md).

## What Ships

- `shiplog` is the only supported 0.7 public crate by default.
- Former 0.6 implementation crates are historical surfaces and internal SRP
  modules going forward.
- `shiplog-schema` is internal for 0.7; `contracts/schemas/` remains the public
  machine contract.
- Package-boundary tooling fails if unsupported workspace packages or
  historical 0.6 implementation crates re-enter the forward publish graph.
- The 0.6 first-run review-pack behavior remains the product guardrail.

See [`CHANGELOG.md`](CHANGELOG.md) `[0.7.0]` for the full entry list.

## Pre-Tag Checklist

- [ ] Release-prep PR merged into `main`.
- [ ] `main` CI green.
- [ ] `bash scripts/package-version-audit.sh` passed.
- [ ] `bash scripts/package-boundary-audit.sh` passed.
- [ ] `bash scripts/publish-dry-run.sh` passed.
- [ ] `cargo test -p shiplog --test intake_cold_start` passed.
- [ ] `cargo test -p shiplog --test front_door_first_pack_smoke` passed.
- [ ] `cargo test -p shiplog --test cli_integration -- intake` passed.
- [ ] `cargo test -p shiplog --test cli_integration -- report` passed.
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` passed.
- [ ] `cargo xtask check-policy-schemas` passed.
- [ ] `cargo xtask check-file-policy --mode blocking-allowlist` passed.
- [ ] `cargo xtask check-executable-files --mode blocking-allowlist` passed.
- [ ] `git diff --check` passed.

## Tag And Release

```bash
git switch main
git pull --ff-only
git tag v0.7.0
git push origin v0.7.0
```

`release.yml` builds the multi-platform binaries and creates a draft GitHub
release. Wait for the workflow to upload assets before publishing the draft.

## Publish To crates.io

Publishing requires explicit approval and `CARGO_REGISTRY_TOKEN`.

```bash
CARGO_REGISTRY_TOKEN=<token> cargo publish -p shiplog --locked
```

Do not run `scripts/publish-v0.5.0.sh` or `scripts/publish-v0.6.0.sh`.

## Publish GitHub Release

After `release.yml` has uploaded the release assets and crates.io publish has
completed, publish the draft GitHub release:

```bash
gh release view v0.7.0
gh release edit v0.7.0 --draft=false --latest
```

`scripts/verify-release.sh` expects the GitHub release to be non-draft and
non-prerelease.

## Post-Tag Smoke

```bash
scripts/release-install-smoke.sh v0.7.0
scripts/release-install-smoke.ps1 -Version v0.7.0
scripts/verify-release.sh v0.7.0
```

## Known Non-Blockers

- The Evidence Repair Loop starts after 0.7 release closure.
- 0.6 implementation crates remain historical; do not yank them as routine
  cleanup.

## Rollback

See the readiness doc's rollback section:
[`docs/release/0.7.0-readiness.md#rollback-path`](docs/release/0.7.0-readiness.md#rollback-path).

## Owner

`EffortlessMetrics`. Release driver: project owner.
