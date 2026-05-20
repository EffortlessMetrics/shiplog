# GitHub activity harvest completion audit

> **Status:** GitHub activity harvest completion audit for the paused 0.9
> candidate.
> **Release posture:** this audit does not approve tagging, publishing,
> GitHub release execution, release workflow dispatch, or release-install
> smoke.
> **Date:** 2026-05-20.

This records the GitHub activity harvest lane after the receipt-backed workflow
landed through status, report, merge, and resume proof. It is release proof, not
a release decision.

## Release Proof Map

For 0.9 release review, this audit proves the advanced GitHub activity harvest
workflow is no longer an operator-only recipe:

- plan writes a static scope receipt before spending provider API budget;
- scout runs the cheap search-only pass first;
- authored and full profiles make the expensive legs explicit;
- progress and per-window receipts make resume decisions concrete;
- API ledger receipts separate search/core requests and cache phases;
- owner filtering is actor-first and receipt-backed, not repository crawling;
- activity status reads plan/progress/API-ledger receipts without writing;
- activity report writes explicit JSON/Markdown report artifacts without
  provider calls;
- activity merge writes final activity outputs from a completed run without
  rendering manager or public share artifacts;
- product tests prove completed resume does not refetch provider data.

Use this as release proof alongside the readiness ledger. It does not lift the
`v0.9.0` release hold.

## Lane Map

| PR | Layer | Landed receipt or proof |
| --- | --- | --- |
| #444 | Proposal | Defines the GitHub activity harvest as `plan -> scout -> run -> resume -> merge -> report API cost`. |
| #445 | Spec | Defines `github.activity.plan.json`, `github.activity.progress.json`, `github.activity.api-ledger.json`, `github.activity.report.json`, profiles, budgets, cache phases, owner filters, and merge rules. |
| #446 | ADR | Records the actor-first, owner-filtered architecture and rejects repository crawling as the default. |
| #447 | Plan command | Adds `shiplog github activity plan` and the static plan receipt. |
| #448 | Search cache | Caches GitHub search probes and pages separately from detail/review cache phases. |
| #449 | Budget guardrails | Adds search/core budget policy and checkpoint-before-stop behavior. |
| #450 | Owner receipts | Adds requested, kept, and dropped owner-filter receipts. |
| #451 | Profiles/progress | Adds scout/authored/full activity runs and `github.activity.progress.json`. |
| #452 | API ledger | Adds `github.activity.api-ledger.json` with request, cache, rate-limit, limit-event, and owner-filter evidence. |
| #453 | Guide | Adds the full GitHub activity harvest guide for the `EffortlessSteven` / `EffortlessMetrics` / `EffortlessSteven` case. |
| #454 | Resume proof | Proves completed progress plus API-ledger receipts allow `--resume` to skip provider refetch. |
| #455 | Status/report/merge | Adds activity `status`, `report`, `merge`, report schema/docs/example, and final activity outputs. |

## Current User Workflow

The landed advanced workflow is:

```bash
shiplog github activity plan --config shiplog-github-full.toml
shiplog github activity scout --config shiplog-github-full.toml --resume
shiplog github activity run --config shiplog-github-full.toml --profile authored --resume
shiplog github activity run --config shiplog-github-full.toml --profile full --resume
shiplog github activity status --out ./out/github-full
shiplog github activity report --out ./out/github-full
shiplog github activity merge --out ./out/github-full
```

This is an advanced harvest path. The default review-readiness loop remains
setup, status, intake, repair, rerun, diff, and share explain.

## Receipts

The activity harvest receipt stack is:

```text
github.activity.plan.json
github.activity.progress.json
github.activity.api-ledger.json
github.activity.report.json
github.activity.report.md
github.activity.windows/<profile>/<window_id>/ledger.events.jsonl
github.activity.windows/<profile>/<window_id>/coverage.manifest.json
github.activity.windows/<profile>/<window_id>/freshness.json
final/packet.md
final/github.activity.report.json
final/github.activity.api-ledger.json
```

`final/intake.report.json` is present only when the completed activity run
produced one. Merge must not invent missing intake reports or share artifacts.

## What Is Proven

- `shiplog github activity plan` writes `github.activity.plan.json` without
  fetching PR details, fetching review pages, rendering packets, mutating
  provider records, or executing release work.
- `shiplog github activity scout` writes progress and API-ledger receipts for
  the search-only phase and checkpoints before budget exhaustion stops a run.
- `shiplog github activity run --profile authored --resume` fetches authored
  PR detail work explicitly and can skip completed work on resume.
- `shiplog github activity run --profile full --resume` is the explicit review
  fidelity path, including review candidate search and review pages.
- Completed windows are trusted only when `ledger.events.jsonl`,
  `coverage.manifest.json`, and `freshness.json` are readable under
  `github.activity.windows/<profile>/<window_id>/`.
- `shiplog github activity status` reads existing plan/progress/API-ledger
  receipts and remains read-only.
- `shiplog github activity report` reads the same receipts and writes
  `github.activity.report.json` plus `github.activity.report.md`.
- `shiplog github activity merge` requires completed progress with a run
  reference, writes final outputs, copies the API ledger, and writes the final
  activity report.
- Activity status/report/merge do not call GitHub, mutate provider records,
  scrape `packet.md`, call an LLM, render share artifacts, or execute release
  work.
- The generated activity report and API ledger schemas include property-name
  hygiene, and tests keep secret sentinels out of examples and generated JSON.

## Cost And Coverage Signals

The lane makes API cost auditable through `github.activity.api-ledger.json` and
the activity report:

```text
search/core request counts
search_probe/search_page/pull_detail/review_page cache counts
rate-limit snapshots
secondary-limit events
requested owners
kept owners
dropped owners
stop reason
receipt refs
```

These are cost and inclusion receipts. They do not claim perfect GitHub
coverage by themselves. Dense or partial windows still need to be split,
resumed, or accepted with caveats.

## Current Caveats

- `shiplog github activity merge` is bounded by the completed run referenced by
  `github.activity.progress.json`; broader cross-run consolidation remains a
  future refinement.
- GitHub activity harvest is an advanced workflow, not the default first-run
  intake path.
- The lane does not add OAuth, provider mutation, repository crawling by
  default, dashboards, TUI, scheduling, team rollups, or generated review
  prose.
- The release-facing docs still need a final 0.9 refresh before release
  execution so the changelog, README, crate README, readiness ledger, handoff,
  and guide map agree on the final candidate scope.
- Swarm development cutover is a separate infrastructure lane and remains
  outside this product proof.

## Intentionally Not Done

- Did not tag `v0.9.0`.
- Did not publish to crates.io.
- Did not create a GitHub release.
- Did not dispatch the release workflow.
- Did not run release-install smoke.
- Did not move release/publish/signing authority away from the source repo.

## Release Decision Signal

This audit supports treating GitHub activity harvest as a landed, receipt-backed
advanced workflow for the 0.9 candidate scope. A release decision still needs:

- final release-doc refresh after this audit;
- current main CI and preflight;
- resolved release-hold checklist;
- explicit owner approval for release execution.
