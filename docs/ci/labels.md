# CI Labels

Labels on a PR change which lanes run. There are two kinds:

- **Spend authorization** labels — the PR is going to cost more than the
  default tier; the author or reviewer is acknowledging the spend.
- **Routing toggles** — force a lane on (or off) regardless of risk-pack
  matching.

This is the human contract. The machine-readable label list lives in
[`policy/ci-budget.toml`](../../policy/ci-budget.toml) under `[labels]`.

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

| Label | Effect |
|-------|--------|
| `ripr` | Force `ripr` analysis on a PR that would otherwise skip it (e.g. docs-only PR that touches a build script). |
| `ripr-waive` | Suppress `ripr` advisory output for this PR. Use when the finding is known and tracked in [`policy/ripr-suppressions.toml`](../../policy/ripr-suppressions.toml). |
| `coverage` | Run the coverage lane on PR. |
| `mutation` | Run targeted mutation on PR (per matched risk-pack scope). |
| `property-tests` | Run the broad property-test sweep on PR (overrides bounded smoke). |
| `fuzz` | Run the full fuzz matrix on PR (overrides touched-target smoke). |
| `bdd` | Run the broader BDD scope on PR (overrides critical-flow smoke). |
| `security-audit` | Run the standalone `security.yml` workflow on PR (otherwise routed to manifest changes / weekly). |
| `release-check` | Run the release preflight (`package-proof.sh`, `publish-dry-run.sh`) on PR. |

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
