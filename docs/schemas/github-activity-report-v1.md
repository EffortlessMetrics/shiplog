# GitHub Activity Report v1

`github.activity.report.v1` is the JSON contract written by:

```bash
shiplog github activity report --out ./out/github-full
shiplog github activity merge --out ./out/github-full
```

The schema lives at:

```text
contracts/schemas/github-activity-report.v1.schema.json
```

Examples live under:

```text
examples/github-activity-report/completed.json
```

The report is the durable activity-harvest receipt for a planned, checkpointed,
or completed run. It summarizes the activity scope, final outputs when merge has
copied them, GitHub API/cache ledger, owner filtering, and receipt refs. It is
not review-loop status JSON, not packet prose, and not a release approval.

`shiplog github activity status` reads the same plan/progress/API-ledger
receipts without writing. `shiplog github activity report` writes
`github.activity.report.json` and `github.activity.report.md` at the activity
output root. `shiplog github activity merge` writes a final
`github.activity.report.json` after merge writes `out/github-full/final/`.
The pre-merge receipt contracts are documented in
[`GitHub Activity Harvest Receipts v1`](github-activity-harvest-v1.md).

## Compatibility

The v1 contract is identified by:

```json
{
  "schema_version": "github.activity.report.v1"
}
```

The following top-level fields are required:

```text
schema_version
generated_at
shiplog_version
activity_id
actor
repo_owners
query_strategy
profile
state
run_ref
source_run_dir
final_dir
final_outputs
github_api
owner_filter
receipt_refs
```

Future compatible changes should be additive and must update the schema,
examples, docs, and tests together. Removing required fields, renaming stable
keys, or changing status meanings requires a new schema version or an ADR.

## Scope

The report records:

```text
activity_id
actor
repo_owners
query_strategy
profile
state
run_ref
source_run_dir
final_dir
```

`actor` is the GitHub login used for actor-first search. `repo_owners` is the
receipt-backed inclusion scope. `query_strategy` should be
`actor_search_owner_filter` for the current harvest design.

`profile` is one of:

```text
scout
authored
full
```

`state` is one of:

```text
planned
scouting
running
checkpointed
completed
blocked
failed
```

Report can describe checkpointed or completed progress. Merge requires a
completed progress receipt today, so normal generated merge reports use
`state = "completed"`.

## Final Outputs

`final_outputs` lists artifacts actually copied into the final directory before
the report was written. Each item has:

```text
label
path
```

Known labels are:

```text
packet
intake_report
coverage
ledger
api_ledger
activity_report
artifact
```

`final_outputs` is empty for the root report before merge. `intake_report` is
present only when the completed activity run produced `intake.report.json` and
merge copied it. Merge must not invent missing intake reports or share
artifacts.

## GitHub API Ledger

`github_api` mirrors the API ledger object from
`github.activity.api-ledger.json`:

```text
requests
cache
rate_limit_snapshots
secondary_limit_events
```

`requests` separates:

```text
search
core
```

`cache` separates:

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

`rate_limit_snapshots` contains sanitized header-derived rate-limit state:

```text
resource
limit
remaining
used
reset_at
observed_at
```

`secondary_limit_events` contains sanitized limit events:

```text
resource
status
category
retry_after_seconds
observed_at
```

These fields are cost and safety receipts. They are not coverage claims by
themselves.

## Owner Filter

`owner_filter` records the actor-first, owner-filtered inclusion receipt:

```text
requested_owners
query_strategy
kept
dropped
```

`kept` is a map from repository owner to kept item count. `dropped` records
owner, count, and reason for activity excluded from the requested owner scope.
If no owner scope was requested, `requested_owners` is empty and the report is
actor-wide.

The current dropped-owner reason is `owner_not_requested`.

## Receipt References

`receipt_refs` is an ordered string list naming durable local receipts such as:

```text
github.activity.plan.json
github.activity.progress.json
github.activity.api-ledger.json
run_fixture/intake.report.json
run_fixture/coverage.manifest.json
```

Receipt refs may name expected completed-run receipts even when `final_outputs`
does not copy every optional artifact. Consumers should use `final_outputs` for
the files actually present in `final_dir`.

## Secrets

GitHub activity report JSON must not include token values, authorization
headers, passwords, redaction key material, or raw private provider response
bodies. It may include public field names such as `github_api` and environment
variable names in surrounding docs, but not secret values.

The schema includes `propertyNames` hygiene for secret-value field names, and
tests keep known secret sentinels out of examples and generated JSON.

## Command Behavior

`shiplog github activity report`:

- reads `github.activity.plan.json`, `github.activity.progress.json`, and
  `github.activity.api-ledger.json`;
- writes `github.activity.report.json` and `github.activity.report.md` when the
  required receipts are present;
- does not call GitHub;
- does not mutate provider records;
- does not render manager or public share artifacts;
- does not scrape `packet.md`;
- does not call an LLM;
- does not execute release work.

`shiplog github activity merge`:

- reads `github.activity.plan.json`, `github.activity.progress.json`, and
  `github.activity.api-ledger.json`;
- requires completed progress with a `run_ref`;
- writes final activity outputs under `out/github-full/final/`;
- writes `github.activity.report.json`;
- does not call GitHub;
- does not mutate provider records;
- does not render manager or public share artifacts;
- does not scrape `packet.md`;
- does not call an LLM;
- does not execute release work.
