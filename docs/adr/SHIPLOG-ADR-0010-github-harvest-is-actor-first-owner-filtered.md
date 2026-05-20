# SHIPLOG-ADR-0010: GitHub Harvest Is Actor-First, Owner-Filtered

Status: accepted
Date: 2026-05-19
Related proposal:
[`SHIPLOG-PROP-0007-github-activity-harvest`](../proposals/SHIPLOG-PROP-0007-github-activity-harvest.md)
Related spec:
[`SHIPLOG-SPEC-0009-github-activity-harvest`](../specs/SHIPLOG-SPEC-0009-github-activity-harvest.md)
Related source-identity spec:
[`SHIPLOG-SPEC-0003-source-identity`](../specs/SHIPLOG-SPEC-0003-source-identity.md)
Related intake report spec:
[`SHIPLOG-SPEC-0002-intake-report-v1`](../specs/SHIPLOG-SPEC-0002-intake-report-v1.md)

## Context

Shiplog's existing GitHub source is actor-oriented. Authored pull requests are
found with queries shaped around `author:<user>`, and review candidates are
found with queries shaped around `reviewed-by:<user>`. That is the right
primitive for a personal activity log because the user's question is:

```text
What did this actor author or review?
```

The full-history harvest lane adds another dimension:

```text
actor: EffortlessSteven
repo owners: EffortlessMetrics, EffortlessSteven
```

Without an explicit decision, future implementation could drift toward one of
two expensive or misleading shapes:

- configure one source per repository owner and make users mentally merge the
  results;
- crawl every repository under each owner and then try to discover actor
  activity from the repository side.

Both shapes are worse than the actor-first model. Repository crawling spends API
budget on repositories where the actor may have no activity, increases
permission and pagination failure modes, and makes partial coverage harder to
explain. One-source-per-owner setups also make it unclear whether the actor was
queried everywhere the token can see or only inside one owner namespace.

The new harvest workflow needs to stay honest about both dimensions:

```text
query the actor;
filter or receipt requested owners;
report kept owners;
report dropped owners;
never pretend owner filtering proves global GitHub coverage.
```

[`SHIPLOG-SPEC-0009`](../specs/SHIPLOG-SPEC-0009-github-activity-harvest.md)
defines the plan/progress/API-ledger receipts. This ADR records the durable
architecture choice those receipts must preserve.

## Decision

GitHub activity harvest is actor-first and owner-filtered.

Shiplog should use actor search as the primary discovery path:

- authored PR harvest queries use `author:<actor>`;
- review harvest queries use `reviewed-by:<actor>`;
- existing `sources.github.user` remains a compatibility alias for `actor`;
- `repo_owners` is an optional inclusion scope, not a list of owners to crawl.

When `repo_owners` is configured, owner filtering is a receipt-backed inclusion
step. The harvest must record:

- the actor queried;
- the requested repository owners;
- the query strategy, such as `actor_search_owner_filter`;
- which repository owners were kept;
- which repository owners were dropped;
- why results were dropped, such as `owner_not_requested`;
- whether the harvest was actor-wide because `repo_owners` was missing.

The owner filter is a receipt, not merely query syntax. Implementation may use
GitHub search qualifiers where they are safe and useful, but the durable user
contract is the receipt:

```text
Queried actor: EffortlessSteven
Requested owners: EffortlessMetrics, EffortlessSteven
Kept: EffortlessMetrics/*, EffortlessSteven/*
Dropped: other owners, with counts and reasons
```

GitHub activity harvest must not crawl every repository in an organization or
user account by default.

Harvest commands must preserve the lifecycle from the proposal and spec:

```text
plan -> scout -> authored -> full -> status -> report -> merge
```

The actor/owner decision applies across:

- `github.activity.plan.json`;
- `github.activity.progress.json`;
- `github.activity.api-ledger.json`;
- `github.activity.report.json`;
- final merged activity reports and packets;
- `shiplog github activity status`, `report`, and `merge` surfaces;
- future `status --latest` summaries when harvest receipts exist.

## Consequences

- Users configure one activity actor instead of one GitHub source per repository
  owner.
- Full-history harvests can cover both `EffortlessMetrics/*` and
  `EffortlessSteven/*` without making users enumerate repositories.
- API budget is spent on actor activity search, detail fetches, review pages,
  and explicit resume work rather than repository inventory crawling.
- Owner scope becomes auditable. Reports can say which owners were kept, which
  owners were dropped, and why.
- A token with private repository access can discover private activity through
  the actor query path without Shiplog needing to list every private repository
  first.
- Missing or omitted owner scope remains honest: Shiplog reports an actor-wide
  harvest instead of implying coverage for a specific owner set.
- Dense-window splitting stays attached to actor query windows rather than
  exploding into owner/repository fanout.
- Merge logic must deduplicate overlapping windows and owner filters, but it
  does not need to reconcile independent per-owner activity sources by default.
- Future implementation PRs must prove owner filtering through receipts and API
  ledger output, not through undocumented query construction alone.
- This decision does not authorize provider mutation, OAuth, repository crawling
  by default, share rendering, generated performance-review prose, tagging,
  publishing, GitHub release creation, workflow dispatch, or release-install
  smoke.

## Alternatives Considered

### One GitHub Source Per Repository Owner

Rejected. It makes the configuration look explicit, but it forces the user to
merge separate owner-scoped reports and obscures the real identity question:
what did this actor author or review? It also increases duplicate risk when
repositories move, forks overlap, or owner boundaries change.

### Crawl Every Repository Under Each Owner

Rejected. Repository crawling is expensive, permission-sensitive, and not the
right discovery primitive for a personal activity log. It spends requests on
repositories with no actor activity and makes rate-limit, pagination, and resume
behavior harder to explain.

### Encode Owner Scope Only In GitHub Search Syntax

Rejected as the durable contract. Search syntax may be an optimization, but
users and agents need a receipt saying what was requested, kept, and dropped.
If query syntax changes later, the receipt contract must remain stable.

### Treat Missing `repo_owners` As An Error

Rejected. Actor-wide harvest is valid. When no owner filter is configured,
Shiplog should say owner filtering was not requested and report the observed
owners, not fail the run.

### Infer Owner Coverage From Token Permissions

Rejected. Token permissions explain what GitHub may return, but they do not
prove which owners were intentionally in scope. Owner scope must come from
configuration and receipts.

### Defer Owner Filtering To The Final Packet

Rejected. Owner filtering affects API ledger, progress, merge, and coverage
claims. It must be visible throughout plan, run, resume, and report phases, not
only at the human packet layer.

## Affected Specs, Plans, Tests, And Schemas

- [`SHIPLOG-PROP-0007-github-activity-harvest`](../proposals/SHIPLOG-PROP-0007-github-activity-harvest.md)
  proposes the plan/scout/authored/full/status/report/merge lifecycle and the
  `EffortlessSteven` / `EffortlessMetrics` / `EffortlessSteven` use case.
- [`SHIPLOG-SPEC-0009-github-activity-harvest`](../specs/SHIPLOG-SPEC-0009-github-activity-harvest.md)
  defines `github.activity.plan.json`, `github.activity.progress.json`,
  `github.activity.api-ledger.json`, and `github.activity.report.json`.
- The `github.activity.plan.v1`, `github.activity.progress.v1`, and
  `github.activity.api-ledger.v1` schemas include actor and owner-filter fields
  consistent with this ADR.
- [`github.activity.report.v1`](../schemas/github-activity-report-v1.md) pins
  the final merge report receipt and carries the same owner-filter shape.
- Future plan-only tests should prove the planned query strategy is
  `actor_search_owner_filter` when owners are configured.
- Future owner-filter tests should prove kept/dropped owner receipts and no
  duplicate events when owner scopes overlap.
- Resume tests prove progress checkpoints stay actor/window based rather than
  repository-crawl based, and that valid completed window receipts are skipped
  on `--resume`.
- Future API-ledger tests should prove request counts and cache phases are
  reported without token values, authorization headers, or raw private response
  bodies.

## Implementation Note

Runtime commands and receipt schemas have since landed for the unreleased
GitHub activity harvest lane: plan, scout, run, status, report, and merge. The
implementation persists completed window receipts under
`github.activity.windows/<profile>/<window_id>/` so `--resume` can skip
completed windows without repository crawling.

The ADR document does not add runtime behavior by itself; the behavior is tied
to the implementation and proof PRs that followed it.
This implementation note does not lift the `v0.9.0` release hold.
