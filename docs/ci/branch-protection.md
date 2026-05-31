# Branch Protection

The rule for what GitHub requires before a PR can merge into `main`.

## The rule

> Require summary checks. Do not require matrix leaves. Do not require
> advisory lanes. Skipped-by-policy is acceptable only when the lane map
> says it is acceptable.

## Why each clause

**Require summary checks.** A summary check is a single pass/fail that
aggregates a related set of underlying jobs. GitHub branch protection
should require the summary, not every leaf job. Otherwise:

- Renaming a leaf job (e.g. adding a fuzz target, splitting a BDD job)
  silently makes the PR un-mergeable until branch protection is updated.
- A PR plan that legitimately skips a lane (docs-only, no matching risk
  pack) shows as "pending" forever.

**Do not require matrix leaves.** Matrix jobs produce one check per leaf.
Requiring a specific matrix leaf locks the required-check name to the
matrix shape. If the matrix changes, branch protection breaks. Require
the umbrella check instead.

**Do not require advisory lanes.** `ripr`, `droid-review`, mutation, full
fuzz, and any lane with `default_pr = false` in
[`policy/ci-lanes.toml`](../../policy/ci-lanes.toml) are advisory. A red
advisory lane is a signal, not a block. Required-check status should not
include them.

**Skipped-by-policy is acceptable only when the lane map says it is.** A
lane that legitimately skips for a particular PR (e.g. coverage on a
docs-only PR, fuzz on a docs PR, BDD on a docs PR) should report
"skipped" with a reason
(see [`skipped-by-policy.md`](skipped-by-policy.md)) rather than
"pending" or "not run." Branch protection should treat these skips as
acceptable. The lane map is the source of truth for which skips are
allowed.

## Required checks (target steady state)

These describe what should appear in branch-protection settings for
`main` once protection is enabled. **`main` is currently unprotected**
at the GitHub level (verified via the `/repos/.../branches/main/protection`
endpoint returning 404 "Branch not protected"). The table is the
**target** set, encoded by the routed default PR summary and by
`blocking = true` lanes in
[`policy/ci-lanes.toml`](../../policy/ci-lanes.toml). It describes what
would be required-enforced if protection were enabled, not what is
currently enforced.

| Required check | Source | Why required |
|---|---|---|
| `Shiplog Rust Small Result` | routed swarm gate | Default PR correctness summary |
| `pr-plan / forecast` | `pr-plan.yml` (added in PR #146) | LEM forecast + risk-pack receipt |
| `CI / Check (self-hosted)` | `ci.yml` | Main/full-ci Rust correctness gate |
| `CI / cargo-deny` | `ci.yml` | Main/full-ci dependency policy |
| `CI / Policy gates` | `ci.yml` (added in PR #165, expanded for SHIPLOG-SPEC-0010) | Main/full-ci policy and source-of-truth enforcement: schemas, doc artifacts, active goals, support tiers, and blocking-allowlist file/lint/panic checks |

The `CI / ...` checks are not default PR checks while `ci.yml` is gated to
push-to-main and `full-ci` PRs. If branch protection later requires them,
update the workflow route and lane ledger in the same PR.

The `MSRV (1.95)` job that previously appeared here was dropped in PR
#164 as redundant compile coverage with `Check (ubuntu-latest)` (both
pin `toolchain: 1.95.0`).

These should **not** be required:

- `Mutation Testing` — weekly cron + `mutation` / `full-ci` label only, advisory.
- `Coverage` — push to `main` + `coverage` / `full-ci` label only, advisory.
- `Quick Fuzz (CI)` — label-gated (`fuzz` / `full-ci`, routed in PR #155); the bounded `Fuzz Smoke` (PR #154) is required-eligible.
- `BDD & Property Tests`, `Ingest Crate Tests`, `Render, Trust, and Contract Crates`, `Optional Surface Tests`, `App Tests` — broad BDD matrix, label-gated (`bdd` / `full-ci`, routed in PR #155); the bounded `BDD Smoke` (PR #154) is required-eligible.
- `Property Testing` — broad sweep, label-gated (`property-tests` / `full-ci`, routed in PR #155); the bounded `Property Smoke` (PR #154) is required-eligible.
- `droid-review`, `droid`, `droid-security-scan` — advisory bot review.
- `ripr` (PR #153) — advisory lane, v1 stub.
- `Cargo Deny Security` (`security.yml`) — duplicate of `CI / cargo-deny` on PR; standalone workflow now label-gated (`security-audit` / `full-ci`, routed in PR #155).

## Migration history

The v0.5.0 ladder shipped every PR that would have triggered a
branch-protection setting change under the rule above. Because `main`
was never protected during the ladder, those rule-triggered setting
changes were not actually performed — they remain forward-looking
guidance.

- **PR #146** added `pr-plan / forecast` as a check. Required-eligible
  but not enforced (no protection setting was added).
- **PR #154** added bounded smoke lanes (`lane.bdd_smoke`,
  `lane.property_smoke`, `lane.fuzz_smoke`). Required-eligible but not
  enforced.
- **PR #155** routed the broad sweep lanes to nightly + label. The
  broad jobs are now `default_pr = false` in the lane policy and
  job-level `if:`-gated in the workflow YAML.
- **PR #164** dropped the redundant `MSRV (1.95)` job — closes the
  candidate-removal step that had been forward-looking in earlier
  versions of this doc.
- **PR #165** added `Policy gates` to `ci.yml` as a blocking job and
  registered `[lane.ci_policy]` with `blocking = true`.

When/if branch-protection is enabled on `main`, the sequencing rules
in [`required-check-migration.md`](required-check-migration.md)
describe how to make those required-check changes safely (avoid
pending-check deadlocks, rename in-merge, etc.).

## Why no checks are blocking today

GitHub branch-protection on `main` is disabled. The
`/repos/EffortlessMetrics/shiplog/branches/main/protection` endpoint
returns 404 "Branch not protected"; no required-check enforcement is
active. The "target steady state" table above describes what
`blocking = true` lanes in `policy/ci-lanes.toml` would require if
protection were enabled — it is the source of truth for the **intended**
required-check set, not a description of currently-enforced behavior.

Enabling protection (and choosing the corresponding required-check
list) is a separate release decision. Until that happens, every
`ci.yml` job runs but no GitHub-level gate forces a green status
before merge — reviewers and the CodeRabbit/Droid advisory lanes
provide the practical merge gate today.

## See also

- [`policy/ci-lanes.toml`](../../policy/ci-lanes.toml) — `blocking` field per lane
- [`required-check-migration.md`](required-check-migration.md) — sequenced migration
- [`skipped-by-policy.md`](skipped-by-policy.md) — what "skipped" means
- [`ci-lane-map.md`](ci-lane-map.md) — which lanes are blocking today
