# `ripr` Operating Doc

`ripr` is shiplog's PR-time oracle-gap detector. It runs as an advisory
lane on every PR (added in PR #153) and answers a mutation-shaped question
**without** running mutations:

> **v0.5.0 status:** The lane in PR #153 ships as a **stub** — the
> workflow, configuration, and contract artifacts (`target/ripr/ripr.json`
> + `.sarif`) are wired so downstream tooling (PR plan, CI actuals,
> future required-check promotion) can treat the lane as live, but the
> stub does not perform real exposure analysis and always reports zero
> findings. Real ripr integration is a follow-up release. The
> doctrine, suppressions schema, severity table, and label model below
> are forward-stable.

> For the behavior changed in this diff, do current tests appear to
> expose that behavior to a meaningful test discriminator?

This pairs with [`mutation.md`](mutation.md) (the mutation lane it
complements) and
[`cost-and-verification-policy.md`](cost-and-verification-policy.md) (the
ripr-vs-mutation economics framing). The doctrine is established in
[`test-evidence-lanes.md`](test-evidence-lanes.md#ripr-and-mutation-economics).

## What ripr claims

A green ripr report means: for the changed code in this diff, the static
analysis did not find a reachable behavior that lacks an exposed
discriminator in current tests. Loosely: "the diff is probably oracle-covered."

A non-green ripr report flags one or more of:

| Severity | Meaning |
|---|---|
| `exposed` | Changed behavior is observable to at least one test discriminator. Notice; usually no action. |
| `weakly_exposed` | Changed behavior is observable but only to weak discriminators (e.g. assert that doesn't distinguish output values). Warning. |
| `reachable_unrevealed` | Changed behavior is reachable from tests but no oracle distinguishes it from the original behavior. Warning. |
| `no_static_path` | ripr could not find a static path from any test to the changed behavior. Notice; may indicate dead code or a test gap. |
| `infection_unknown` | Mutation analog ambiguous — the static analysis could not determine if the change would be caught. Notice. |
| `propagation_unknown` | Whether the change propagates to test-observable output is uncertain. Notice. |
| `static_unknown` | The static analysis itself was uncertain about a path. Notice. |

Severity → action mapping is configured in
[`policy/ripr-suppressions.toml`](../../policy/ripr-suppressions.toml)
header (`[suppressions]`) and can be overridden per-PR via labels.

## What ripr does NOT claim

- ripr does not run mutants. It does not produce killed/survived counts.
- ripr does not prove the test suite is adequate. It surfaces oracle
  gaps; full mutation testing (the weekly run + targeted runs) is the
  authoritative oracle-adequacy evidence.
- ripr does not prove the changed behavior is correct. It only addresses
  whether tests would notice a regression.
- ripr does not replace targeted mutation. Severe ripr findings should
  trigger a targeted mutation run on the touched scope.

## Lane shape

| Aspect | Value |
|---|---|
| Workflow | `.github/workflows/ripr.yml` (added in PR #153) |
| Trigger | PR open / sync; can be forced via `ripr` label or skipped via `ripr-waive` label |
| Runner | `ubuntu-latest` |
| Base LEM | 4 |
| Default PR? | yes |
| Blocking | no (advisory) |
| Outputs | `target/ripr/ripr.json`, `target/ripr/ripr.sarif`, GitHub step summary |

The workflow triggers on Rust diffs (paths under `apps/**`, `crates/**`,
`xtask/**`, `Cargo.toml`, `Cargo.lock`, `ripr.toml`,
`policy/ripr-suppressions.toml`).

## Labels

| Label | Effect |
|---|---|
| `ripr` | Force ripr to run on a PR that would otherwise skip (e.g. docs-only PR that touches a build script) |
| `ripr-waive` | Suppress ripr advisory output for this PR. Use when the finding is known and tracked in `policy/ripr-suppressions.toml` |

## Suppressions

Tracked in [`policy/ripr-suppressions.toml`](../../policy/ripr-suppressions.toml).
Each suppression entry is reserved for findings the team has decided are
acceptable, typically because:

- A property test or mutation calibration covers the equivalence class
- The finding is in a code path unreachable from production
- The finding is a known false positive in ripr's static analysis

The suppression carries the standard receipt fields (`id`, `owner`,
`reason`, `created`, `review_after`, `expires`) plus
`tracked_by_test` / `tracked_by_mutation_run` for evidence linkage.

## When a ripr finding triggers targeted mutation

A `weakly_exposed` or `reachable_unrevealed` finding on a high-risk
surface should trigger a targeted mutation run. PR #155 wires the
auto-routing:

| Trigger | Auto-applies |
|---|---|
| `ripr` finding severity `weakly_exposed` AND touched path matches `redaction-privacy` / `bundle-share` / `schema-ids` risk pack | `mutation` label → runs `lane.mutation_targeted` |
| `ripr` finding severity `reachable_unrevealed` on any path | reviewer attention; may add `mutation` label manually |

The targeted mutation run produces concrete killed/survived data for the
touched crate, which calibrates the ripr finding.

## Mutation calibration

ripr is a static analysis that approximates a mutation question. It can
be wrong (false positives) or miss things (false negatives). The
weekly mutation lane (`lane.mutation_weekly`) and the targeted mutation
lane (`lane.mutation_targeted`) provide ground truth that calibrates
ripr over time:

- A ripr finding that's reliably caught by mutation should keep the
  same severity (or escalate).
- A ripr finding that mutation reliably contradicts (e.g. mutation
  shows the equivalence class IS killed) should be added to
  `policy/ripr-suppressions.toml` with `tracked_by_mutation_run`.
- A code path that mutation flags as surviving but ripr did not should
  drive ripr config tuning.

This is the "mutation calibrates ripr" relationship from
[`cost-and-verification-policy.md`](cost-and-verification-policy.md#ripr-and-mutation-economics).

## Configuration

`ripr.toml` (added in PR #153) at the repo root:

```toml
schema_version = 1

[analysis]
mode = "diff"

[policy]
fail_on = []   # advisory only; no severity blocks the lane

[report]
max_related_tests = 8
include_context = true

[severity]
exposed = "notice"
weakly_exposed = "warning"
reachable_unrevealed = "warning"
no_static_path = "notice"
infection_unknown = "notice"
propagation_unknown = "notice"
static_unknown = "notice"

[suppressions]
path = "policy/ripr-suppressions.toml"
```

`fail_on = []` keeps the lane advisory in v0.5.0. Promotion to
severity-gated blocking is a follow-up release decision after the
suppressions ledger has matured and mutation calibration is observed.

## See also

- [`policy/ripr-suppressions.toml`](../../policy/ripr-suppressions.toml) — suppressions ledger
- [`mutation.md`](mutation.md) — the mutation lane that calibrates ripr
- [`test-evidence-lanes.md`](test-evidence-lanes.md#ripr-and-mutation-economics) — the ripr-vs-mutation doctrine
- [`cost-and-verification-policy.md`](cost-and-verification-policy.md) — economics framing
- [`labels.md`](labels.md) — `ripr` and `ripr-waive` label semantics
- [`verification-ladder.md`](verification-ladder.md) — where ripr sits in the signal-vs-cost ladder (rung 10d)
