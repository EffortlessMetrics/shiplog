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

**Do not require matrix leaves.** The Windows + Ubuntu CI matrix produces
two `Check (...)` checks. Requiring a specific matrix leaf locks the
required-check name to the matrix shape. If the matrix changes, branch
protection breaks. Require the umbrella check instead.

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

These are what should appear in branch-protection settings for `main` after
the rollout. **Today (post-#142), branch protection has not been changed
yet** — these are the planned set, encoded in
[`policy/ci-lanes.toml`](../../policy/ci-lanes.toml) via `blocking = true`
on the relevant lanes.

| Required check | Source | Why required |
|---|---|---|
| `CI / Check (ubuntu-latest)` | `ci.yml` | Primary Rust correctness gate |
| `CI / Check (windows-latest)` | `ci.yml` | Cross-platform parity (covered by `ci-exception-0001`) |
| `CI / cargo-deny` | `ci.yml` | Dependency policy |
| `CI / Policy gates` | `ci.yml` (added in PR #165) | Policy ledger enforcement: schemas + 10 blocking-allowlist file/lint/panic checks |
| `pr-plan / forecast` | `pr-plan.yml` (added in PR #146) | LEM forecast + risk-pack receipt |

The `MSRV (1.95)` job that previously appeared here was dropped in PR
#164 as redundant compile coverage with `Check (ubuntu-latest)` (both
pin `toolchain: 1.95.0`).

These should **not** be required:

- `Mutation Testing` — weekly only, advisory
- `Coverage` — push-main only or label-gated, advisory
- `Quick Fuzz (CI)` — bounded smoke is required-eligible; full matrix is nightly
- `BDD & Property Tests`, `Ingest Crate Tests`, `Render, Trust, and Contract Crates`, `Optional Surface Tests`, `App Tests` — full matrix today; bounded critical-flow smoke (PR #154) is required-eligible after lane carve-out
- `Property Testing` — full sweep today; bounded smoke (PR #154) is required-eligible
- `droid-review`, `droid`, `droid-security-scan` — advisory bot review
- `ripr` (added in PR #153) — advisory advisory
- `Cargo Deny Security` (`security.yml`) — duplicate of `CI / cargo-deny` on PR

## Migration plan

The actual GitHub branch-protection settings move when:

- PR #146 lands the PR plan, so `pr-plan / forecast` exists as a check.
- PR #154 lands bounded stochastic, so the small-cost stochastic smokes
  become required-eligible.
- PR #155 lands lane routing, so the broad sweeps move to nightly /
  label.

The migration sequence is documented in
[`required-check-migration.md`](required-check-migration.md).

## Why no checks are blocking on Phase 0–1

PRs #140, #141, and #142 are docs / TOML only. They do not change branch
protection. The first PR that changes branch protection is #146 (adds
`pr-plan / forecast`). Hard required-check changes for the broader lane
moves happen in their own follow-up PR (likely after PR #148 confirms
actuals).

## See also

- [`policy/ci-lanes.toml`](../../policy/ci-lanes.toml) — `blocking` field per lane
- [`required-check-migration.md`](required-check-migration.md) — sequenced migration
- [`skipped-by-policy.md`](skipped-by-policy.md) — what "skipped" means
- [`ci-lane-map.md`](ci-lane-map.md) — which lanes are blocking today
