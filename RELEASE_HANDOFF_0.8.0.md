# shiplog 0.8.0 - Release Handoff

**Tag:** `v0.8.0`
**Date:** 2026-05-15

> Evidence repair loop release.

This is the short-form handoff. The full readiness ledger is at
[`docs/release/0.8.0-readiness.md`](docs/release/0.8.0-readiness.md).

## What Ships

- `intake.report.json` carries receipt-derived repair items with stable IDs,
  keys, safe actions, clear conditions, and receipt references.
- `shiplog repair plan --latest` prints the latest report's repair queue.
- `shiplog journal add --from-repair <repair_id>` turns manual evidence repair
  items into local journal entries.
- `shiplog repair diff --latest` shows cleared, new, still-open, and changed
  repair states between compatible reports.
- The evidence repair guide and product proof show that a cold rough packet can
  become a better rerun packet without provider mutation.

See [`CHANGELOG.md`](CHANGELOG.md) `[0.8.0]` for the full entry list.

## Pre-Tag Checklist

- [ ] Release-prep PR merged into `main`.
- [ ] `main` CI green.
- [ ] `bash scripts/package-version-audit.sh` passed.
- [ ] `bash scripts/package-boundary-audit.sh` passed.
- [ ] `bash scripts/publish-dry-run.sh` passed.
- [ ] `cargo test -p shiplog --test front_door_first_pack_smoke` passed.
- [ ] `cargo test -p shiplog --test intake_cold_start` passed.
- [ ] `cargo test -p shiplog --test cli_integration -- repair` passed.
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` passed.
- [ ] `cargo xtask check-policy-schemas` passed.
- [ ] `cargo xtask check-file-policy --mode blocking-allowlist` passed.
- [ ] `cargo xtask check-executable-files --mode blocking-allowlist` passed.
- [ ] `cargo xtask check-no-panic-family --mode blocking-allowlist` passed.
- [ ] Local cargo-install smoke reports `shiplog 0.8.0`.
- [ ] `git diff --check` passed.

## Tag And Release

```bash
git switch main
git pull --ff-only
git tag v0.8.0
git push origin v0.8.0
```

`release.yml` builds the multi-platform binaries and creates a draft GitHub
release. Wait for the workflow to upload assets before publishing the draft.

## Publish To crates.io

Publishing requires explicit approval and `CARGO_REGISTRY_TOKEN`.

```bash
CARGO_REGISTRY_TOKEN=<token> cargo publish -p shiplog --locked
```

Do not publish `shiplog-testkit` or `xtask`.

## Publish GitHub Release

After `release.yml` has uploaded the release assets and crates.io publish has
completed, publish the draft GitHub release:

```bash
gh release view v0.8.0
gh release edit v0.8.0 --draft=false --latest
```

`scripts/verify-release.sh` expects the GitHub release to be non-draft and
non-prerelease.

## Post-Tag Smoke

```bash
scripts/release-install-smoke.sh v0.8.0
scripts/verify-release.sh v0.8.0
```

On Windows PowerShell:

```powershell
scripts\release-install-smoke.ps1 -Version v0.8.0
```

## Known Non-Blockers

- The repair loop intentionally does not mutate provider records.
- Historical 0.6 implementation crates remain historical; do not yank them as
  routine cleanup.

## Rollback

See the readiness doc's rollback section:
[`docs/release/0.8.0-readiness.md#rollback-path`](docs/release/0.8.0-readiness.md#rollback-path).

## Owner

`EffortlessMetrics`. Release driver: project owner.
