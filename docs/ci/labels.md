# CI Labels

Labels on a PR change which lanes run. There are two kinds:

- **Spend authorization** labels — the PR is going to cost more than the
  default tier; the author or reviewer is acknowledging the spend.
- **Routing toggles** — force a lane on (or off) regardless of risk-pack
  matching.

This is the human contract. The machine-readable label list lives in
[`policy/ci-budget.toml`](../../policy/ci-budget.toml) under `[labels]`.

The `enforced` vs `declared but not enforced today` split is itself a
machine-checked invariant: `cargo xtask check-label-enforcement` parses
`policy/ci-budget.toml [labels]`, scans `.github/workflows/*.yml` for
`contains(github.event.pull_request.labels.*.name, '<label>')`
references, and fails if a label moves between categories without
updating `[labels_enforcement].declared_only`. The check runs in the
`Policy gates` job on every PR. See
[`policy/ci-budget.toml`](../../policy/ci-budget.toml) for the
declared-only list.

## Spend authorization

| Label | Tier | Effect |
|-------|------|--------|
| `ci-budget-ack` | elevated (≤75 LEM) | Acknowledge that the PR exceeds `default_limit_lem` (35 LEM). Required for elevated tier. |
| `ci-budget-override` | hard (≤125 LEM) | Acknowledge that the PR exceeds `elevated_limit_lem` (75 LEM). Required for hard tier. |
| `full-ci` | hard (implies override) | Force every targeted lane on the PR. Use when the change is high-risk or cross-cutting. Implies `ci-budget-override`. |

When the PR plan ([`ci-plan-json.md`](ci-plan-json.md), PR #146) projects a
total above `default_limit_lem`, it expects to see `ci-budget-ack`. Without
it, the plan emits a warning. Hard enforcement is a follow-up release
decision.

## Routing toggles

| Label | Status | Effect |
|-------|--------|--------|
| `coverage` | enforced (coverage.yml job-if) | Run the coverage lane on PR. |
| `mutation` | enforced (mutation-testing.yml job-if) | Run targeted mutation on PR (per matched risk-pack scope). |
| `property-tests` | enforced (property-testing.yml job-if) | Run the broad property-test sweep on PR (overrides bounded smoke). |
| `fuzz` | enforced (fuzzing.yml job-if) | Run the full fuzz matrix on PR (overrides touched-target smoke). |
| `bdd` | enforced (bdd-testing.yml job-if) | Run the broader BDD scope on PR (overrides critical-flow smoke). |
| `security-audit` | enforced (security.yml job-if) | Run the standalone `security.yml` workflow on PR (otherwise routed to manifest changes / weekly). |
| `ripr` | declared but **not enforced today** | Intended to force `ripr` analysis on a PR that would otherwise skip it. Declared in `policy/ci-budget.toml [labels].ripr_force` and `[lane.ripr_advisory].labels` (`policy/ci-lanes.toml`), but `ripr.yml` is currently paths-driven (`apps/**`, `crates/**`, `xtask/**`, `Cargo.toml`, `Cargo.lock`, `ripr.toml`, `policy/ripr-suppressions.toml`), not label-driven. Routing planned for the real ripr integration follow-up release. |
| `ripr-waive` | declared but **not enforced today** | Intended to suppress `ripr` advisory output. The label name lives in `policy/ci-budget.toml [labels].ripr_waive` and `[lane.ripr_advisory].labels`, but the v0.5.0 stub `ripr.yml` emits its artifacts unconditionally on path match. Suppressions are tracked in [`policy/ripr-suppressions.toml`](../../policy/ripr-suppressions.toml); honoring the label is part of the real ripr integration follow-up release. |
| `release-check` | declared but **not enforced today** | Intended to run the release preflight (`package-proof.sh` + `publish-dry-run.sh`) on PR. Declared in `policy/ci-budget.toml [labels].release_check`, but `release.yml` only triggers on tag push and `workflow_dispatch` — no label-gated PR path exists. Adding a PR-time release-check workflow is a follow-up. |

## What a label is not

A label is **not** "make CI greener" or "skip a check I find inconvenient."
The label model is reviewable spend / scope:

- High-cost labels (`full-ci`, `ci-budget-override`) require a human to take
  responsibility for the spend.
- Routing labels (`ripr`, `coverage`, etc.) opt **in** to expensive lanes;
  there are no labels that opt **out** of required gates.

If a required gate is the wrong choice for a particular PR, that is a
[`required-check-migration.md`](required-check-migration.md) discussion, not
a label.

## How labels combine with risk packs

Risk packs (see [`risk-packs.md`](risk-packs.md)) auto-apply some labels and
auto-select some lanes based on changed paths. Manual labels override or
extend risk-pack auto-routing, never replace it.

Example: a PR that touches `crates/shiplog-redact/` matches the
`redaction-privacy` risk pack, which auto-applies the `mutation` label and
selects `mutation_targeted` + `property` lanes. The author can additionally
apply `full-ci` to run every targeted lane (e.g. coverage + BDD).

## Label hygiene

- A reviewer can apply / remove routing labels (`ripr-waive`, `coverage`,
  etc.) at any point in review.
- Spend-authorization labels (`ci-budget-ack`, `ci-budget-override`,
  `full-ci`) should be set by the author or by the reviewer who is asking
  for the elevated lanes.
- Removing `ci-budget-ack` from a PR that the plan flagged as needing it
  causes the next CI run to re-emit the warning. That is intended.

## See also

- [`policy/ci-budget.toml`](../../policy/ci-budget.toml) — `[labels]` table (machine-readable)
- [`ci-lane-map.md`](ci-lane-map.md) — which lanes correspond to which labels
- [`risk-packs.md`](risk-packs.md) — auto-applied label/lane selection by changed-path pattern
- [`cost-and-verification-policy.md`](cost-and-verification-policy.md) — the spend-vs-signal doctrine
- [`per-pr-acceptance-contract.md`](per-pr-acceptance-contract.md) — the per-PR template that records label decisions
