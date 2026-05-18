# Setup-readiness dogfood matrix

> **Audience:** Guided Setup / Doctor hardening, agent control-plane planning,
> and release-hold review.
> **Status:** 0.9 remains paused. This matrix does not approve tag, publish,
> GitHub release creation, release workflow dispatch, or release-install smoke.
> **Companion docs:**
> [`docs/release/0.9.0-release-hold.md`](../release/0.9.0-release-hold.md),
> [`docs/release/0.9.0-readiness.md`](../release/0.9.0-readiness.md),
> [`docs/release/0.9.0-release-decision.md`](../release/0.9.0-release-decision.md),
> [`docs/guides/guided-setup-doctor.md`](../guides/guided-setup-doctor.md),
> [`review-ready dogfood matrix`](review-ready-dogfood-matrix.md).

This matrix turns #399-#410 into a control-plane ledger for setup readiness.
It is not a test harness and it is not release approval. It records which setup
flows are already covered, which still need proof, and which caveats are
accepted for the paused 0.9 candidate.

## Release posture

`v0.8.0` is the latest shipped release. The review-ready packet and Guided
Setup / Doctor work on `main` are an unreleased 0.9 candidate. Release
execution stays blocked while the hold receipt exists.

Do not use this matrix as permission to run:

- `git tag v0.9.0`
- `git push origin v0.9.0`
- `cargo publish -p shiplog --locked`
- `gh release edit v0.9.0 --draft=false --latest`
- manual `release.yml` dispatch for `v0.9.0`
- release-install smoke against `v0.9.0` assets

## Control-plane boundary

Setup readiness is a prerequisite signal. It is deliberately separate from:

- source freshness;
- packet readiness;
- repair clearance;
- share posture.

Doctor may say GitHub setup is unavailable because `GITHUB_TOKEN` is missing.
That is not a weak GitHub evidence claim. It means the GitHub evidence path is
not ready for intake.

The intended front-door flow is:

```bash
shiplog init --guided
shiplog doctor --setup
shiplog sources status
shiplog doctor --setup --json
shiplog intake --last-6-months --explain
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog runs diff --latest
shiplog share explain manager --latest
```

Doctor, `sources status`, and `doctor --setup --json` should help humans and
agents choose one of these safe next moves without scraping text or treating
setup state as evidence state:

- run intake;
- fix a local file first;
- ask for a credential;
- stay local-only;
- avoid a write-producing repair command;
- avoid share rendering until redaction setup is ready.

## Matrix

| Row | Flow | Trusted proof | Missing proof | Accepted caveat |
| --- | --- | --- | --- | --- |
| 1 | Empty directory | Missing config is modeled as setup work with `init --guided` as the write-producing next action (#402, #403). `init --guided` creates `shiplog.toml` and `manual_events.yaml` (#405). | A refreshed transcript should show the exact first-run text after future wording changes. | Empty directories are allowed to start with setup writes; doctor itself remains read-only. |
| 2 | Git repo with no config | `doctor --setup` reports missing config instead of trying intake, and `init --guided` enables local Git when the current directory is a readable repo (#402, #403, #405). | Keep proof that a no-config Git repo does not trigger provider checks or packet claims before setup exists. | The first useful command is `init --guided`, which writes local setup files. |
| 3 | Git repo with guided config | Guided setup creates a local-first `shiplog.toml` and valid `manual_events.yaml`; doctor can report local Git/manual ready and manager/public share blocked by missing redaction setup (#405, #407, #408). | PR #420 should record the full guided setup transcript through repair diff and share explain. | Guided config makes setup usable, not evidence strong; packet readiness remains intake/report territory. |
| 4 | Malformed manual journal | The setup model marks malformed manual journals as blocked, and the product proof shows doctor catches the setup block before repair resumes (#402, #403, #408). | PR #415 should pin the full `doctor -> intake -> repair plan` route so `journal add --from-repair` cannot appear while the journal is malformed. | Manual journal repair is a setup fix first; evidence repair resumes after schema validity returns. |
| 5 | Disabled manual source | The setup model does not validate a disabled manual journal as malformed, and unit coverage pins the disabled-manual behavior (#402). | PR #420 should keep disabled manual setup visible in a front-door transcript or accepted caveat. | Disabled means intentionally out of scope for setup, not a hidden evidence failure. |
| 6 | Enabled GitHub without token | `sources status` reports enabled provider token gaps as unavailable without calling provider APIs or printing token values (#404). Token presence is tested without secret disclosure. | PR #414 should prove doctor and `sources status` agree on source keys, labels, enabled flags, and unavailable token status. | Env-var presence is setup readiness only; intake owns evidence fetching and source freshness. |
| 7 | Manager share without redaction key | Doctor reports manager share blocked by missing `SHIPLOG_REDACT_KEY` without rendering profile artifacts (#407). `share explain manager` remains read-only from the review-ready lane. | PR #417 should pin agreement among doctor, `share explain manager`, `share verify manager`, and manager rendering for the same redaction blocker. | Setting an env var may be shown as read-only guidance because Shiplog does not write the secret. |
| 8 | Public share cautious path | Doctor reports public share redaction setup and strict-verification caveats without rendering profile artifacts (#407). Public share explain/verify remains separated from rendering in the review-ready matrix. | PR #417 should keep public and manager share readiness aligned without leaking redaction key values. | Doctor cannot prove public packet safety before a public packet exists and strict verification has run. |
| 9 | Old config / old report compatibility | Setup readiness is intentionally not embedded into old intake reports, and #410 records doctor/setup as candidate scope without changing release posture. Existing old-report tests keep packet-quality compatibility separate from setup readiness. | PR #420 should harden older setup configs: missing optional provider tokens unavailable, disabled manual source not validated, missing paths unavailable, stale config reserved for incompatible settings. | Doctor may inspect durable reports only when a future command explicitly asks for that; current setup readiness comes from local setup state. |
| 10 | Windows path/env-var display | Guided setup and share-readiness docs include PowerShell env-var examples, and Windows CI runs the CLI/doc tests. Prior post-0.8 soak fixed noisy Windows path display in intake handoffs (#375, #376). | PR #419 should only change wording if a transcript shows Windows path or env-var text causing confusion. | Shell-specific examples are documentation and guidance; Shiplog must not print secret values. |

## Targeted proof backlog

The next implementation PRs should keep this ledger narrow:

- doctor/source-status consistency tests;
- setup-blocked repair proof that stays read-first while local setup is broken;
- share readiness consistency across doctor, share explain, share verify, and
  share rendering;
- setup-aware intake handoff only when setup is genuinely blocked;
- old-config and old-report compatibility hardening.

## Release decision use

Use this matrix to decide what setup readiness still needs proof before any
future 0.9 release-resume discussion. It is not itself the release decision.

Current decision: keep the `v0.9.0` hold active (#398, updated by #410).

The setup lane is ready for a release posture decision only when:

- every row has current proof or an explicit accepted caveat;
- setup readiness JSON exists and schema/examples are pinned;
- doctor and `sources status` cannot drift on source identity/status;
- setup-blocked repair and share paths remain read-first/no-write-safe;
- `v0.9.0` still has not been tagged, published to crates.io, or released on
  GitHub;
- the owner explicitly approves any release execution.
