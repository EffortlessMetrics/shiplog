# SHIPLOG-SPEC-0009: GitHub Activity Harvest

Status: proposed
Owner: product/schema
Created: 2026-05-19
Related proposal:
[`SHIPLOG-PROP-0007-github-activity-harvest`](../proposals/SHIPLOG-PROP-0007-github-activity-harvest.md)
Related source-identity spec:
[`SHIPLOG-SPEC-0003-source-identity`](SHIPLOG-SPEC-0003-source-identity.md)
Related intake report spec:
[`SHIPLOG-SPEC-0002-intake-report-v1`](SHIPLOG-SPEC-0002-intake-report-v1.md)
Related setup spec:
[`SHIPLOG-SPEC-0007-setup-readiness`](SHIPLOG-SPEC-0007-setup-readiness.md)
Related status spec:
[`SHIPLOG-SPEC-0008-review-loop-status`](SHIPLOG-SPEC-0008-review-loop-status.md)
Related ADR: not yet assigned

## Purpose

This spec defines the contract for GitHub activity harvest receipts:

```text
github.activity.plan.json
github.activity.progress.json
github.activity.api-ledger.json
```

The harvest workflow turns a full-history GitHub run into an audited batch job:

```text
plan -> scout -> run -> resume -> merge -> report API cost
```

The workflow is actor-first and owner-filtered. Shiplog should query GitHub for
the actor (`author:<actor>` and `reviewed-by:<actor>`), then receipt which
repository owners were kept or dropped. It must not crawl every repository in an
organization or user account by default.

This spec does not add behavior by itself. It defines the durable contract that
future implementation PRs must satisfy.

## Scope

This spec owns:

- GitHub activity scope vocabulary: actor, repository owners, windows,
  profiles, query kinds, and owner-filter receipts;
- plan receipts that estimate API cost and dense-window risk before expensive
  fetching;
- progress receipts that checkpoint completed and pending work for safe resume;
- API ledger receipts that separate search/core request cost and cache behavior
  by phase;
- budget policy for search and core buckets;
- rate-limit and secondary-limit recording;
- compatibility with existing `sources.github.user` configuration;
- no-secret, no-provider-mutation, no-OAuth safety boundaries;
- proof expectations for future implementation PRs.

Out of scope:

- OAuth;
- crawling every repository under an organization or user account;
- provider mutation;
- automatic repair;
- automatic share rendering;
- dashboards, GUI, TUI, scheduler, or team rollups;
- generated performance-review prose;
- public crate changes;
- release execution.

Safety boundaries:

- No provider mutation.
- No packet Markdown scraping.
- No release execution.

## Signal Boundary

GitHub activity harvest is an expensive-source workflow. It must compose with
the existing setup/status/intake loop without replacing it:

| Signal | Owner | Answers | Harvest may summarize | Must not imply |
| --- | --- | --- | --- | --- |
| Setup readiness | doctor/setup | Is GitHub configured enough to start? | Yes, as a prerequisite receipt. | GitHub activity was collected. |
| Activity plan | GitHub harvest | What will be queried and what might it cost? | Yes, via plan receipt. | Evidence was collected. |
| Activity progress | GitHub harvest | What work finished and what can resume? | Yes, via progress receipt. | The final packet is complete. |
| API cost | GitHub harvest | What search/core/cache budget was spent? | Yes, via API ledger. | GitHub coverage is complete by itself. |
| Source freshness | intake report | What evidence was collected, cached, skipped, or partial? | Yes, after harvest feeds intake/report. | Setup is globally ready. |
| Review-loop status | status | What is safe next? | Yes, by referencing harvest receipts. | Harvest commands ran implicitly. |

Boundary:

```text
doctor explains setup readiness.
github activity plan estimates harvest work.
github activity scout/run executes explicit harvest phases.
github activity progress checkpoints resumable state.
github activity api-ledger reports API/cache cost.
intake/report consumes activity outputs as evidence receipts.
status summarizes receipts and chooses safe next actions.
```

Harvest commands must not scrape `packet.md`, infer activity health from
terminal prose, mutate provider records, or render share artifacts.

## User Contract

The intended user path is:

```bash
shiplog github activity plan --config shiplog-github-full.toml
shiplog github activity scout --config shiplog-github-full.toml
shiplog github activity run --config shiplog-github-full.toml --profile authored --resume
shiplog github activity run --config shiplog-github-full.toml --profile full --resume
shiplog github activity status --config shiplog-github-full.toml
shiplog github activity report --config shiplog-github-full.toml
shiplog github activity merge --config shiplog-github-full.toml
```

The exact command spelling may change before implementation. Equivalent intake
subcommands are acceptable only if they preserve the same lifecycle and
receipts.

Minimum user-visible guarantees:

- plan runs before expensive detail/review fetching;
- scout is cheaper than authored/full;
- run supports `--resume`;
- status can say whether the harvest is planned, running, checkpointed,
  completed, blocked, or failed;
- report shows API cost and cache reuse;
- merge combines completed period outputs into one activity packet or explains
  why merge is blocked;
- all write-producing commands label what they write;
- no command hides API budget exhaustion as success.

## Configuration Contract

The new activity scope may live under `[github_activity]` or an equivalent
source-specific namespace. The required concepts are stable even if TOML
placement changes:

```toml
[github_activity]
actor = "EffortlessSteven"
repo_owners = ["EffortlessMetrics", "EffortlessSteven"]
since = "2020-01-01"
until = "2026-05-20"

include_authored_prs = true
include_reviews = true
include_comments = false
include_commits = false

profile = "scout"
cache_dir = "./out/github-full/.cache"

[github_activity.budget]
max_search_requests = 300
max_core_requests = 1000
max_search_per_minute = 24
on_exhausted = "checkpoint_and_stop"
```

Compatibility:

- `sources.github.user` remains a compatibility alias for `actor`.
- Missing `repo_owners` means actor-wide harvest; the report must say owner
  filtering was not requested.
- Existing `sources.github.mode`, `include_reviews`, `no_details`,
  `throttle_ms`, `cache_dir`, and `no_cache` behavior remains valid for normal
  intake.
- New harvest fields must not make old configs invalid unless the user opts
  into the activity workflow.

## Profile Contract

Profiles encode the safe operator path:

| Profile | Authored PR search | PR details | Review search | Review pages | Intended use |
| --- | --- | --- | --- | --- | --- |
| `scout` | Yes | No | No | No | Estimate coverage and dense-window risk cheaply. |
| `authored` | Yes | Yes | No | No | Build authored PR fidelity using warmed search/detail cache. |
| `full` | Yes | Yes | Yes | Yes | Add review activity using warmed cache and budget guards. |

Rules:

- `scout` must not fetch PR details or review pages.
- `authored` must not fetch review search or review pages.
- `full` may fetch review candidate searches and review pages.
- Each profile must write the profile name into plan, progress, and API ledger
  receipts.
- A later profile may reuse cache/progress from an earlier profile, but it must
  not pretend earlier missing detail/review work already completed.

## Common Receipt Fields

Each harvest receipt must include:

```json
{
  "schema_version": "github.activity.<kind>.v1",
  "generated_at": "2026-05-19T00:00:00Z",
  "shiplog_version": "0.9.0",
  "activity_id": "github_activity_20260519_000000",
  "actor": "EffortlessSteven",
  "repo_owners": ["EffortlessMetrics", "EffortlessSteven"],
  "profile": "scout",
  "receipt_refs": []
}
```

Rules:

- `schema_version` is required and uses the exact strings defined below.
- `generated_at` is UTC RFC 3339.
- `activity_id` is stable across plan/progress/ledger receipts for one harvest.
- Arrays must be deterministic.
- Secret values, tokens, authorization headers, and raw private response bodies
  must not be written.
- Unknown future fields are additive. Consumers must ignore unknown fields.
- Required fields may not disappear without a new schema version.

## Plan Receipt Contract

File:

```text
github.activity.plan.json
```

Schema version:

```text
github.activity.plan.v1
```

Purpose:

```text
Show what shiplog intends to query before expensive fetching.
Estimate API cost and dense-window risk.
Define resumable windows and query kinds.
```

Minimum shape:

```json
{
  "schema_version": "github.activity.plan.v1",
  "generated_at": "2026-05-19T00:00:00Z",
  "shiplog_version": "0.9.0",
  "activity_id": "github_activity_20260519_000000",
  "actor": "EffortlessSteven",
  "repo_owners": ["EffortlessMetrics", "EffortlessSteven"],
  "query_strategy": "actor_search_owner_filter",
  "profile": "scout",
  "since": "2020-01-01",
  "until": "2026-05-20",
  "planning_mode": "probe",
  "windows": [
    {
      "window_id": "2024-q1",
      "since": "2024-01-01",
      "until": "2024-04-01",
      "granularity": "quarter",
      "query_kinds": ["authored_prs"],
      "queries": [
        {
          "query_kind": "authored_prs",
          "search_query": "is:pr author:EffortlessSteven created:2024-01-01..2024-03-31",
          "estimated_search_requests": 4,
          "estimated_core_requests": 0,
          "estimated_review_requests": 0,
          "dense_window_risk": "ok",
          "cache_reuse": "unknown"
        }
      ]
    }
  ],
  "estimated_totals": {
    "search_requests": 14,
    "core_requests": 220,
    "review_requests": 0
  },
  "budget_policy": {
    "max_search_requests": 300,
    "max_core_requests": 1000,
    "max_search_per_minute": 24,
    "on_exhausted": "checkpoint_and_stop"
  },
  "next_actions": [
    {
      "command": "shiplog github activity scout --config shiplog-github-full.toml",
      "writes": true,
      "reason": "plan is available"
    }
  ],
  "receipt_refs": []
}
```

Plan rules:

- Plan windows must be deterministic for the same config and current date.
- `until` is exclusive in shiplog's date model; generated GitHub search ranges
  may be inclusive and must record the exact `search_query` used.
- `planning_mode = "static"` must not make GitHub API calls.
- `planning_mode = "probe"` may perform search metadata probes, but must count
  them in the API ledger if a ledger is written during planning.
- Plan must not fetch PR details.
- Plan must not fetch review pages.
- Plan must not render packets.
- `dense_window_risk` is one of `ok`, `dense`, `incomplete`, `unknown`, or
  `blocked`.
- If a window is expected to exceed GitHub's 1,000-result search cap, the plan
  must either split it or mark it `dense`.

## Progress Receipt Contract

File:

```text
github.activity.progress.json
```

Schema version:

```text
github.activity.progress.v1
```

Purpose:

```text
Record what completed, what remains, and where resume should continue.
```

Minimum shape:

```json
{
  "schema_version": "github.activity.progress.v1",
  "generated_at": "2026-05-19T00:00:00Z",
  "shiplog_version": "0.9.0",
  "activity_id": "github_activity_20260519_000000",
  "plan_ref": "github.activity.plan.json",
  "actor": "EffortlessSteven",
  "repo_owners": ["EffortlessMetrics", "EffortlessSteven"],
  "profile": "authored",
  "state": "checkpointed",
  "completed_windows": ["2024-q1"],
  "pending_windows": ["2024-q2"],
  "active_window": {
    "window_id": "2024-q2",
    "query_kind": "authored_prs",
    "phase": "details",
    "resume_cursor": {
      "search_page": 3,
      "detail_url": "https://api.github.com/repos/EffortlessMetrics/shiplog/pulls/123"
    }
  },
  "stop_reason": "budget_exhausted",
  "budget_checkpoint": {
    "search_remaining": 12,
    "core_remaining": 0,
    "reset_at": "2026-05-19T01:00:00Z"
  },
  "receipt_refs": ["github.activity.plan.json", "github.activity.api-ledger.json"]
}
```

Progress rules:

- `state` is one of `planned`, `scouting`, `running`, `checkpointed`,
  `completed`, `blocked`, or `failed`.
- Progress must be written before a budget guard stops a run.
- Progress writes must be atomic enough that an interrupted write does not leave
  a valid-looking partial JSON file.
- Resume must use progress plus cache receipts to skip completed windows.
- Resume must not refetch completed detail/review pages unless the user passes
  an explicit refresh option.
- `stop_reason` is required when `state` is `checkpointed`, `blocked`, or
  `failed`.
- `stop_reason` is one of `budget_exhausted`, `rate_limited`,
  `secondary_rate_limited`, `manual_stop`, `config_error`,
  `provider_error`, `malformed_receipt`, or `unknown`.
- Progress must never contain token values.

## API Ledger Contract

File:

```text
github.activity.api-ledger.json
```

Schema version:

```text
github.activity.api-ledger.v1
```

Purpose:

```text
Report the API and cache cost of the harvest by bucket and phase.
```

Minimum shape:

```json
{
  "schema_version": "github.activity.api-ledger.v1",
  "generated_at": "2026-05-19T00:00:00Z",
  "shiplog_version": "0.9.0",
  "activity_id": "github_activity_20260519_000000",
  "plan_ref": "github.activity.plan.json",
  "progress_ref": "github.activity.progress.json",
  "actor": "EffortlessSteven",
  "repo_owners": ["EffortlessMetrics", "EffortlessSteven"],
  "profile": "full",
  "github_api": {
    "requests": {
      "search": 84,
      "core": 312
    },
    "cache": {
      "search_probe": {
        "fresh_hits": 10,
        "stale_hits": 0,
        "misses": 14
      },
      "search_page": {
        "fresh_hits": 80,
        "stale_hits": 0,
        "misses": 84
      },
      "pull_detail": {
        "fresh_hits": 220,
        "stale_hits": 0,
        "misses": 88
      },
      "review_page": {
        "fresh_hits": 140,
        "stale_hits": 0,
        "misses": 66
      }
    },
    "rate_limit_snapshots": [
      {
        "resource": "search",
        "limit": 30,
        "remaining": 19,
        "reset_at": "2026-05-19T00:01:00Z",
        "observed_at": "2026-05-19T00:00:30Z"
      },
      {
        "resource": "core",
        "limit": 5000,
        "remaining": 4632,
        "reset_at": "2026-05-19T01:00:00Z",
        "observed_at": "2026-05-19T00:00:30Z"
      }
    ],
    "secondary_limit_events": []
  },
  "owner_filter": {
    "requested_owners": ["EffortlessMetrics", "EffortlessSteven"],
    "kept": {
      "EffortlessMetrics": 120,
      "EffortlessSteven": 44
    },
    "dropped": [
      {
        "owner": "other-owner",
        "count": 17,
        "reason": "owner_not_requested"
      }
    ]
  },
  "receipt_refs": ["github.activity.plan.json", "github.activity.progress.json"]
}
```

API ledger rules:

- Search and core request counts must be separated.
- Cache hit/miss/stale counts must be separated by phase.
- Search probes and search pages must not be hidden inside detail-cache counts.
- Rate-limit snapshots must use header-derived values when available:
  `x-ratelimit-limit`, `x-ratelimit-remaining`, `x-ratelimit-used`,
  `x-ratelimit-reset`, and `x-ratelimit-resource`.
- If a `retry-after` header is present on a rate-limit response, the event must
  record it.
- Primary or secondary rate-limit responses must be recorded with HTTP status
  and sanitized error category, not raw token-bearing request data.
- Ledger entries must not contain token values, authorization headers, or raw
  private response bodies.
- `secondary_limit_events` is an array even when empty.

## Budget Policy

Budget policy belongs in the plan and is summarized in progress and API ledger
receipts.

Required fields:

```json
{
  "max_search_requests": 300,
  "max_core_requests": 1000,
  "max_search_per_minute": 24,
  "on_exhausted": "checkpoint_and_stop"
}
```

Rules:

- Search and core budgets are separate.
- `max_search_per_minute` must default below GitHub's authenticated search limit
  to preserve safety margin.
- `on_exhausted = "checkpoint_and_stop"` is the default and only required
  behavior for the first implementation.
- The run must stop before knowingly exceeding configured budgets.
- The run must checkpoint before stopping when checkpointing is possible.
- The run must not retry forever.
- If a provider returns a primary or secondary rate-limit response, shiplog must
  obey `retry-after` or reset guidance instead of immediately retrying.

## Owner Filter Contract

The owner filter is a receipt, not just query syntax.

Required owner-filter fields:

```json
{
  "requested_owners": ["EffortlessMetrics", "EffortlessSteven"],
  "query_strategy": "actor_search_owner_filter",
  "kept": {
    "EffortlessMetrics": 120,
    "EffortlessSteven": 44
  },
  "dropped": [
    {
      "owner": "other-owner",
      "count": 17,
      "reason": "owner_not_requested"
    }
  ]
}
```

Rules:

- The plan must state whether owner filtering is requested.
- The API ledger or harvest report must state which owners were kept.
- The API ledger or harvest report must state which owners were dropped, unless
  no results were dropped.
- Results from overlapping owner filters must not duplicate events.
- If `repo_owners` is empty or missing, the report must say the harvest was
  actor-wide.
- Owner filtering must not be implemented by crawling every repository in each
  owner by default.

## Dense Window Contract

GitHub search returns up to 1,000 results per search query and may report
`incomplete_results`. Harvest planning and execution must preserve that fact.

Rules:

- Windows start from configured year/quarter/month ranges and may be split to
  month, week, or day granularity.
- A window with `total_count > 1000` must be split when possible.
- A window with `incomplete_results = true` must be split when possible.
- A day-level window that still exceeds the cap or remains incomplete must be
  marked partial, not silently accepted as complete.
- Plan and progress receipts must preserve the reason for splitting:
  `cap`, `incomplete`, `configured_window`, `resume`, or `manual`.

## Cache Contract

Harvest cache phases:

```text
search_probe
search_page
pull_detail
review_page
```

Rules:

- Search probes and search pages are cacheable harvest receipts.
- Pull details and review pages continue to use the existing API cache path or a
  compatible successor.
- Historical windows may use a longer TTL or immutable cache policy.
- Recent windows use normal TTL unless the user opts into refresh.
- Cache lookups must distinguish fresh hit, stale hit, and miss.
- `--resume` must prefer valid cached data for completed work.
- Normal reruns must not delete the cache.
- Explicit refresh must be opt-in and receipted.

## Merge Output Contract

The merge command is allowed to produce final packet/report outputs only after
required windows are completed or explicitly accepted as partial.

Minimum merge receipts:

```text
out/github-full/final/packet.md
out/github-full/final/intake.report.json
out/github-full/final/github.activity.report.json
out/github-full/final/github.activity.api-ledger.json
```

Rules:

- Merge must preserve source receipts and API ledger references.
- Merge must not invent activity for missing or partial windows.
- Merge must deduplicate events across overlapping windows and owner filters.
- Merge must report accepted partial windows.
- Merge must not render manager/public share packets.

## External API Constraints

Verified against GitHub's official docs on 2026-05-19:

- Authenticated personal access token REST API requests generally count against
  a 5,000 requests/hour personal rate limit, with different limits for some
  tokens, apps, and organizations.
- GitHub search endpoints have a custom authenticated rate limit of 30
  requests/minute for search endpoints except code search.
- GitHub REST search provides up to 1,000 results for each search.
- Search may set `incomplete_results = true`.
- GitHub rate-limit headers include `x-ratelimit-limit`,
  `x-ratelimit-remaining`, `x-ratelimit-used`, `x-ratelimit-reset`, and
  `x-ratelimit-resource`.
- Primary or secondary rate-limit responses can be `403` or `429`.
- If `retry-after` is present, clients should not retry until that many seconds
  have elapsed. Continuing while rate limited can worsen integration posture.

References:

- <https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api>
- <https://docs.github.com/en/rest/search/search>

## Compatibility

- Existing GitHub intake config remains valid.
- `user` remains a compatibility alias for `actor`.
- Existing cache databases remain valid for normal intake.
- New harvest receipts are additive.
- Missing harvest receipts must not make old reports invalid.
- `status --latest` may show harvest state only when harvest receipts exist.
- Future JSON schemas for these receipts must use additive compatibility unless
  a later ADR approves a breaking version.

## Acceptance Criteria

Future implementation PRs must prove:

- plan writes `github.activity.plan.json` before expensive detail/review fetches;
- scout writes search-only receipts and does not fetch details/reviews;
- authored profile fetches details without review pages;
- full profile fetches review candidate search and review pages;
- progress checkpoints before budget exhaustion stop;
- resume skips completed windows and completed detail/review pages;
- API ledger separates search/core requests and cache phases;
- owner filtering records requested, kept, and dropped owners;
- dense windows split or remain marked partial with reasons;
- no tokens or authorization headers appear in receipts;
- merge deduplicates and carries API ledger refs into final outputs;
- no provider mutation, OAuth, share rendering, or release work occurs.

## Proof Map

Required proof surfaces for the lane:

- docs command test for this spec;
- JSON schema and examples for `github.activity.plan.v1`;
- JSON schema and examples for `github.activity.progress.v1`;
- JSON schema and examples for `github.activity.api-ledger.v1`;
- mock-GitHub or recorded-fixture tests for plan/scout/run/resume;
- tests showing search probes/pages and detail/review pages reuse cache;
- tests showing budget exhaustion writes progress before stopping;
- tests showing owner filtering keeps/drops expected owners without duplicate
  events;
- guide proof for the `EffortlessSteven` / `EffortlessMetrics` /
  `EffortlessSteven` activity harvest case.

This spec does not authorize tag, crates.io publish, GitHub release creation,
release workflow dispatch, or release-install smoke for `v0.9.0`.
