# shiplog 0.8.0 - Release Handoff

**Tag:** `v0.8.0`
**Date:** 2026-05-15

> Evidence repair loop release.

This is the short-form handoff. The full readiness ledger is at
[`docs/release/0.8.0-readiness.md`](docs/release/0.8.0-readiness.md).

## Shipped State

- `v0.8.0` is tagged at `b72a5dda9e965ab7bae4381d586499424071c7dc`.
- GitHub release:
  <https://github.com/EffortlessMetrics/shiplog/releases/tag/v0.8.0>
- crates.io:
  <https://crates.io/crates/shiplog/0.8.0>
- Release workflow:
  <https://github.com/EffortlessMetrics/shiplog/actions/runs/25934420182>
- The Evidence Repair Loop goal is archived at
  [`.shiplog/goals/archive/2026-05-15-evidence-repair-0.8.0.toml`](.shiplog/goals/archive/2026-05-15-evidence-repair-0.8.0.toml).

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

- [x] Release-prep PR merged into `main`.
- [x] `main` CI green.
- [x] Package version audit passed.
- [x] Package boundary audit passed.
- [x] Publish dry-run passed.
- [x] `cargo test -p shiplog --test front_door_first_pack_smoke` passed.
- [x] `cargo test -p shiplog --test intake_cold_start` passed.
- [x] `cargo test -p shiplog --test cli_integration -- repair` passed.
- [x] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` passed.
- [x] `cargo xtask check-policy-schemas` passed.
- [x] `cargo xtask check-file-policy --mode blocking-allowlist` passed.
- [x] `cargo xtask check-executable-files --mode blocking-allowlist` passed.
- [x] `cargo xtask check-no-panic-family --mode blocking-allowlist` passed.
- [x] Local cargo-install smoke reports `shiplog 0.8.0`.
- [x] `git diff --check` passed.

## Tag And Release

```bash
git switch main
git pull --ff-only
git tag v0.8.0
git push origin v0.8.0
```

`release.yml` built the multi-platform binaries, uploaded release assets, and
completed release validation plus release-mode integration tests.

## Publish To crates.io

Publishing required explicit approval and a Cargo registry credential.

```bash
CARGO_REGISTRY_TOKEN=<token> cargo publish -p shiplog --locked
```

Do not publish `shiplog-testkit` or `xtask`.

## Publish GitHub Release

After `release.yml` uploaded release assets and crates.io publish completed,
the GitHub release was confirmed non-draft, non-prerelease, and marked latest:

```bash
gh release view v0.8.0
gh release edit v0.8.0 --draft=false --latest
```

`scripts/verify-release.sh` expects the GitHub release to be non-draft and
non-prerelease. In the Windows release session, the bash verifier timed out, so
the equivalent public release, checksum, release-asset, crates.io-install, and
binary smoke checks were run with PowerShell-native commands.

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
