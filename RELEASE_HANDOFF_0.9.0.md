# shiplog 0.9.0 - Paused Release Handoff

**Planned tag:** `v0.9.0` (not created)
**Status:** paused after the shipped `v0.8.0` release
**Hold receipt:** [`docs/release/0.9.0-release-hold.md`](docs/release/0.9.0-release-hold.md)
**Readiness ledger:** [`docs/release/0.9.0-readiness.md`](docs/release/0.9.0-readiness.md)

> Review-ready packet, Guided Setup / Doctor, and Review-loop Status work are
> on `main` as an unreleased 0.9 candidate. Do not tag or publish 0.9.0 from
> this handoff.

## Current State

- `v0.8.0` is the latest shipped release.
- PR #319 prepared 0.9.0 version metadata and release docs.
- PRs #424-#436 added the review-loop status cockpit, JSON contract, proof
  coverage, dogfood transcript, recurring guide, and README/status alignment.
- PRs #437-#440 curated the release-facing changelog, READMEs, and release
  docs around the 0.9 review-loop story.
- No `v0.9.0` tag exists from this handoff.
- No 0.9.0 GitHub release exists from this handoff.
- No 0.9.0 crates.io publish was performed from this handoff.

## Candidate Contents On Main

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
- The recurring review-loop guide teaches status-first weekly/monthly operation.
- The front-door product proof covers cold intake through share posture
  explanation without provider mutation.

See [`CHANGELOG.md`](CHANGELOG.md) `[0.9.0]` for the candidate entry list.

## Blocked While Hold Is Active

Do not run release execution for 0.9.0 while the hold receipt is active:

- do not create or push `v0.9.0`;
- do not publish `shiplog 0.9.0` to crates.io;
- do not create, undraft, or mark latest a 0.9.0 GitHub release;
- do not manually dispatch `release.yml` for `v0.9.0`;
- do not run release-install smoke against 0.9.0 assets.

## Resume Criteria

Resume 0.9 release execution only for a concrete reason:

- a security fix;
- broken 0.8 behavior;
- a major UX bug in the shipped repair loop;
- enough validated post-0.8 value to justify another release.

## Before Any Future Resume

- Update [`docs/release/0.9.0-release-hold.md`](docs/release/0.9.0-release-hold.md)
  with the release-resume decision.
- Record explicit owner approval for release execution.
- Rerun package version, package boundary, publish dry-run, product proof,
  docs/share/runs CLI tests, clippy, policy gates, no-panic, and local install
  smoke from current `main`.
- If using manual `release.yml` dispatch, provide a semver `release_tag` and set
  `owner_approved_release_execution`; branch refs are not release inputs.
- Replace paused handoff wording with fresh copy-ready tag, publish, GitHub
  release, and post-tag smoke commands.

## Known Non-Blockers

- The candidate intentionally does not write review prose, score employees, or
  use LLMs for claim generation.
- Provider setup remains an operator action followed by another intake run.
- Historical 0.6 implementation crates remain historical; do not yank them as
  routine cleanup.

## Owner

`EffortlessMetrics`. Release driver: project owner.
