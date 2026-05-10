# shiplog 0.5.0 — Release Handoff

**Tag:** `v0.5.0` · **Date:** 2026-05-10

> Operational Review Rescue + Rust 1.95 quality floor + policy/CI economics foundation.

This is the short-form handoff. The full readiness ledger is at
[`docs/release/0.5.0-readiness.md`](docs/release/0.5.0-readiness.md).

## What ships

- The operational hardening lane already merged after v0.4.0 (PRs #125–#139).
- MSRV bump from Rust 1.92 to **Rust 1.95** (#145).
- 18 policy ledgers under `policy/` covering CI lanes, budget, risk packs,
  exceptions, Clippy, no-panic, file-policy companions, and ripr
  suppressions.
- A thin Rust-native `xtask` runner (`cargo xtask <task>`) that powers all
  policy enforcement and CI plan/actuals collection.
- LEM-budgeted CI lane plan (advisory) + ci-actuals collector that closes
  the forecast/actual loop.
- `ripr` advisory lane (v1 stub; real analysis is a follow-up).
- Bounded smoke lanes for the PR-fast tier; broad evidence sweeps routed
  off PR-on-every behind labels + nightly + dispatch.
- Four `cold_path()` hints on fail-closed redaction error paths (#156).

See [`CHANGELOG.md`](CHANGELOG.md) `[0.5.0]` for the full entry list.

## Pre-tag checklist

- [x] PR #157 merged into `main`.
- [ ] `main` CI green (`Check (ubuntu)`, `Check (windows)`, `MSRV (1.95)`,
  `cargo-deny`, `forecast`, three smokes, `advisory` (ripr), `CodeRabbit`,
  `GitGuardian`, `droid-review`).
- [ ] `ci-actuals.yml` recorded actuals for the merge commit.
- [ ] `scripts/package-proof.sh` PASS on `main`.
- [ ] `scripts/publish-dry-run.sh` PASS on `main`.
- [ ] `cargo xtask policy-report` clean.
- [ ] `scripts/demo-review-rescue.sh` PASS locally.
- [ ] `scripts/demo-review-rescue.ps1` PASS locally on a Windows host.

## Tag + release

```bash
git switch main
git pull --ff-only
git tag v0.5.0
git push origin v0.5.0
```

`release.yml` builds Linux x86_64, macOS x86_64, macOS arm64, and Windows
x86_64 artifacts and creates the GitHub release with checksums.

## Post-tag smoke

```bash
scripts/release-install-smoke.sh v0.5.0
scripts/release-install-smoke.ps1 -Tag v0.5.0
```

## Publish to crates.io

Foundation → adapters → engine → CLI (see readiness doc for the full
order). Each crate:

```bash
cargo publish -p <crate> --dry-run   # if not already proven
cargo publish -p <crate>
```

## Known non-blockers

- `lane.ci_msrv` is redundant with `lane.ci_check` while toolchain pin ==
  MSRV; candidate to drop in a follow-up.
- `clippy::disallowed_fields` is `[[planned]]` until protected seams are
  configured.
- ripr is a v1 stub.
- Risk-pack auto-application of the `mutation` label on severe ripr
  findings is deferred.

## Rollback

See the readiness doc's [Rollback path](docs/release/0.5.0-readiness.md#rollback-path)
section.

## Owner

`EffortlessMetrics`. Release driver: project owner.
