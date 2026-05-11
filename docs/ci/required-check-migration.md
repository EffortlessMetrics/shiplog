# Required-Check Migration

How GitHub branch-protection required checks move across the v0.5.0
rollout, and why each move happens.

This pairs with [`branch-protection.md`](branch-protection.md) (the rule)
and [`ci-lane-map.md`](ci-lane-map.md) (the inventory). It exists so a
later PR cannot quietly land a required-check change without a recorded
migration step.

## The headline

```text
Today  (post-v0.5.0):  0 required checks on `main`
                       (`main` is not branch-protected; verified via
                       /repos/EffortlessMetrics/shiplog/branches/main/protection)
Target (steady state): 5 required checks on `main`
                       (CI / Check ubuntu, CI / Check windows,
                        CI / cargo-deny, CI / Policy gates,
                        pr-plan / forecast)
```

The v0.5.0 ladder (#140–#157) shipped without enabling GitHub
branch-protection on `main`. The migration rules below describe how
required-check changes **must** be sequenced once protection is
enabled — they are forward-looking guidance, not a retrospective
changelog of changes that have already been applied.

Required-check renames and additions happen at specific points in the
ladder. Required-check removals happen as separate PRs after the lane
they replace has been observed stable.

## Today (post-v0.5.0)

`main` is not branch-protected; no required checks are enforced at the
GitHub level. When protection is enabled, the target set is what
`policy/ci-lanes.toml` marks `blocking = true` for `default_pr = true`
lanes:

| Required check (target) | Source | Notes |
|---|---|---|
| `CI / Check (ubuntu-latest)` | `ci.yml` | Primary correctness gate |
| `CI / Check (windows-latest)` | `ci.yml` | Cross-platform parity |
| `CI / cargo-deny` | `ci.yml` | Dependency policy |
| `CI / Policy gates` | `ci.yml` (added in PR #165) | Policy ledger enforcement: schemas + 10 blocking-allowlist file/lint/panic checks |
| `pr-plan / forecast` | `pr-plan.yml` (added in PR #146; never enforced as required during v0.5.0) | LEM forecast + risk-pack receipt |

The `CI / MSRV (1.95)` check previously appeared here; it was dropped
in PR #164 as redundant compile coverage with `Check (ubuntu-latest)`.

PRs #140–#143 did not touch branch protection. PR #145 (the
MSRV-rename PR below) would have required a same-merge
branch-protection update **if `main` had been protected at the time**;
because protection was never enabled during the v0.5.0 ladder, no
actual GitHub setting changes were performed.

## At PR #145 (MSRV bump 1.92 → 1.95)

The MSRV job in `ci.yml` was renamed from `MSRV (1.92)` to `MSRV (1.95)`
when its toolchain pin changed. **This is a required-check rename**, which
is destructive under enabled branch protection: GitHub treats the new
check name as a new check.

| Required check | Action | Why |
|---|---|---|
| `CI / MSRV (1.92)` | rename to `CI / MSRV (1.95)` | Toolchain pin matches workspace MSRV |

Under enabled protection, PR #145 would have required a same-merge
branch-protection settings update so the rename did not leave PRs
blocked on a check that no longer existed. Because `main` was not
protected at the time, the rename shipped without a settings update.
The check was subsequently removed in PR #164 (see below).

## At PR #146 (advisory LEM PR plan)

Added the `pr-plan` workflow, which writes `target/ci/ci-plan.json` and
a GitHub step summary.

| Required check | Action | Why |
|---|---|---|
| `pr-plan / forecast` | **add** as required (when protection is enabled) | Forces every PR to carry a plan + LEM forecast |

Adding a required check is non-destructive (no rename), but the first
PR merged after `pr-plan / forecast` becomes required must have run
the workflow. Wait until the workflow has been live on `main` for at
least one PR cycle before adding it.

`pr-plan / forecast` has been live since PR #146 merged; it is
required-eligible but has not been added to branch-protection settings
(branch protection is not enabled on `main`).

## At PR #164 (MSRV job removal)

`CI / MSRV (1.95)` was redundant with `CI / Check (ubuntu-latest)`
because the `Check` job already compiled on the toolchain pin (which
equalled MSRV). The original v0.5.0 plan noted this as an optional
follow-up to PR #147 (cache normalization); it was carved out into its
own dedicated PR after v0.5.0 shipped.

| Required check | Action | Why |
|---|---|---|
| `CI / MSRV (1.95)` | **removed** | Redundant with `Check` while toolchain pin == MSRV |

This is a removal, which is non-destructive (no rename, no new check
name). Because `main` was not protected at the time, no branch-protection
settings update was required.

## At PR #165 (Policy gates job)

Added a `Policy gates` job to `ci.yml` that runs all 11 `cargo xtask
check-*` policy ledger gates (`check-policy-schemas` plus 10
`--mode blocking-allowlist` checks).

| Required check | Action | Why |
|---|---|---|
| `CI / Policy gates` | **add** as required (when protection is enabled) | Catches drift in any of the 18 policy ledgers |

`CI / Policy gates` has been live since PR #165 merged; it is
required-eligible but has not been added to branch-protection settings
(branch protection is not enabled on `main`).

## At PR #154 (bounded stochastic PR-fast lane)

Added bounded smoke jobs in their own dedicated workflows:
`bdd-smoke.yml`, `property-smoke.yml`, `fuzz-smoke.yml`. These bounded
jobs are required-eligible.

| Required check | Action | Why |
|---|---|---|
| `BDD Smoke / smoke` | add as required (when protection is enabled) | Critical-flow CLI smoke is cheap PR-fast signal |
| `Property Smoke / smoke` | add as required (when protection is enabled) | Bounded property invariants are cheap PR-fast signal |
| `Fuzz Smoke / smoke` | add as required (when protection is enabled) | Touched-parser robustness is cheap PR-fast signal |

Lanes for these are `[lane.bdd_smoke]`, `[lane.property_smoke]`, and
`[lane.fuzz_smoke]` in `policy/ci-lanes.toml`. All three are live and
required-eligible; none are enforced via branch-protection settings
(branch protection is not enabled on `main`).

## At PR #155 (broad lane routing)

Routed the broad-sweep jobs off default-PR (moved to nightly cron +
label). The leaf jobs (full BDD matrix, full property sweep,
all-9-targets fuzz, mutation testing, standalone security cargo-deny,
coverage) now have job-level `if:` blocks that require a matching
label (`bdd`, `property-tests`, `fuzz`, `mutation`, `security-audit`,
`coverage`, or `full-ci`) on PRs. Without the label, the leaf reports
**`skipped`** with category `label-absent` (see
[`skipped-by-policy.md`](skipped-by-policy.md)).

| Required check | Action | Why |
|---|---|---|
| `BDD Testing / *` (broad jobs) | **must not be required** under any protection setting | Label-gated; required-check would deadlock label-less PRs |
| `Property Testing` (broad) | **must not be required** | Same |
| `Quick Fuzz (CI)` (full matrix) | **must not be required** | Same |
| `Mutation Testing` | **must not be required** | Label-gated + weekly cron |
| `Cargo Deny Security` (security.yml) | **must not be required** | Label-gated; duplicate of `CI / cargo-deny` on PR |
| `Codecov Coverage` | **must not be required** | Label-gated; push-main + label only |

`main` is not branch-protected today, so the "must not be required"
guidance is forward-looking: it applies when/if protection is enabled
and is a constraint on what required-checks list to choose.

## Avoiding pending-check deadlocks

The classic GitHub failure mode: branch protection requires
`workflow / job-name`, the workflow is renamed or removed, every PR shows
`workflow / job-name` as `pending` forever, no PR can merge.

The migration rules below avoid this:

1. A PR that renames a required check **must** update branch protection
   in the same merge.
2. A PR that removes a required check leaf **must** update branch
   protection in the same merge.
3. A PR that adds a new required check **may** wait one cycle on `main`
   before adding it to branch protection (proves the check actually
   succeeds).
4. Required checks must reference summary checks where possible (e.g.
   `CI / Check (ubuntu-latest)` is the summary for that matrix leaf).
5. Skipped-by-policy lanes (see [`skipped-by-policy.md`](skipped-by-policy.md))
   must report a definite skip reason, not "pending."

## See also

- [`branch-protection.md`](branch-protection.md) — the underlying rule
- [`policy/ci-lanes.toml`](../../policy/ci-lanes.toml) — `blocking` field per lane
- [`ci-lane-map.md`](ci-lane-map.md) — current required-check inventory
- [`skipped-by-policy.md`](skipped-by-policy.md) — what counts as an acceptable skip
