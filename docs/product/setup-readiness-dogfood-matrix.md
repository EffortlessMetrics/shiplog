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
> [`guided setup transcript`](guided-setup-transcript.md),
> [`review-ready dogfood matrix`](review-ready-dogfood-matrix.md).

This matrix turns #399-#410 into a control-plane ledger for setup readiness.
It is not a test harness and it is not release approval. It records which setup
flows are already covered, which still need proof, and which caveats are
accepted for the paused 0.9 candidate.

## Release Proof Map

For 0.9 release review, the rows that matter most are:

- empty directory and guided config (#405, #418);
- malformed manual journal routing to doctor before repair writes (#408, #415,
  #416);
- enabled provider without token staying unavailable, not weak evidence (#414,
  #415, #416);
- manager/public share blocked consistently by missing redaction setup (#407,
  #417);
- old setup/config compatibility staying calm for existing users (#420).

These are proof inputs for the release readiness ledger. They still do not
approve tag, publish, GitHub release creation, workflow dispatch, or
release-install smoke.

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
| 1 | Empty directory | Missing config is modeled as setup work with `init --guided` as the write-producing next action (#402, #403). `init --guided` creates `shiplog.toml` and `manual_events.yaml` (#405). The guided setup transcript starts from an empty temp workspace and proves init writes setup files while doctor, doctor JSON, and sources status create no output artifacts (#418). | Future wording passes should keep the exact first-run text aligned with the transcript. | Empty directories are allowed to start with setup writes; doctor itself remains read-only. |
| 2 | Git repo with no config | `doctor --setup` reports missing config instead of trying intake, and `init --guided` enables local Git when the current directory is a readable repo (#402, #403, #405). | Keep proof that a no-config Git repo does not trigger provider checks or packet claims before setup exists. | The first useful command is `init --guided`, which writes local setup files. |
| 3 | Git repo with guided config | Guided setup creates a local-first `shiplog.toml` and valid `manual_events.yaml`; doctor can report local Git/manual ready and manager/public share blocked by missing redaction setup (#405, #407, #408). The guided setup transcript proves the manual-first guided config path through intake, repair plan, journal repair, rerun, repair diff, runs diff, and read-only share explain (#418). Transcript follow-up coverage now proves local Git intake can derive repository identity from the workdir when guided config uses `repo = "."` without an origin remote (#419). | Future transcript proof should exercise full local Git plus manual repair together. | Guided config makes setup usable, not evidence strong; packet readiness remains intake/report territory. |
| 4 | Malformed manual journal | The setup model marks malformed manual journals as blocked, and the product proof shows doctor catches the setup block before repair resumes (#402, #403, #408). Doctor JSON now proves agents see the malformed manual journal as read-first setup work and do not receive `journal add --from-repair` while setup is broken (#415). Intake now starts its top-level handoff with `doctor --setup` and `sources status` before repair plan when manual setup is malformed (#416). | Future product proof should keep this visible in the full guided transcript. | Manual journal repair is a setup fix first; evidence repair resumes after schema validity returns. |
| 5 | Disabled manual source | The setup model does not validate a disabled manual journal as malformed, and unit coverage pins the disabled-manual behavior (#402). Setup compatibility tests now prove a disabled manual source stays disabled/optional even when `manual_events.yaml` is malformed (#420). | Future docs polish should keep disabled-source wording aligned with the CLI if labels change. | Disabled means intentionally out of scope for setup, not a hidden evidence failure. |
| 6 | Enabled GitHub without token | `sources status` reports enabled provider token gaps as unavailable without calling provider APIs or printing token values (#404). Token presence is tested without secret disclosure. Doctor JSON and `sources status` now agree on source keys, labels, enabled flags, status, and reasons (#414). Doctor JSON also exposes the missing token as a read-only setup action for agents (#415). Intake now routes configured provider-token gaps through doctor/source status before repair plan, while optional autodetected provider skips keep normal repair flow (#416). | Future provider-specific rows can expand this proof if new source setup states are added. | Env-var presence is setup readiness only; intake owns evidence fetching and source freshness. |
| 7 | Manager share without redaction key | Doctor reports manager share blocked by missing `SHIPLOG_REDACT_KEY` without rendering profile artifacts (#407). `share explain manager` remains read-only from the review-ready lane. Doctor JSON now proves agents see manager/public share as blocked and do not receive share render commands while redaction setup is missing (#415). Doctor, `share explain manager`, `share verify manager`, and manager rendering now agree on the same missing-key blocker without writing profile artifacts (#417). | Future wording passes should keep the blocker text clear if command labels change. | Setting an env var may be shown as read-only guidance because Shiplog does not write the secret. |
| 8 | Public share cautious path | Doctor reports public share redaction setup and strict-verification caveats without rendering profile artifacts (#407). Public share explain/verify remains separated from rendering in the review-ready matrix. Public share explain now stays aligned with doctor, public verify, and public rendering on the missing-key blocker while keeping the strict-review caveat visible (#417). | Future transcript proof should keep public review wording visible after rendering with a key. | Doctor cannot prove public packet safety before a public packet exists and strict verification has run. |
| 9 | Old config / old report compatibility | Setup readiness is intentionally not embedded into old intake reports, and #410 records doctor/setup as candidate scope without changing release posture. Existing old-report tests keep packet-quality compatibility separate from setup readiness. Setup compatibility tests now pin the older-config boundary: missing optional provider tokens and missing local paths are `unavailable`, disabled manual source state is not validated as malformed, unsupported config versions are the reserved `stale_config` case, and old report/packet artifacts do not drive doctor setup state (#420). | Future compatibility proof should expand only when new setup status fields or config versions are introduced. | Doctor may inspect durable reports only when a future command explicitly asks for that; current setup readiness comes from local setup state. |
| 10 | Windows path/env-var display | Guided setup and share-readiness docs include PowerShell env-var examples, and Windows CI runs the CLI/doc tests. Prior post-0.8 soak fixed noisy Windows path display in intake handoffs (#375, #376). | PR #419 should only change wording if a transcript shows Windows path or env-var text causing confusion. | Shell-specific examples are documentation and guidance; Shiplog must not print secret values. |

## Targeted proof backlog

The next implementation PRs should keep this ledger narrow:

- release-resume proof only after owner approval, if the hold is later lifted.

## Release decision use

Use this matrix to decide what setup readiness still needs proof before any
future 0.9 release-resume discussion. It is not itself the release decision.

Current decision: keep the `v0.9.0` hold active (#398, #410, #422, #440, and
#460).

The setup lane is ready for a release posture decision only when:

- every row has current proof or an explicit accepted caveat;
- setup readiness JSON exists and schema/examples are pinned;
- doctor and `sources status` cannot drift on source identity/status;
- setup-blocked repair and share paths remain read-first/no-write-safe;
- `v0.9.0` still has not been tagged, published to crates.io, or released on
  GitHub;
- the owner explicitly approves any release execution.
