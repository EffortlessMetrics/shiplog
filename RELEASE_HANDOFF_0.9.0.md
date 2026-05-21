# shiplog 0.9.0 - Release Execution Handoff

**Planned tag:** `v0.9.0` (not created yet)
**Status:** release resume approved; final merged-main preflight pending
**Approval date:** 2026-05-20
**Lifted hold receipt:** [`docs/release/0.9.0-release-hold-lifted.md`](docs/release/0.9.0-release-hold-lifted.md)
**Readiness ledger:** [`docs/release/0.9.0-readiness.md`](docs/release/0.9.0-readiness.md)

> Owner approval to resume `v0.9.0` release execution was recorded on
> 2026-05-20. This handoff prepares the post-merge release steps; it does not
> claim that the tag, crates.io publish, GitHub release, or install smoke have
> already happened.

## Current State

- `v0.8.0` remains the latest shipped release until `v0.9.0` is tagged,
  published, and released.
- PR #319 prepared 0.9.0 version metadata and release docs.
- PRs #424-#436 added the review-loop status cockpit, JSON contract, proof
  coverage, dogfood transcript, recurring guide, and README/status alignment.
- PRs #437-#440 curated the release-facing changelog, READMEs, and release
  docs around the 0.9 review-loop story.
- PRs #444-#455 added budgeted GitHub activity harvest: plan, scout, authored
  and full profile runs, resume, API ledger, status, report, merge, guide, and
  product proof.
- PR #460 recorded the GitHub activity harvest completion audit.
- PR #310 aligned deterministic redaction aliases with the documented
  HMAC-SHA256 primitive while preserving cached aliases.
- No `v0.9.0` tag exists from this handoff.
- No 0.9.0 GitHub release exists from this handoff.
- No 0.9.0 crates.io publish was performed from this handoff.

## Release Contents On Main

- `intake.report.json` carries compatible v1 `packet_quality` data for packet
  readiness, evidence strength, claim candidates, and share posture fields.
- `packet.md` opens with packet readiness and renders receipt-backed claim
  candidates with missing-context prompts.
- `shiplog share explain manager|public` explains included, removed, blocked,
  and needs-review posture without requiring a redaction key or writing profile
  artifacts.
- `shiplog runs diff --latest` shows packet quality movement across reruns.
- `shiplog init --guided`, `shiplog doctor --setup`, and
  `shiplog sources status` provide the guided setup front door before intake.
- `shiplog doctor --setup --json` exposes setup readiness for agents and
  scripts without scraping terminal prose.
- `shiplog status --latest` prints the read-only review-loop cockpit over
  setup, latest run, packet readiness, source state, repair, diff, share
  posture, blockers, next actions, and receipt refs.
- `shiplog status --latest --json` exposes the same status model for agents and
  scripts.
- `review-loop-status.v1` schema docs and examples pin the status JSON
  contract.
- Status consistency and safe-next-action proofs cover doctor JSON, sources
  status, intake reports, repair plan, repair diff, runs diff, and share
  explain.
- Setup-blocked repairs route to doctor/source-status before repair action, and
  doctor reports manager/public share setup readiness without rendering
  profiles.
- The review-ready packet guide explains the collect, repair, rerun, compare,
  interpret, share loop.
- The Guided Setup / Doctor guide explains local-only, manual-only,
  token-backed GitHub, manager-share-ready, and public-share-cautious modes.
- The recurring review-loop guide teaches status-first weekly/monthly
  operation.
- `shiplog github activity plan`, `scout`,
  `run --profile authored --resume`, `run --profile full --resume`, `status`,
  `report`, and `merge` provide the advanced full-history GitHub harvest path
  with plan/progress/API-ledger/report receipts.
- `github.activity.plan.v1`, `github.activity.progress.v1`,
  `github.activity.api-ledger.v1`, and `github.activity.report.v1` schema docs
  and examples pin the GitHub activity machine contracts.
- GitHub activity harvest status/report/merge are receipt-derived: no provider
  calls, no provider mutation, no packet scraping, no share rendering, and no
  release work.
- Redaction aliases use HMAC-SHA256 for new uncached aliases; existing cached
  aliases remain stable through `redaction.aliases.json`.
- The front-door product proof covers cold intake through share posture
  explanation without provider mutation.

See [`CHANGELOG.md`](CHANGELOG.md) `[0.9.0]` for the release entry list.

## Final Preflight From Merged Main

After the release-resume PR merges:

```bash
git switch main
git pull --ff-only

gh pr list --state open --limit 30
git status --short
git tag -l v0.9.0
git ls-remote --tags origin v0.9.0 || true
gh release view v0.9.0 || true

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo xtask check-no-panic-family --mode blocking-allowlist
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
cargo xtask check-generated --mode blocking-allowlist
git diff --check

bash scripts/package-proof.sh
bash scripts/publish-dry-run.sh
cargo publish -p shiplog --dry-run --locked
bash scripts/check-release-hold.sh v0.9.0
```

Stop before tagging if any check fails or if `v0.9.0` already exists locally,
remotely, or as a GitHub release.

## Release Execution After Green Preflight

```bash
git tag -a v0.9.0 -m "shiplog v0.9.0"
git push origin v0.9.0
```

Watch `release.yml` until package proof, binary builds, draft release creation,
asset upload, validation, and release-mode integration tests are green. Then:

```bash
cargo publish -p shiplog --locked
gh release edit v0.9.0 --draft=false --latest
```

## Install Smoke After Publish

```bash
cargo install shiplog --version 0.9.0 --locked --force
shiplog --version
shiplog --help
shiplog init --help
shiplog doctor --help
shiplog status --help
shiplog intake --help
shiplog repair --help
shiplog runs --help
shiplog share --help
shiplog github activity --help
```

Then verify public state:

```bash
cargo search shiplog --limit 5
gh release view v0.9.0 --json tagName,isDraft,isPrerelease,publishedAt,assets,url
```

## Stop Conditions

Stop before publishing if any of these happen:

- any open release-blocking PR appears;
- `v0.9.0` tag already exists unexpectedly;
- `v0.9.0` GitHub release already exists unexpectedly;
- publish dry-run fails;
- release workflow fails package proof, binary build, validation, or
  release-mode tests;
- `cargo publish` fails for anything other than "version already uploaded";
- release assets are missing `SHA256SUMS.txt` or a platform binary.

## Known Non-Blockers

- The release intentionally does not write review prose, score employees, or
  use LLMs for claim generation.
- Provider setup remains an operator action followed by another intake run.
- Historical 0.6 implementation crates remain historical; do not yank them as
  routine cleanup.

## Owner

`EffortlessMetrics`. Release driver: project owner.
