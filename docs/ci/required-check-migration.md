# Required-Check Migration

How GitHub branch-protection required checks move across the v0.5.0
rollout, and why each move happens.

This pairs with [`branch-protection.md`](branch-protection.md) (the rule)
and [`ci-lane-map.md`](ci-lane-map.md) (the inventory). It exists so a
later PR cannot quietly land a required-check change without a recorded
migration step.

## The headline

```text
Today  (post-#142):  4 required checks on `main`  (CI / Check ubuntu, CI / Check windows, CI / cargo-deny, CI / MSRV)
Target (post-#155):  5 required checks on `main`  (the same 4, plus `pr-plan / forecast`; MSRV optionally dropped if redundant)
```

Required-check renames and additions happen at specific points in the
ladder. Required-check removals happen as separate PRs after the lane
they replace has been observed stable.

## Today (post-#145)

| Required check | Source | Notes |
|---|---|---|
| `CI / Check (ubuntu-latest)` | `ci.yml` | Primary correctness gate |
| `CI / Check (windows-latest)` | `ci.yml` | Cross-platform parity |
| `CI / cargo-deny` | `ci.yml` | Dependency policy |
| `CI / MSRV (1.95)` | `ci.yml` | MSRV regression (renamed from `MSRV (1.92)` in PR #145) |

PRs #140–#143 did not touch branch protection. PR #145 (this rename)
required a same-merge branch-protection update.

## At PR #145 (MSRV bump 1.92 → 1.95)

The MSRV job in `ci.yml` is renamed from `MSRV (1.92)` to `MSRV (1.95)`
when its toolchain pin changes. **This is a required-check rename**, which
is destructive: GitHub treats the new check name as a new check.

| Required check | Action | Why |
|---|---|---|
| `CI / MSRV (1.92)` | rename to `CI / MSRV (1.95)` | Toolchain pin matches workspace MSRV |

PR #145 must update branch protection in the same merge so the rename
does not leave PRs blocked on a check that no longer exists. The PR
description includes the branch-protection settings change as a sub-step.

## At PR #146 (advisory LEM PR plan)

Adds the `pr-plan` workflow, which writes `target/ci/ci-plan.json` and a
GitHub step summary.

| Required check | Action | Why |
|---|---|---|
| `pr-plan / forecast` | **add** as required | Forces every PR to carry a plan + LEM forecast |

Adding a required check is non-destructive (no rename), but the first PR
to be merged after `pr-plan / forecast` becomes required must have run
the workflow. Do this after the workflow has been live on `main` for at
least one PR cycle.

## At PR #147 (cache normalization)

Optional removal: `CI / MSRV (1.95)` becomes redundant with
`CI / Check (ubuntu-latest)` because the `Check` job already compiles on
the toolchain pin (which now equals MSRV).

| Required check | Action | Why |
|---|---|---|
| `CI / MSRV (1.95)` | optionally remove | Redundant with `Check` while toolchain pin == MSRV |

This is a removal, which is non-destructive but should not happen in the
same PR as the cache change. Track as a follow-up if PR #147 reviewers
want it pulled out.

## At PR #154 (bounded stochastic PR-fast lane)

Adds bounded smoke jobs to `bdd-testing.yml`, `property-testing.yml`,
`fuzzing.yml`. These bounded jobs are required-eligible.

| Required check | Action | Why |
|---|---|---|
| `BDD Testing / smoke` (or similar) | add as required (after one cycle on `main`) | Critical-flow CLI smoke is cheap PR-fast signal |
| `Property Testing / smoke` (or similar) | add as required (after one cycle on `main`) | Bounded property invariants are cheap PR-fast signal |
| `Fuzzing / touched-target smoke` (or similar) | add as required (after one cycle on `main`) | Touched-parser robustness is cheap PR-fast signal |

The exact check names are decided in PR #154's design.

## At PR #155 (broad lane routing)

Removes the broad-sweep jobs from default-PR (they move to nightly +
label). The leaf jobs that exist today (full BDD matrix, full property
sweep, all-9-targets fuzz) become PR-skipped-by-policy unless a label
matches.

| Required check | Action | Why |
|---|---|---|
| `BDD Testing / *` (broad jobs) | **must not be required** before this PR | They move to nightly + label routing; required-check would deadlock label-less PRs |
| `Property Testing` (broad) | **must not be required** before this PR | Same |
| `Quick Fuzz (CI)` (full matrix) | **must not be required** before this PR | Same |

If branch protection currently includes any of these as required, the
PR must remove them in the same merge. Today (post-#142) they are not
required, so this is a non-issue.

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
