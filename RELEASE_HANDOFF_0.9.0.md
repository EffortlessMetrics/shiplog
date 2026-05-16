# shiplog 0.9.0 - Release Handoff

**Tag:** `v0.9.0`
**Date:** 2026-05-15

> Review-ready packet quality release.

This is the short-form handoff. The full readiness ledger is at
[`docs/release/0.9.0-readiness.md`](docs/release/0.9.0-readiness.md).

## Prepared State

- `v0.9.0` is prepared but not tagged by this PR.
- GitHub release: not created yet.
- crates.io: not published yet.
- Previous release checkpoint: `v0.8.0`.

## What Ships

- `intake.report.json` carries compatible v1 `packet_quality` data for packet
  readiness, evidence strength, claim candidates, and share posture fields.
- `packet.md` opens with packet readiness and renders receipt-backed claim
  candidates with missing-context prompts.
- `shiplog share explain manager|public` explains included, removed, blocked,
  and needs-review posture without requiring a redaction key or writing profile
  artifacts.
- `shiplog runs diff --latest` shows packet quality movement across reruns.
- The review-ready packet guide explains the collect, repair, rerun, compare,
  interpret, share loop.
- The front-door product proof covers cold intake through share posture
  explanation without provider mutation.

See [`CHANGELOG.md`](CHANGELOG.md) `[0.9.0]` for the full entry list.

## Pre-Tag Checklist

- [ ] Release-prep PR merged into `main`.
- [ ] `main` CI green.
- [ ] Package version audit passed.
- [ ] Package boundary audit passed.
- [ ] Publish dry-run passed.
- [ ] `cargo test -p shiplog --test front_door_first_pack_smoke` passed.
- [ ] `cargo test -p shiplog --test docs_commands` passed.
- [ ] `cargo test -p shiplog --test cli_integration -- share` passed.
- [ ] `cargo test -p shiplog --test cli_integration -- runs` passed.
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` passed.
- [ ] `cargo xtask check-policy-schemas` passed.
- [ ] `cargo xtask check-file-policy --mode blocking-allowlist` passed.
- [ ] `cargo xtask check-executable-files --mode blocking-allowlist` passed.
- [ ] `cargo xtask check-no-panic-family --mode blocking-allowlist` passed.
- [ ] Local cargo-install smoke reports `shiplog 0.9.0`.
- [ ] `git diff --check` passed.

## Tag And Release

```bash
git switch main
git pull --ff-only
git tag v0.9.0
git push origin v0.9.0
```

`release.yml` should build the multi-platform binaries, upload release assets,
and complete release validation plus release-mode integration tests.

## Publish To crates.io

Publishing requires explicit approval and a Cargo registry credential.

```bash
CARGO_REGISTRY_TOKEN=<token> cargo publish -p shiplog --locked
```

Do not publish `shiplog-testkit` or `xtask`.

## Publish GitHub Release

After `release.yml` uploads release assets and crates.io publish completes,
confirm the GitHub release is non-draft, non-prerelease, and marked latest:

```bash
gh release view v0.9.0
gh release edit v0.9.0 --draft=false --latest
```

## Post-Tag Smoke

```bash
scripts/release-install-smoke.sh v0.9.0
scripts/verify-release.sh v0.9.0
```

On Windows PowerShell:

```powershell
scripts\release-install-smoke.ps1 -Version v0.9.0
```

## Known Non-Blockers

- The release intentionally does not write review prose, score employees, or use
  LLMs for claim generation.
- Provider setup remains an operator action followed by another intake run.
- Historical 0.6 implementation crates remain historical; do not yank them as
  routine cleanup.

## Rollback

See the readiness doc's rollback section:
[`docs/release/0.9.0-readiness.md#rollback-path`](docs/release/0.9.0-readiness.md#rollback-path).

## Owner

`EffortlessMetrics`. Release driver: project owner.

