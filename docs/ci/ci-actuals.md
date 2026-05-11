# `ci-actuals.json` Contract

The CI actuals workflow (added in PR #148) writes a machine-readable
record of what each lane actually cost — wall-clock, runner, cache, and
conclusion — so the LEM forecast in
[`ci-plan-json.md`](ci-plan-json.md) can be calibrated against
observed reality.

The JSON Schema lives at
[`contracts/schemas/ci-actuals.v1.schema.json`](../../contracts/schemas/ci-actuals.v1.schema.json).
This doc is the human reference.

Coverage of every declared lane is machine-checked by
`cargo xtask check-actuals-coverage` (run in the `Policy gates` CI job):
every `workflow_name` declared in `policy/ci-lanes.toml` must either
appear in `ci-actuals.yml`'s `on.workflow_run.workflows` list or be
explicitly listed in `[actuals_exemptions].not_subscribed` in
`policy/ci-lanes.toml`. The check also fails on orphan subscriptions
(workflow subscribed but no lane declares it as `workflow_name` —
every job would categorise as `lane.unknown`).

## Why actuals matter

Without actuals, LEM is a forecast: each lane has a `base_lem` in
`policy/ci-lanes.toml`, the PR plan sums them, and that's the projected
cost. But the projection is only as good as the base_lem estimates, which
are educated guesses pre-rollout.

Actuals close the loop:

- **Calibrate forecasts.** If `lane.bdd` consistently runs ~12 minutes
  but `base_lem = 32`, the estimate is wrong. The actuals receipt
  surfaces the drift.
- **Spot regressions.** A lane that suddenly takes 2× longer is a
  signal — toolchain change, dep upgrade, runner change, cache miss.
- **Hard-enforce budgets.** Hard enforcement of LEM budgets is a
  follow-up release decision after we have ≥1 month of actuals to
  confirm the model.

## Where it lives

- Output path: `target/ci/ci-actuals.json` (per workflow run, uploaded
  as artifact)
- Aggregate: a long-running collector (later; not in PR #148) merges
  per-run files into a rolling time series

## Top-level shape

```json
{
  "schema_version": 1,
  "repo": "shiplog",
  "run_id": "25616652982",
  "workflow": "CI",
  "head_sha": "<head-commit-sha>",
  "pr_number": 142,
  "lanes": [
    {
      "lane_id": "lane.ci_check",
      "workflow": "CI",
      "job": "Check (ubuntu-latest)",
      "runner": "ubuntu-latest",
      "runner_multiplier": 1.0,
      "estimated_lem": 12,
      "started_at": "2026-05-10T01:32:37Z",
      "completed_at": "2026-05-10T01:37:52Z",
      "actual_seconds": 315,
      "actual_lem": 5.25,
      "cache_hit": true,
      "cache_key": "check-ubuntu-latest",
      "conclusion": "success",
      "skip_reason": null,
      "skip_detail": null
    },
    {
      "lane_id": "lane.coverage",
      "workflow": "Coverage",
      "job": "Codecov Coverage",
      "runner": "ubuntu-latest",
      "runner_multiplier": 1.0,
      "estimated_lem": 45,
      "started_at": null,
      "completed_at": null,
      "actual_seconds": 0,
      "actual_lem": 0,
      "cache_hit": null,
      "cache_key": null,
      "conclusion": "skipped",
      "skip_reason": "label-absent",
      "skip_detail": "PR did not have 'coverage' or 'full-ci' label"
    }
  ],
  "totals": {
    "estimated_lem": 78,
    "actual_lem": 47.3,
    "drift_pct": -39.4
  }
}
```

## Field reference

### Top-level

| Field | Type | Required | Notes |
|---|---|---|---|
| `schema_version` | integer | yes | `1` for this version |
| `repo` | string | yes | Repository name |
| `run_id` | string | yes | GitHub Actions run ID |
| `workflow` | string | yes | Top-level workflow name (the one whose run produced the artifact) |
| `head_sha` | string | yes | Head commit SHA |
| `pr_number` | integer or null | yes | PR number, or null on push/dispatch |
| `lanes` | array of object | yes | Per-lane records |
| `totals` | object | yes | Aggregate forecast vs actual |

### `lanes[]`

| Field | Type | Required | Notes |
|---|---|---|---|
| `lane_id` | string | yes | Matches `lane.X` in `policy/ci-lanes.toml` |
| `workflow` | string | yes | Workflow name (e.g. `"CI"`, `"BDD Testing"`) |
| `job` | string | yes | Job name within the workflow |
| `runner` | string | yes | Runner label (e.g. `"ubuntu-latest"`, `"windows-latest"`) |
| `runner_multiplier` | number | yes | Mirror of `policy/ci-budget.toml` value |
| `estimated_lem` | number | yes | Forecast from `policy/ci-lanes.toml` `base_lem` |
| `started_at` | string or null | yes | ISO 8601 timestamp; null if skipped |
| `completed_at` | string or null | yes | ISO 8601 timestamp; null if skipped |
| `actual_seconds` | number | yes | `(completed_at - started_at)` in seconds; 0 if skipped |
| `actual_lem` | number | yes | `actual_seconds / 60 × runner_multiplier`; 0 if skipped |
| `cache_hit` | boolean or null | yes | true / false / null (no cache used or skipped) |
| `cache_key` | string or null | yes | The `Swatinem/rust-cache` `shared-key` or `actions/cache` key |
| `conclusion` | string | yes | `"success"`, `"failure"`, `"skipped"`, `"cancelled"`, `"timed_out"` |
| `skip_reason` | string or null | yes | One of [`skipped-by-policy.md`](skipped-by-policy.md) categories; null unless `conclusion == "skipped"` |
| `skip_detail` | string or null | yes | Free-form explanation; null unless skipped |

### `totals`

| Field | Type | Required | Notes |
|---|---|---|---|
| `estimated_lem` | number | yes | Sum of `lanes[].estimated_lem` for non-skipped lanes |
| `actual_lem` | number | yes | Sum of `lanes[].actual_lem` |
| `drift_pct` | number | yes | `(actual_lem - estimated_lem) / estimated_lem × 100`; positive means over-budget |

## How `actuals` is collected

Per workflow run, an aggregator job runs after the workflow's other
jobs and:

1. Queries the GitHub Actions API for per-job timings on this run.
2. Joins each job to its `lane_id` via the `policy/ci-lanes.toml`
   `workflow` field.
3. Reads the cache hit/miss from each job's step output (the
   `Swatinem/rust-cache` action exposes this).
4. Reads `conclusion` and `skip_reason` from the job's step summary.
5. Writes `target/ci/ci-actuals.json` and uploads as an artifact.

Implementation detail of PR #148. The schema is stable; the collector
mechanism may evolve.

## What "drift" means

`drift_pct` is the gap between forecast and actuals. Interpretation:

- **−50% to +20%**: forecast is roughly right; no action.
- **+20% to +50%**: forecast underestimates. Update `base_lem` in
  `policy/ci-lanes.toml`. May indicate a new dep slowing the build.
- **> +50%**: significant overrun. Investigate the lane (cache miss?
  toolchain regression? new test?).
- **< −50%**: forecast overestimates. Update `base_lem`. Often happens
  when a lane gets faster (e.g. cache improvements, test selection).

A persistent drift pattern across PRs is actionable; a single-PR drift
is usually cache-warmth or runner variance.

## Stability

`schema_version = 1` will not change incompatibly. Adding optional fields
is allowed without bumping the version. The two most likely future
additions:

- `lanes[].external_cost_usd` — mirror of `policy/ci-budget.toml`
  `external_cost_notes`, computed for bot lanes
- `lanes[].steps[]` — per-step timings for fine-grained drift analysis

These would be added in v1; consumers must tolerate unknown fields.

## See also

- [`contracts/schemas/ci-actuals.v1.schema.json`](../../contracts/schemas/ci-actuals.v1.schema.json) — JSON Schema
- [`ci-plan-json.md`](ci-plan-json.md) — the forecast counterpart
- [`policy/ci-lanes.toml`](../../policy/ci-lanes.toml) — `base_lem` source
- [`policy/ci-budget.toml`](../../policy/ci-budget.toml) — `runner_multipliers` source
- [`cache-policy.md`](cache-policy.md) — what `cache_hit` records
- [`skipped-by-policy.md`](skipped-by-policy.md) — `skip_reason` categories
