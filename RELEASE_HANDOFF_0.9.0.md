# shiplog 0.9.0 - Paused Release Handoff

**Planned tag:** `v0.9.0` (not created)
**Status:** paused after the shipped `v0.8.0` release
**Hold receipt:** [`docs/release/0.9.0-release-hold.md`](docs/release/0.9.0-release-hold.md)
**Readiness ledger:** [`docs/release/0.9.0-readiness.md`](docs/release/0.9.0-readiness.md)

> Review-ready packet work is on `main` as an unreleased 0.9 candidate. Do not
> tag or publish 0.9.0 from this handoff.

## Current State

- `v0.8.0` is the latest shipped release.
- PR #319 prepared 0.9.0 version metadata and release docs.
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
- The review-ready packet guide explains the collect, repair, rerun, compare,
  interpret, share loop.
- The front-door product proof covers cold intake through share posture
  explanation without provider mutation.

See [`CHANGELOG.md`](CHANGELOG.md) `[0.9.0]` for the candidate entry list.

## Blocked While Hold Is Active

Do not run release execution for 0.9.0 while the hold receipt is active:

- do not create or push `v0.9.0`;
- do not publish `shiplog 0.9.0` to crates.io;
- do not create, undraft, or mark latest a 0.9.0 GitHub release;
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
- Rerun package version, package boundary, publish dry-run, product proof,
  docs/share/runs CLI tests, clippy, policy gates, no-panic, and local install
  smoke from current `main`.
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

