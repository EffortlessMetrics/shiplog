# shiplog 0.6.0 - Release Handoff

**Tag:** `v0.6.0`
**Date:** 2026-05-13

> User-polish review-pack release.

This is the short-form handoff. The full readiness ledger is at
[`docs/release/0.6.0-readiness.md`](docs/release/0.6.0-readiness.md).

## What Ships

- First-run intake prints the review-pack output directory and next-step
  commands.
- Latest-artifact opening works for `intake-report`, `packet`, and `out`.
- Intake report JSON has canonical `source_key` and `source_label` fields for
  source-facing receipts, with the v1 `source` alias retained.
- Skipped configured sources appear in `source_freshness` with reasons.
- `ApiCache::lookup` exposes `Fresh`, `Stale`, and `Miss`; adapters emit
  `stale` only when a stale cache row is proven.
- Recorded GitHub HTTP fixtures prove fresh-then-cached warm reruns without
  live network access.
- The proposal/spec/ADR/plan/active-goal source-of-truth stack is in the repo
  and points future agents at the next proof command instead of chat history.

See [`CHANGELOG.md`](CHANGELOG.md) `[0.6.0]` for the full entry list.

## Pre-Tag Checklist

- [ ] Release-prep PR merged into `main`.
- [ ] `main` CI green for `Check (ubuntu-latest)`, `Check (windows-latest)`,
  `Policy gates`, and `cargo-deny`.
- [ ] `cargo test -p shiplog --test intake_cold_start` passed.
- [ ] `cargo test -p shiplog --test front_door_first_pack_smoke` passed.
- [ ] `cargo test -p shiplog --test cli_integration -- intake` passed.
- [ ] `cargo test -p shiplog --test cli_integration -- report` passed.
- [ ] `cargo test -p shiplog-cache` passed.
- [ ] `cargo test -p shiplog-ingest-github` passed.
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` passed.
- [ ] `cargo xtask package-version` passed, or native metadata inspection
  verified every workspace package at `0.6.0` and every normal workspace
  dependency requirement at `^0.6.0`.
- [ ] `cargo xtask check-policy-schemas` passed.
- [ ] `cargo xtask check-file-policy --mode blocking-allowlist` passed.
- [ ] `git diff --check` passed.

## Tag And Release

```bash
git switch main
git pull --ff-only
git tag v0.6.0
git push origin v0.6.0
```

`release.yml` builds the multi-platform binaries and creates the GitHub
release.

## Publish To crates.io

Publishing requires explicit approval and `CARGO_REGISTRY_TOKEN`.

```bash
CARGO_REGISTRY_TOKEN=<token> bash scripts/publish-v0.6.0.sh
```

Release closure note: the shipped release used this script through
`shiplog-ingest-linear`; publishing then paused because `shiplog-team` was
listed before `shiplog-merge`. The release driver resumed manually with
`shiplog-merge`, `shiplog-engine`, `shiplog-team`, and `shiplog`. The script is
now corrected on `main`.

## Post-Tag Smoke

```bash
scripts/release-install-smoke.sh v0.6.0
scripts/release-install-smoke.ps1 -Version v0.6.0
```

## Known Non-Blockers

- Protected-fields and `disallowed_fields` stay out of this lane.
- `source` remains as a report v1 compatibility alias next to `source_key`.
- `.shiplog/goals/active.toml` was archived after the release shipped.

## Rollback

See the readiness doc's rollback section:
[`docs/release/0.6.0-readiness.md#rollback-path`](docs/release/0.6.0-readiness.md#rollback-path).

## Owner

`EffortlessMetrics`. Release driver: project owner.
