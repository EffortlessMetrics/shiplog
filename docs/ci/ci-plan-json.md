# `ci-plan.json` Contract

The PR plan workflow (added in PR #146) writes a machine-readable
forecast of the lanes a PR will run, the risk packs that matched, and
the projected LEM cost. This document specifies the v1 contract.

The JSON Schema lives at
[`contracts/schemas/ci-plan.v1.schema.json`](../../contracts/schemas/ci-plan.v1.schema.json).
This doc is the human reference.

## Where it lives

- Output path: `target/ci/ci-plan.json` (in the workflow workspace)
- Mirror: GitHub Actions step summary on the `pr-plan` job
- Eventual consumer: PR comment / branch-protection summary check
  (later; not in PR #146)

## Top-level shape

```json
{
  "schema_version": 1,
  "repo": "shiplog",
  "base_sha": "<base-commit-sha>",
  "head_sha": "<head-commit-sha>",
  "pr_number": 142,
  "labels": ["full-ci"],
  "changed": {
    "files": ["docs/ci/ci-plan-json.md", "..."],
    "areas": ["docs"],
    "crates": []
  },
  "selection": {
    "risk_packs": [
      {"id": "docs-only", "matched_paths": ["docs/ci/ci-plan-json.md"]}
    ],
    "lanes": [
      {"id": "lane.pr_plan", "selected_by": "default_pr"},
      {"id": "lane.ci_check", "selected_by": "default_pr"},
      {"id": "lane.ci_check_windows", "selected_by": "default_pr"},
      {"id": "lane.ci_deny", "selected_by": "default_pr"},
      {"id": "lane.ci_msrv", "selected_by": "default_pr"},
      {"id": "lane.droid_review", "selected_by": "default_pr"}
    ],
    "skipped_lanes": [
      {"id": "lane.coverage", "skip_reason": "label-absent"},
      {"id": "lane.bdd", "skip_reason": "no-matching-risk-pack"},
      {"id": "lane.property", "skip_reason": "no-matching-risk-pack"}
    ]
  },
  "budget": {
    "estimated_lem": 78,
    "band": "elevated",
    "default_limit_lem": 35,
    "elevated_limit_lem": 75,
    "hard_limit_lem": 125,
    "ack_required": "ci-budget-override",
    "ack_present": true
  },
  "warnings": [
    "lane.ci_check_windows estimated 36 LEM; covered by ci-exception-0001"
  ]
}
```

## Field reference

### Top-level

| Field | Type | Required | Notes |
|---|---|---|---|
| `schema_version` | integer | yes | `1` for this version |
| `repo` | string | yes | Repository name (e.g. `"shiplog"`) |
| `base_sha` | string | yes | Base commit SHA (PR target branch HEAD) |
| `head_sha` | string | yes | Head commit SHA (PR source HEAD) |
| `pr_number` | integer | no | PR number; absent on dispatch / push to non-PR ref |
| `labels` | array of string | yes | Labels currently applied to the PR |
| `changed` | object | yes | What the PR touched |
| `selection` | object | yes | Lane selection result |
| `budget` | object | yes | LEM forecast |
| `warnings` | array of string | yes | Non-blocking diagnostics from the planner |

### `changed`

| Field | Type | Required | Notes |
|---|---|---|---|
| `files` | array of string | yes | Repo-relative paths from the PR diff |
| `areas` | array of string | yes | Top-level areas affected (e.g. `"docs"`, `"crates"`, `"apps"`, `"scripts"`, `"policy"`, `"workflows"`) |
| `crates` | array of string | yes | Workspace crates touched (e.g. `"shiplog-redact"`) |

### `selection`

| Field | Type | Required | Notes |
|---|---|---|---|
| `risk_packs` | array of object | yes | Each: `{id: string, matched_paths: array of string}` |
| `lanes` | array of object | yes | Selected lanes (will run); each: `{id: string, selected_by: string}` |
| `skipped_lanes` | array of object | yes | Lanes deliberately skipped; each: `{id: string, skip_reason: string}` |

`selected_by` values:

- `"default_pr"` — the lane has `default_pr = true` in `policy/ci-lanes.toml`
- `"label:<name>"` — the lane was selected because the label is present (e.g. `"label:full-ci"`)
- `"risk_pack:<id>"` — the lane was selected by a risk pack (e.g. `"risk_pack:redaction-privacy"`)
- `"branch:<name>"` — the lane was selected because of branch (e.g. `"branch:main"`)

`skip_reason` values must match the categories in
[`skipped-by-policy.md`](skipped-by-policy.md):

- `"docs-only"`, `"no-matching-risk-pack"`, `"label-absent"`,
  `"nightly-only"`, `"release-only"`, `"ripr-waived"`, `"duplicate"`

### `budget`

| Field | Type | Required | Notes |
|---|---|---|---|
| `estimated_lem` | number | yes | Sum of `base_lem` × `runner_multiplier` for selected lanes |
| `band` | string | yes | `"preferred"`, `"default"`, `"elevated"`, or `"hard"` |
| `default_limit_lem` | integer | yes | Mirror of `policy/ci-budget.toml` value |
| `elevated_limit_lem` | integer | yes | Mirror of `policy/ci-budget.toml` value |
| `hard_limit_lem` | integer | yes | Mirror of `policy/ci-budget.toml` value |
| `ack_required` | string or null | yes | Label needed for the band (`"ci-budget-ack"`, `"ci-budget-override"`, `null`) |
| `ack_present` | boolean | yes | Whether the required label is on the PR |

`band` derivation:

- `estimated_lem ≤ preferred_default_lem` → `"preferred"` (no ack required)
- `≤ default_limit_lem` → `"default"` (no ack required)
- `≤ elevated_limit_lem` → `"elevated"` (`ci-budget-ack` required)
- `≤ hard_limit_lem` → `"hard"` (`ci-budget-override` required)
- `> hard_limit_lem` → `"hard"` with a warning entry (PR #146 emits
  warning; hard enforcement is a follow-up release)

### `warnings`

Free-form strings. Examples:

- `"lane.X estimated N LEM; covered by ci-exception-Y"`
- `"PR exceeds default_limit_lem (35); ci-budget-ack label expected"`
- `"PR matched only docs-only risk pack; consider adding 'coverage' label if docs touch coverage configs"`

Warnings are non-blocking. Hard enforcement is post-#148 actuals.

## Stability

`schema_version = 1` will not change incompatibly. Adding optional fields
is allowed without bumping the version. Removing fields, renaming fields,
or changing types requires `schema_version = 2`.

The schema is stored at
[`contracts/schemas/ci-plan.v1.schema.json`](../../contracts/schemas/ci-plan.v1.schema.json).
Tooling that emits or consumes `ci-plan.json` should validate against
that schema.

## Producers and consumers

| Producer | Where | Notes |
|---|---|---|
| `pr-plan.yml` | PR #146 | Writes `target/ci/ci-plan.json`; uses `policy/ci-lanes.toml` + `policy/ci-risk-packs.toml` + `policy/ci-budget.toml` + `policy/ci-exceptions.toml` |
| `cargo xtask ci plan` | PR #143 (xtask) | Local equivalent for testing |

| Consumer | Where | Notes |
|---|---|---|
| GitHub Actions step summary | PR #146 | Renders the plan as Markdown |
| `gh pr comment` | (later) | Posts a fresh plan as a comment when the PR diff changes |
| Required-check summary | (later) | `pr-plan / forecast` becomes a required check |

## See also

- [`contracts/schemas/ci-plan.v1.schema.json`](../../contracts/schemas/ci-plan.v1.schema.json) — JSON Schema
- [`policy/ci-budget.toml`](../../policy/ci-budget.toml), [`policy/ci-lanes.toml`](../../policy/ci-lanes.toml), [`policy/ci-risk-packs.toml`](../../policy/ci-risk-packs.toml), [`policy/ci-exceptions.toml`](../../policy/ci-exceptions.toml) — inputs
- [`labels.md`](labels.md) — `labels` field semantics
- [`risk-packs.md`](risk-packs.md) — `risk_packs` selection
- [`skipped-by-policy.md`](skipped-by-policy.md) — `skip_reason` categories
- [`ci-actuals.md`](ci-actuals.md) — counterpart schema for observed cost
