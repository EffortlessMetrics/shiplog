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

## Current enforcement state

The two repositories have different GitHub branch-protection state:

- `EffortlessMetrics/shiplog-swarm/main` is protected. It requires exactly
  `Shiplog Rust Small Result`, with strict status checks enabled. Force pushes
  and branch deletion are disabled; administrator enforcement is disabled.
- `EffortlessMetrics/shiplog/main` is governed by the active repository
  ruleset `main` (ruleset `12681248`). It requires the metadata-only
  `reject-routine-bot-pr` check, and also enforces pull-request-only merges,
  deletion protection, and non-fast-forward protection. The legacy
  `/branches/main/protection` endpoint returns 404 because source uses a
  ruleset rather than the legacy branch-protection API. There are no bypass
  actors in the ruleset.

The swarm setting is the normal-development merge gate. The source setting is
the release/public-surface boundary and must not be changed as part of an
ordinary swarm PR.

## Required checks (target steady state)

These describe the future correctness checks that should be added to the
source ruleset after they have completed a safe migration cycle. The active
ruleset currently requires only `reject-routine-bot-pr`, which is the
source-side merge-control boundary for routine automation. The legacy
`/branches/main/protection` endpoint returning 404 is not evidence that the
source branch is unprotected. The table is the **target** set, encoded by the
routed default PR summary and by
`blocking = true` lanes in
[`policy/ci-lanes.toml`](../../policy/ci-lanes.toml). It describes what
should be additionally required on `shiplog/main` after migration, not what
is currently enforced on either repository.

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
branch-protection setting change under the rule above. Because the source
ruleset was not configured during that ladder, the correctness-check changes
remain forward-looking guidance. The later source ruleset rollout added the
routine-automation guard separately; swarm protection is recorded in the
current enforcement section above.

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

The sequencing rules in [`required-check-migration.md`](required-check-migration.md)
describe how to add the remaining correctness checks safely (avoid
pending-check deadlocks, rename in-merge, etc.).

## Why source correctness checks are not blocking today

The active `main` ruleset on `shiplog` currently requires only
`reject-routine-bot-pr`. That guard blocks routine Dependabot and Factory
Droid product PRs while allowing normal authors, but it is not a substitute
for the broader correctness set below. The `shiplog-swarm/main` branch does
enforce `Shiplog Rust Small Result`; that is the normal-development gate.
The "target steady state" table above describes what `blocking = true` lanes
in `policy/ci-lanes.toml` should add to the source ruleset after migration;
it is not a description of currently enforced source checks.

Adding the remaining correctness checks to `shiplog/main` is a separate
release decision. Until that happens, the configured `ci.yml` routes still
run for pushes to `main` and `full-ci` pull requests, but the source ruleset
does not yet force the broader correctness set before merge. The required
`reject-routine-bot-pr` check remains the source merge-control boundary.

## See also

- [`policy/ci-lanes.toml`](../../policy/ci-lanes.toml) — `blocking` field per lane
- [`required-check-migration.md`](required-check-migration.md) — sequenced migration
- [`skipped-by-policy.md`](skipped-by-policy.md) — what "skipped" means
- [`ci-lane-map.md`](ci-lane-map.md) — which lanes are blocking today
