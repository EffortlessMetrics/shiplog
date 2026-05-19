# GitHub activity harvest guide

Use this guide when you need a full GitHub activity log for one actor across
multiple repository owners without hand-managing API burn.

The workflow is:

```text
plan -> scout -> authored -> full -> inspect receipts
```

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
| `github.activity.plan.json` | Planned actor, owners, windows, profile, request estimates, and next action. |
| `github.activity.progress.json` | Completed state, checkpoint state, pending windows, run reference, and stop reason. |
| `github.activity.api-ledger.json` | Search/core requests, cache counts by phase, owner filtering, rate-limit snapshots, and limit events. |
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

Do not delete the cache between runs. The cache is what turns scout/authored/full
from repeated API work into a staged harvest.

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

## What is not landed yet

The current implemented activity commands are:

```bash
shiplog github activity plan
shiplog github activity scout
shiplog github activity run
```

These are planned follow-up surfaces, not current commands:

```bash
shiplog github activity status
shiplog github activity report
shiplog github activity merge
```

Until those land, use the existing receipt readers:

```bash
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
