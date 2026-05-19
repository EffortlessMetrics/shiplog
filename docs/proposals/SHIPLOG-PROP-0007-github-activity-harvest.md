# SHIPLOG-PROP-0007: GitHub Activity Harvest

Status: proposed
Owner: product/docs
Created: 2026-05-19
Target release: after the held 0.9.0 review-loop cockpit candidate
Follow-up spec:
[`SHIPLOG-SPEC-0009-github-activity-harvest`](../specs/SHIPLOG-SPEC-0009-github-activity-harvest.md)
Architecture decision: not yet assigned

## Summary

Shiplog should turn full GitHub history collection into a first-class activity
harvest workflow:

```text
plan -> scout -> run -> resume -> merge -> report API cost
```

Today the GitHub source already has the right low-level pieces: it queries by
GitHub actor login, uses `author:<user>` for authored PRs, uses
`reviewed-by:<user>` for review candidates, adaptively slices dense date
windows, and caches detail/review fetches. That is enough for ordinary intake.
It is still awkward for a full-history run across private repositories, multiple
repository owners, and several years of activity because the operator must
manually decide the harvest shape.

The proposed lane makes the expensive GitHub path explicit and receipt-backed:

```text
actor: EffortlessSteven
repo owners: EffortlessMetrics, EffortlessSteven
windows: year/quarter/day as needed
profile: scout, authored, full
budget: search/core request limits
resume: checkpointed
output: merged activity packet plus API ledger
```

This is not a request to crawl organizations. The architectural center is
actor-first search with optional owner filtering and receipts.

## Problem

A user trying to build a full GitHub activity log has to answer too many
operator questions by hand:

- Should I configure one GitHub source or one source per repository owner?
- Did I collect activity for the actor everywhere the token can see?
- How should I split six years of history without hitting search caps?
- Which phase spends API budget: search, PR details, or review pages?
- Which cache entries were reused, stale, or missed?
- If the run stops halfway through, what is safe to resume?
- How much API budget did the final packet cost?
- Which repository owners were kept or dropped from actor-wide search results?

The current answer is a careful recipe: use one `user = "EffortlessSteven"`
source, slice by years or quarters, scout with `no_details = true`, warm the
cache, enable reviews last, avoid parallel runs, and inspect coverage receipts.
That recipe works, but it belongs in shiplog as a workflow instead of living as
operator memory.

## Target Users

Primary users:

- a self-reviewer who wants a full GitHub authored-PR and review history across
  personal and organization-owned repositories;
- a deadline-pressure user who needs a cheap scout before spending detail/review
  API budget;
- a user with private repository access who needs receipts for what was kept,
  dropped, cached, partial, or interrupted;
- an agent using `doctor --setup --json` and `status --latest --json` that needs
  a bounded way to plan, run, checkpoint, and resume an expensive source.

Secondary users:

- maintainers extending the expensive-source pattern to GitLab, Jira, Linear, or
  future team harvests;
- release reviewers checking that GitHub activity collection does not hide API
  cost or overstate source coverage.

## Product End State

The user should be able to declare the activity scope once:

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

api_budget = 800
search_budget_per_minute = 24
profile = "scout"
cache_dir = "./out/github-full/.cache"
```

Then run a lifecycle:

```bash
shiplog github activity plan --config shiplog-github-full.toml
shiplog github activity scout --config shiplog-github-full.toml
shiplog github activity run --config shiplog-github-full.toml --resume
shiplog github activity status --config shiplog-github-full.toml
shiplog github activity report --config shiplog-github-full.toml
shiplog github activity merge --config shiplog-github-full.toml
```

The exact command spelling can change in the spec. The required product shape is
the lifecycle:

```text
plan before spending budget
scout before full fidelity
checkpoint before stopping
resume without refetching completed work
merge period outputs into one activity packet
report API cost and cache reuse
```

## Current Capabilities To Preserve

The GitHub source already does useful work that should remain the foundation:

- actor-oriented authored PR search with `author:<user>`;
- best-effort review search with `reviewed-by:<user>`, followed by review-page
  filtering by reviewer and submitted date;
- `mode = "created"` and merged-mode authored PR queries;
- `include_reviews`, `no_details`, `throttle_ms`, `cache_dir`, and `no_cache`
  knobs;
- month-to-week-to-day adaptive slicing when search counts exceed GitHub search
  caps or report incomplete results;
- SQLite API cache entries for PR details and PR review pages;
- source freshness receipts for cache hits, misses, and stale hits;
- coverage slices in the intake report when source coverage is partial.

The harvest lane should compose these pieces before replacing them.

## Proposed Concepts

### Actor And Repository Owners

Add an explicit model:

```toml
[sources.github]
enabled = true
actor = "EffortlessSteven"
repo_owners = ["EffortlessMetrics", "EffortlessSteven"]
mode = "created"
include_reviews = true
```

Keep `user = "EffortlessSteven"` as a compatibility alias for `actor`, but make
reports use the clearer language:

```text
Actor:
  EffortlessSteven

Repository owners included:
  EffortlessMetrics
  EffortlessSteven

Query strategy:
  actor search with owner filtering
```

If shiplog queries actor-wide and filters locally, the report should say:

```text
Queried:
  author:EffortlessSteven

Kept:
  EffortlessMetrics/*
  EffortlessSteven/*

Dropped:
  other owners: 17 PRs
```

That receipt is better than hiding owner scope in query syntax.

### Plan-Only Mode

Before fetching details or review pages, shiplog should build a deterministic
plan:

```json
{
  "actor": "EffortlessSteven",
  "repo_owners": ["EffortlessMetrics", "EffortlessSteven"],
  "windows": [
    {
      "since": "2024-01-01",
      "until": "2024-04-01",
      "query_kinds": ["authored_prs", "reviews"],
      "estimated_search_requests": 14,
      "estimated_detail_requests": 220,
      "risk": "ok"
    }
  ],
  "estimated_total_requests": 780,
  "rate_limit_policy": {
    "max_search_per_minute": 24,
    "pause_on_secondary_limit": true
  }
}
```

The plan is a receipt, not a guarantee. It should estimate API burn, identify
dense windows, and show where scout/full profiles will spend budget.

### Scout, Authored, And Full Profiles

Profiles should encode the safe operator path:

```text
scout:
  search only
  no details
  no reviews
  coverage estimate

authored:
  authored PR details
  no reviews
  uses warmed search/detail cache

full:
  authored PR details
  review search and review pages
  uses warmed cache
```

The printed next commands should route:

```text
plan -> scout -> authored -> full -> merge -> report
```

### Search Cache And Historical TTL

The existing cache helps most with PR details and review pages. Full-history
harvest needs a cache boundary for search work too:

```text
search metadata probes
search result pages
rate-limit snapshots
activity plan windows
owner-filtered result sets
```

Historical windows should support a longer TTL than recent windows:

```text
window ended more than 90 days ago:
  long TTL or permanent until --refresh

recent window:
  normal TTL
```

### Resume Checkpoints

A full harvest should write durable progress:

```text
out/github-full/github.activity.plan.json
out/github-full/github.activity.progress.json
out/github-full/github.activity.api-ledger.json
```

`--resume` should skip completed windows and completed detail/review pages. If a
budget guard stops a run, shiplog should checkpoint and stop cleanly instead of
retrying forever or failing after spending half the budget.

### API Budget Ledger

Every harvest run should emit an API ledger:

```json
{
  "github_api": {
    "search_requests": 84,
    "core_requests": 312,
    "cache_hits": 440,
    "cache_misses": 312,
    "stale_hits": 0,
    "rate_limit_remaining": {
      "search": 19,
      "core": 4632
    },
    "secondary_limit_events": []
  }
}
```

The human report should include the same shape:

```markdown
## API Budget

GitHub:
- Search requests: 84
- Detail/review requests: 312
- Cache hits: 440
- Cache misses: 312
- Search remaining: 19
- Core remaining: 4632
- Secondary limit events: none
```

No token values should ever appear in this ledger.

## External Constraints

Verified against GitHub's official docs on 2026-05-19:

- Authenticated REST API requests generally count against a 5,000 requests/hour
  personal rate limit, with different limits for some token types and
  organizations.
- Search endpoints have a separate custom rate limit: authenticated search
  requests allow up to 30 requests/minute for search endpoints other than code
  search.
- GitHub REST search returns up to 1,000 results for each search query.
- Search responses may set `incomplete_results`, so shiplog must keep
  date-slicing and partial-coverage receipts.
- GitHub recommends using response headers such as `x-ratelimit-remaining` and
  `x-ratelimit-reset` and respecting `retry-after` when rate limited.
- Primary or secondary rate-limit responses can be `403` or `429`; continuing
  to send requests while rate limited can worsen the integration posture.

References:

- <https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api>
- <https://docs.github.com/en/rest/search/search>

## Non-Goals

This lane should not add:

- OAuth;
- repository crawling across every repository in an organization or account;
- live provider probing from doctor or status;
- provider mutation;
- automatic repair;
- automatic share rendering;
- dashboards, TUI, GUI, or scheduler work;
- generated performance-review prose;
- public crate surface churn;
- release execution.

It also should not make a full GitHub harvest the default `intake` behavior.
Full-history activity harvest is an explicit expensive workflow.

## Alternatives Considered

### Keep the operator recipe in docs

Rejected. The manual recipe works, but it makes the user understand GitHub
search caps, rate limits, cache behavior, period slicing, and review-page cost
before shiplog can produce a trustworthy full history.

### Crawl repository owners directly

Rejected as the default architecture. Crawling every repository under
`EffortlessMetrics` and `EffortlessSteven` spends API on repository discovery
and can miss actor activity in repositories outside those owners. Actor search
is the better primary query; owner filtering should be a receipt-backed
inclusion scope.

### Use only complex GitHub search qualifiers for owner scope

Rejected as the only strategy. Search syntax can help, but owner scope should
remain visible as a receipt. The report should say what was queried, kept, and
dropped.

### Cache only PR details and review pages

Rejected for full-history harvest. Detail/review cache is still valuable, but
the search phase is live and expensive enough that search probes, result pages,
and plan windows need their own cache and API ledger accounting.

### Retry until GitHub accepts the run

Rejected. Rate-limit and secondary-limit behavior should checkpoint and stop or
wait according to explicit budget policy. It should not spin indefinitely.

## Proposed Artifact Stack

Land the lane in small semantic PRs:

1. This proposal:
   `docs/proposals/SHIPLOG-PROP-0007-github-activity-harvest.md`.
2. Spec: GitHub activity harvest contract, including plan, progress, and API
   ledger shapes.
3. ADR: GitHub activity harvest is actor-first and owner-filtered.
4. Plan-only activity command or intake profile.
5. Search cache for metadata probes and result pages.
6. API budget guardrails and checkpointing.
7. Actor and repo-owner scope model with `user` compatibility alias.
8. Scout/authored/full harvest profiles.
9. API ledger in intake/activity reports.
10. Full-history GitHub activity harvest guide.
11. Product proof using recorded HTTP fixtures or a mock server, not live
    GitHub.

## Proof Map

Existing proof surfaces to link from future specs and plans:

- [`apps/shiplog/src/ingest/github.rs`](../../apps/shiplog/src/ingest/github.rs):
  current actor search, review search, adaptive date slicing, and detail/review
  cache behavior.
- [`docs/guides/recurring-review-loop.md`](../guides/recurring-review-loop.md):
  the status-first recurring loop this expensive-source workflow should feed.
- [`docs/proposals/SHIPLOG-PROP-0006-review-loop-status.md`](SHIPLOG-PROP-0006-review-loop-status.md):
  the status cockpit lane that should eventually summarize activity harvest
  progress and next actions.
- [`docs/schemas/review-loop-status-v1.md`](../schemas/review-loop-status-v1.md):
  the current agent-facing status contract that future activity status should
  compose with rather than replace.
- [`docs/release/0.9.0-release-hold.md`](../release/0.9.0-release-hold.md):
  the active release hold. This proposal does not approve `v0.9.0` release
  execution.

Future proof should include:

- deterministic plan JSON snapshots;
- API budget ledger schema/examples;
- resume/progress fixtures;
- mock-GitHub product tests proving scout -> run -> resume does not refetch
  completed detail/review pages;
- report assertions for kept/dropped repository owners and partial coverage.
