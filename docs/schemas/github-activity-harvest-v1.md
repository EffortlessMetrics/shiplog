# GitHub Activity Harvest Receipts v1

GitHub activity harvest writes three pre-merge receipts:

```text
github.activity.plan.json
github.activity.progress.json
github.activity.api-ledger.json
```

Their schemas live at:

```text
contracts/schemas/github-activity-plan.v1.schema.json
contracts/schemas/github-activity-progress.v1.schema.json
contracts/schemas/github-activity-api-ledger.v1.schema.json
```

Examples live under:

```text
examples/github-activity-plan/full.json
examples/github-activity-progress/completed.json
examples/github-activity-progress/checkpointed.json
examples/github-activity-api-ledger/completed.json
```

These receipts are the machine contract for:

```bash
shiplog github activity plan
shiplog github activity scout
shiplog github activity run --profile authored --resume
shiplog github activity run --profile full --resume
shiplog github activity status
shiplog github activity report
shiplog github activity merge
```

They are activity-harvest receipts, not packet prose, not review-loop status,
and not release approval.

## Compatibility

Each receipt carries a `schema_version` string:

```text
github.activity.plan.v1
github.activity.progress.v1
github.activity.api-ledger.v1
```

Future compatible changes should be additive and must update schemas, examples,
docs, and tests together. Removing required fields, renaming stable keys, or
changing state/profile meanings requires a new schema version or an ADR.

## Common Fields

All three receipts include:

```text
schema_version
generated_at
shiplog_version
activity_id
actor
repo_owners
profile
receipt_refs
```

`activity_id` ties plan, progress, and API ledger receipts together for one
harvest scope. `actor` is the GitHub login queried through actor-first search.
`repo_owners` is the requested owner inclusion scope; an empty list means
actor-wide harvest.

Profiles are:

```text
scout
authored
full
```

## Plan Receipt

`github.activity.plan.v1` is written by:

```bash
shiplog github activity plan
```

Required fields:

```text
schema_version
generated_at
shiplog_version
activity_id
actor
repo_owners
owner_filter_requested
query_strategy
profile
since
until
planning_mode
windows
estimated_totals
budget_policy
next_actions
receipt_refs
```

The current `query_strategy` is:

```text
actor_search_owner_filter
```

`planning_mode` is `static` today. The schema also reserves `probe` for a later
planning mode that may spend search metadata requests and receipt them.

Each window has:

```text
window_id
since
until
granularity
query_kinds
queries
```

Supported query kinds:

```text
authored_prs
reviewed_prs
```

Each query records:

```text
query_kind
search_query
estimated_search_requests
estimated_core_requests
estimated_review_requests
dense_window_risk
cache_reuse
```

`dense_window_risk` is one of:

```text
ok
dense
incomplete
unknown
blocked
```

`cache_reuse` is one of:

```text
unknown
none
partial
warm
fresh
```

The plan must not fetch PR details, fetch review pages, render packets, mutate
provider records, or execute release work.

## Progress Receipt

`github.activity.progress.v1` is written by scout/run phases and updated before
a clean checkpoint stop.

Required fields:

```text
schema_version
generated_at
shiplog_version
activity_id
plan_ref
actor
repo_owners
profile
state
completed_windows
pending_windows
active_window
stop_reason
budget_checkpoint
run_ref
receipt_refs
```

State values:

```text
planned
scouting
running
checkpointed
completed
blocked
failed
```

`active_window`, `stop_reason`, `budget_checkpoint`, and `run_ref` may be
`null` because the Rust serializer writes `Option` fields explicitly.

Stop reasons:

```text
budget_exhausted
rate_limited
secondary_rate_limited
manual_stop
config_error
provider_error
malformed_receipt
unknown
```

Completed progress should carry `run_ref`. Checkpointed progress should carry a
`stop_reason` and may carry a `budget_checkpoint` with:

```text
search_requests
core_requests
```

The current budget policy action is `checkpoint_and_stop`.

Completed window receipts live outside the progress JSON at:

```text
github.activity.windows/<profile>/<window_id>/
```

Each completed window directory contains existing shiplog artifact shapes:

```text
ledger.events.jsonl
coverage.manifest.json
freshness.json
```

`--resume` may trust a completed window only when those receipts are readable.
Missing or malformed window receipts leave that window pending so it can be
fetched again.

## API Ledger Receipt

`github.activity.api-ledger.v1` records what the harvest spent and reused.

Required fields:

```text
schema_version
generated_at
shiplog_version
activity_id
plan_ref
progress_ref
actor
repo_owners
profile
stop_reason
github_api
owner_filter
receipt_refs
```

`github_api.requests` separates:

```text
search
core
```

`github_api.cache` separates:

```text
search_probe
search_page
pull_detail
review_page
```

Each cache phase carries:

```text
fresh_hits
stale_hits
misses
```

`rate_limit_snapshots` records sanitized header-derived state:

```text
resource
limit
remaining
used
reset_at
observed_at
```

`secondary_limit_events` records sanitized limit events:

```text
resource
status
category
retry_after_seconds
observed_at
```

`stop_reason` may be `null` when the run completed normally.

## Owner Filter

The API ledger carries the owner-filter receipt:

```text
requested_owners
query_strategy
kept
dropped
```

The current dropped-owner reason is:

```text
owner_not_requested
```

Owner filtering is a receipt-backed inclusion scope. It must not be inferred
from token permissions, and it must not require crawling every repository under
each owner.

## Secrets

GitHub activity harvest receipts must not include token values, authorization
headers, passwords, redaction key material, or raw private provider response
bodies. They may name public concepts such as `github_api` or environment
variable names in surrounding docs, but not secret values.

The schemas include `propertyNames` hygiene for secret-value field names, and
tests keep known secret sentinels out of examples and generated JSON.

## Command Behavior

Harvest receipt commands:

- may query GitHub only in explicit scout/run phases;
- must not query GitHub from `status` or `report`;
- must not mutate provider records;
- must not delete cache during normal reruns;
- must checkpoint before stopping when budget exhaustion is detected;
- must not render manager or public share artifacts;
- must not scrape `packet.md`;
- must not call an LLM;
- must not execute release work.

After merge, [`github.activity.report.v1`](github-activity-report-v1.md)
records the final activity output bundle.
