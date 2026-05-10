# Skipped-by-Policy

When a CI lane skips for a particular PR, the lane must say **why**. Not
"did not run." Not silent absence. A definite skip with a categorised
reason.

This pairs with [`branch-protection.md`](branch-protection.md): only
definite, declared skips are acceptable to branch protection. A pending
or absent check is not.

## Skip categories

A lane that skips for a PR must report one of these categories in its
step summary (and, eventually, in the `target/ci/ci-actuals.json` record):

| Category | Meaning | Example |
|---|---|---|
| `docs-only` | The PR matches the `docs-only` risk pack and only the `docs-only` risk pack | A PR that only changes `docs/**` and `*.md` |
| `no-matching-risk-pack` | The lane is opt-in via risk pack, and no risk pack on this PR selects it | A PR touching `crates/shiplog-engine/` that does not select `mutation_targeted` |
| `label-absent` | The lane is opt-in via label, and the label is not present | A PR without `coverage` label does not run `coverage.yml` |
| `nightly-only` | The lane only runs on cron, never on PR | `Extended Fuzz`, `Mutation Testing` weekly |
| `release-only` | The lane only runs on tag push | `release.yml` jobs |
| `ripr-waived` | A `ripr-waive` label suppressed the advisory output for this PR | Author has waived a known finding |
| `duplicate` | The lane's intent is already produced by another lane on this PR | `security.yml` cargo-deny on a PR where `ci.yml` cargo-deny already ran |

A lane that genuinely failed should report `failure`, not a skip
category.

## How a skip is reported

Each skipping lane writes to its GitHub step summary:

```markdown
> Skipped by policy: docs-only
>
> This PR matches the `docs-only` risk pack only. The lane is intended
> for code changes; rerun with the `coverage` label if you need
> coverage on a docs-touching change.
```

The `docs/ci/ci-actuals.md` schema (PR #148) extends this to a
machine-readable receipt:

```json
{
  "workflow": "Coverage",
  "job": "Codecov Coverage",
  "conclusion": "skipped",
  "skip_reason": "no-matching-risk-pack",
  "skip_detail": "PR matched docs-only; coverage requires 'coverage' or 'full-ci' label"
}
```

## Why definite skips matter

Without a skip category, three failure modes appear:

1. **Pending forever.** Branch protection waits on a check that will
   never run, and no PR can merge.
2. **Silent regression.** A lane that should have run for this risk
   surface didn't, and no one noticed because the absence wasn't
   reported.
3. **Bot confusion.** Review bots interpret "did not run" as "passed,"
   which means a misrouted PR can ship with no actual evidence.

Definite skips fix all three: branch protection sees the check report
"skipped" (acceptable), reviewers can see in the PR plan which lanes
were skipped and why, and bots see explicit skip context rather than
absence.

## Who is responsible for a skip

| Skip category | Owner | When to override |
|---|---|---|
| `docs-only` | author | Apply `coverage` / `full-ci` if the docs change pulls in a code path |
| `no-matching-risk-pack` | reviewer | Apply the relevant label if the PR is touching a risk surface the path patterns missed |
| `label-absent` | reviewer | Apply the label if signal is needed |
| `nightly-only` | release engineer | Cannot override on PR; the PR plan can opt-in to a one-off via `dispatch` |
| `release-only` | release engineer | Cannot override on PR |
| `ripr-waived` | reviewer | Remove the `ripr-waive` label if the waiver no longer applies |
| `duplicate` | release/ci | Should not occur after PR #155 routes the duplicate `security.yml` lane |

## What this PR adds

PR #142 adds the documented contract. Implementation:

- PR #146 emits skip categories in the PR plan + step summary.
- PR #148 records skips in `target/ci/ci-actuals.json` against the v1
  schema.
- PR #155 wires the actual `if:` conditions in workflow YAML so skips
  happen with explicit reasons rather than via `if: false` hacks.

## See also

- [`branch-protection.md`](branch-protection.md) — why definite skips are required
- [`ci-actuals.md`](ci-actuals.md) — the receipt schema that records skips
- [`risk-packs.md`](risk-packs.md) — how risk packs select / skip lanes
- [`labels.md`](labels.md) — how labels override skip categories
