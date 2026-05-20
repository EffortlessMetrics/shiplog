# GitHub activity harvest guide

Use this guide when you need a full GitHub activity log for one actor across
multiple repository owners without hand-managing API burn.

The workflow is:

```text
plan -> scout -> authored -> full -> status -> report -> merge
```

Release proof for the landed 0.9 candidate work is recorded in
[`docs/product/github-activity-harvest-completion-audit.md`](../product/github-activity-harvest-completion-audit.md).

This is an expensive-source workflow. It is actor-first and owner-filtered:
shiplog queries the GitHub actor, then records which repository owners were kept
or dropped. It does not crawl every repository in an organization by default.

## When to use this

Use GitHub activity harvest when you want:

- authored PR history across several years;
- review activity for the same GitHub login;
- a shared cache across scout and full-fidelity runs;
- resumable progress when budget or rate limits stop a run;
- API cost receipts that separate search, details, review pages, and cache
  reuse.

For normal review-cycle use, start with
[`recurring-review-loop.md`](recurring-review-loop.md). Use this guide only when
the GitHub source is the expensive part of the packet.

## Scope the actor and owners

The usual shape is one GitHub actor plus optional owner filters:

```toml
[shiplog]
config_version = 1

[defaults]
out = "./out/github-full"
profile = "internal"

[github_activity]
actor = "EffortlessSteven"
repo_owners = ["EffortlessMetrics", "EffortlessSteven"]
since = "2020-01-01"
until = "2026-05-20"
include_authored_prs = true
include_reviews = true
profile = "scout"
cache_dir = "./out/github-full/.cache"
cache_ttl_days = 3650

[github_activity.budget]
max_search_requests = 300
max_core_requests = 1000
max_search_per_minute = 24
on_exhausted = "checkpoint_and_stop"

[sources.github]
enabled = true
user = "EffortlessSteven"
mode = "created"
repo_owners = ["EffortlessMetrics", "EffortlessSteven"]
include_reviews = true
no_details = false
throttle_ms = 2500
cache_dir = "./out/github-full/.cache"
```

`actor` is the GitHub login. `repo_owners` is an inclusion scope for reporting
and filtering. If the token can see private repositories under both owners, one
actor query can collect activity across both.

Keep token values out of the config:

```powershell
$env:GITHUB_TOKEN = "<token>"
```

```bash
export GITHUB_TOKEN="<token>"
```

## Preflight setup

Run setup checks before spending API:

```bash
shiplog config validate --config shiplog-github-full.toml
shiplog doctor --config shiplog-github-full.toml --setup
shiplog sources status --config shiplog-github-full.toml
```

These commands do not query GitHub. They tell you whether setup is ready enough
to start an explicit harvest.

## Plan before spending API

Write the static plan:

```bash
shiplog github activity plan --config shiplog-github-full.toml
```

Plan writes:

```text
out/github-full/github.activity.plan.json
```

Plan does not call GitHub, fetch PR details, fetch review pages, render packets,
or create evidence run artifacts. It records:

- actor;
- repository owners;
- profile;
- date windows;
- planned query kinds;
- estimated search/core/review requests;
- budget policy;
- next executable command.

Use `--out` when the activity receipts should live outside the configured
default output root:

```bash
shiplog github activity plan --config shiplog-github-full.toml --out ./out/github-full
```

## Run scout first

Scout is the cheap first pass:

```bash
shiplog github activity scout --config shiplog-github-full.toml --resume
```

Scout uses the `scout` profile:

| Phase | Scout behavior |
| --- | --- |
| Authored PR search | Yes |
| PR details | No |
| Review search | No |
| Review pages | No |

Scout writes:

```text
out/github-full/github.activity.plan.json
out/github-full/github.activity.progress.json
out/github-full/github.activity.api-ledger.json
out/github-full/<run_id>/
```

Read the API ledger after scout. It should show search work and owner-filter
receipts without hiding token values.

## Add authored PR details

After scout, run authored:

```bash
shiplog github activity run --config shiplog-github-full.toml --profile authored --resume
```

Authored mode fetches authored PR details using the same actor/window/owner
scope and the warmed cache:

| Phase | Authored behavior |
| --- | --- |
| Authored PR search | Yes |
| PR details | Yes |
| Review search | No |
| Review pages | No |

If a matching completed progress receipt and API ledger already exist,
`--resume` skips the provider calls and reports that no provider calls were
made.

## Add reviews last

Run full only after scout and authored have warmed the cache:

```bash
shiplog github activity run --config shiplog-github-full.toml --profile full --resume
```

Full mode is the expensive path:

| Phase | Full behavior |
| --- | --- |
| Authored PR search | Yes |
| PR details | Yes |
| Review search | Yes |
| Review pages | Yes |

Review collection searches candidate PRs with `reviewed-by:<actor>`, then uses
review pages to filter by reviewer and date. That is more API-expensive than
authored PR search, so it belongs last.

## Inspect receipts

After each profile, inspect the receipts and latest run:

```bash
shiplog status --out ./out/github-full --latest
shiplog runs list --out ./out/github-full
shiplog open intake-report --out ./out/github-full --latest
shiplog open packet --out ./out/github-full --latest
```

Important receipts:

| Receipt | What it proves |
| --- | --- |
| `github.activity.plan.json` | Planned actor, owners, windows, profile, request estimates, and next action. Pinned by [`github.activity.plan.v1`](../schemas/github-activity-harvest-v1.md). |
| `github.activity.progress.json` | Completed state, checkpoint state, pending windows, run reference, and stop reason. Pinned by [`github.activity.progress.v1`](../schemas/github-activity-harvest-v1.md). |
| `github.activity.api-ledger.json` | Search/core requests, cache counts by phase, owner filtering, rate-limit snapshots, and limit events. Pinned by [`github.activity.api-ledger.v1`](../schemas/github-activity-harvest-v1.md). |
| `github.activity.report.json` | Durable harvest report written by `shiplog github activity report` and final merge receipt pinned by [`github.activity.report.v1`](../schemas/github-activity-report-v1.md). |
| `github.activity.report.md` | Human-readable harvest report written beside the JSON report. |
| `<run_id>/intake.report.json` | Evidence/source report for the generated run. |
| `<run_id>/coverage.manifest.json` | Source coverage, warnings, and partial-coverage receipts. |

## Budget and resume rules

Set budgets low when you are proving the path:

```toml
[github_activity.budget]
max_search_requests = 50
max_core_requests = 200
max_search_per_minute = 24
on_exhausted = "checkpoint_and_stop"
```

When budget is exhausted, shiplog writes progress and API ledger receipts before
stopping. Resume with the same command:

```bash
shiplog github activity run --config shiplog-github-full.toml --profile full --resume
```

Completed windows are persisted under:

```text
out/github-full/github.activity.windows/<profile>/<window_id>/
```

Each completed window carries `ledger.events.jsonl`, `coverage.manifest.json`,
and `freshness.json`. On resume, shiplog reuses valid completed window receipts,
continues from pending windows, and carries cumulative API cost forward in
`github.activity.api-ledger.json`.

Do not delete the cache between runs. The cache is what turns scout/authored/full
from repeated API work into a staged harvest.

For historical harvests, set `cache_ttl_days` to a large positive value so old
search pages, PR details, and review pages remain reusable while you scout,
resume, and rerun. Omit it for normal review-cycle intake behavior.

## Owner filtering receipts

Owner filtering is a receipt, not repo crawling. The API ledger records:

```text
requested owners:
  EffortlessMetrics
  EffortlessSteven

kept:
  EffortlessMetrics/*
  EffortlessSteven/*

dropped:
  other owners, with counts and reason
```

If `repo_owners` is empty, the harvest is actor-wide and the receipts should say
that no owner filter was requested.

## Dense or partial windows

GitHub search can cap or return incomplete results. If a period looks partial,
split it in the config and rerun only the affected scope:

```toml
[github_activity]
actor = "EffortlessSteven"
repo_owners = ["EffortlessMetrics", "EffortlessSteven"]
since = "2024-01-01"
until = "2024-04-01"
profile = "full"
cache_dir = "./out/github-full/.cache"
```

The goal is not to hide gaps. The goal is to receipt them clearly enough that
you can split, resume, or accept the caveat.

## Current activity commands

The current implemented activity commands are:

```bash
shiplog github activity plan
shiplog github activity scout
shiplog github activity run
shiplog github activity status
shiplog github activity report
shiplog github activity merge
```

`report` writes `github.activity.report.json` and `github.activity.report.md`
from the existing plan/progress/API-ledger receipts; it does not call GitHub or
render share artifacts.

`merge` currently writes final outputs for a completed activity run into
`out/github-full/final/`: `packet.md`, `github.activity.report.json`,
`github.activity.api-ledger.json`, any available coverage/ledger receipts, and
`intake.report.json` when the completed run produced one. Multi-period
deduplication remains bounded by the completed run referenced by
`github.activity.progress.json`.

Use these receipt readers after each activity pass:

```bash
shiplog github activity status --out ./out/github-full
shiplog github activity report --out ./out/github-full
shiplog github activity merge --out ./out/github-full
shiplog status --out ./out/github-full --latest
shiplog runs list --out ./out/github-full
shiplog open intake-report --out ./out/github-full --latest
shiplog open packet --out ./out/github-full --latest
```

## Safety boundaries

GitHub activity harvest should not:

- crawl every repository in an organization by default;
- mutate provider records;
- store token values in receipts;
- query GitHub from `doctor` or `status`;
- render manager/public share artifacts;
- generate performance-review prose;
- execute release work.

It should answer:

```text
What did I plan to query?
What did I spend?
What did the cache save?
What owners were kept or dropped?
What can resume safely?
Which receipts prove that?
```
