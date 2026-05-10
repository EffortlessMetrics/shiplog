# CI Lane Map

The mechanical inventory of every CI workflow and job, what it claims, what
it costs, and how it is routed. This is the human-readable companion to
[`policy/ci-lanes.toml`](../../policy/ci-lanes.toml). When the two disagree,
the TOML is authoritative; this doc explains it.

It is added in PR #142 (Phase 1 — machine-readable roadmap) and is the
anti-duplication map: every workflow/job declares one **intent**, and any
duplicate intent declares `duplicate_of`. See
[`cost-and-verification-policy.md`](cost-and-verification-policy.md) for the
doctrine.

## Reading the table

| Column | Meaning |
| ------ | ------- |
| Workflow | Path under `.github/workflows/` (or planned addition in a later PR) |
| Job | The job name inside the workflow (when the workflow has multiple) |
| Trigger | What causes the job to run today |
| Default PR? | Runs on every PR by default (yes/no) |
| Blocking | Failure blocks merge (yes/no/advisory) |
| Runner | GitHub-hosted runner the job uses |
| Base LEM | Estimated [Linux Equivalent Minutes](lem-budgeting.md) per run |
| Intent | The single primary thing the job claims |
| Failure mode caught | What real defect this job is designed to surface |
| Evidence | Artifact / step-summary / receipt the job produces |
| `duplicate_of` | Lane that produces the same intent (if any) |
| Target lane | Where this lane should live in the steady-state lane model |
| Owner | Who responds when the lane fails (workspace package or `release`/`policy`/`ci`/`docs`) |

`Base LEM` is the steady-state estimate. Actuals are recorded in
[`ci-actuals.md`](ci-actuals.md) once PR #148 lands.

## ci.yml — primary PR correctness gate

| Job | Trigger | Default PR? | Blocking | Runner | Base LEM | Intent | Failure mode caught | Evidence | duplicate_of | Target lane | Owner |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `Check (ubuntu-latest)` | push main + PR | yes | yes | ubuntu-latest | 12 | Rust correctness | Type/borrow errors, lint regressions, doc-test failure, broken release build, broken canary publish | step summary | — | PR fast | release/ci |
| `Check (windows-latest)` | push main + PR | yes | yes | windows-latest | 36 | Rust correctness (Windows parity) | Windows-only regressions before tag | step summary | — | PR fast (with `ci-exception-0001`) | release/ci |
| `cargo-deny` | push main + PR | yes | yes | ubuntu-latest | 4 | Dependency policy | Disallowed bans/licenses/sources/advisories | step summary | — | PR fast | release/ci |
| `MSRV (1.92)` | push main + PR | yes | yes | ubuntu-latest | 5 | Rust correctness (MSRV) | MSRV regression | step summary | `lane.ci_check` (while pin == MSRV) | candidate to drop in PR #147 | release/ci |

## release.yml — tag-triggered release pipeline

| Job | Trigger | Default PR? | Blocking | Runner | Base LEM | Intent | Failure mode caught | Evidence | duplicate_of | Target lane | Owner |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `release-preflight` | tag `v*` + dispatch | no | yes | ubuntu-latest | 8 | Publish readiness | Package boundary / version drift / cargo-deny / publish-dry-run failure | step summary | — | release | release |
| `build-binary` (4 targets) | tag `v*` + dispatch | no | yes | ubuntu-latest, macos-latest, macos-15-intel, windows-latest | 90 (max leg) | Multi-platform release readiness | Per-target build failure | platform binaries | — | release | release |
| `create-release` | tag `v*` + dispatch | no | yes | ubuntu-latest | 1 | release artifact creation | Draft release create failure | GitHub release object | — | release | release |
| `upload-assets` | tag `v*` + dispatch | no | yes | ubuntu-latest | 2 | Release asset upload | Per-asset upload failure / SHA256SUMS missing | release assets + SHA256SUMS.txt | — | release | release |
| `release-validation` | tag `v*` + dispatch | no | yes | ubuntu-latest | 12 | Release asset smoke | Downloaded artifact does not launch | step summary + downloaded artifact run | — | release | release |
| `release-test` | tag `v*` + dispatch | no | yes | ubuntu-latest | 15 | Release-mode test parity | Release-mode-only failures | test output | `cargo test --release` in `Check` | release | release |

## security.yml — duplicate of ci.yml `cargo-deny` on PR

| Job | Trigger | Default PR? | Blocking | Runner | Base LEM | Intent | Failure mode caught | Evidence | duplicate_of | Target lane | Owner |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `Cargo Deny Security` | push main+develop + PR + weekly cron + dispatch | yes (PR) | no | ubuntu-latest | 4 | Dependency policy | Same as ci.yml `cargo-deny` | step summary | `lane.ci_deny` | manifest changes / weekly / `security-audit` label (PR #155 routing) | release/ci |

## property-testing.yml — bounded smoke + broad nightly

| Job | Trigger | Default PR? | Blocking | Runner | Base LEM | Intent | Failure mode caught | Evidence | duplicate_of | Target lane | Owner |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `Property Testing` | push main + PR | yes (broad form today) | no | ubuntu-latest | 20 | Schema/property invariant | Invariant breaks under randomized input | step summary | — | PR fast (smoke 16-64 cases on selected invariants in PR #154) + PR-targeted (risk-pack scope at elevated cases in PR #155) + nightly (full sweep in PR #155) | policy |

## bdd-testing.yml — bounded smoke + broad nightly

| Job | Trigger | Default PR? | Blocking | Runner | Base LEM | Intent | Failure mode caught | Evidence | duplicate_of | Target lane | Owner |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `BDD & Property Tests` | push + PR | yes (full form today) | no | ubuntu-latest | 8 | CLI/user-flow behavior proof | testkit BDD scenario regression | step summary | — | PR fast (1-2 critical-flow smokes in PR #154) | policy |
| `Ingest Crate Tests` | push + PR | yes (full form today) | no | ubuntu-latest | 8 | Source adapter behavior | Per-adapter regression | step summary | — | PR-targeted (`source-adapter` risk pack in PR #155) | policy |
| `Render, Trust, and Contract Crates` | push + PR | yes (full form today) | no | ubuntu-latest | 8 | Stable contract + trust surface behavior | Contract/trust regression | step summary | — | PR-targeted (relevant risk packs in PR #155) | policy |
| `Optional Surface Tests` | push + PR | yes (full form today) | no | ubuntu-latest | 4 | Optional feature behavior | `team` / `merge` / `workstreams` regression | step summary | — | PR-targeted in PR #155 | policy |
| `App Tests` | push + PR | yes (full form today) | no | ubuntu-latest | 8 | App feature surface behavior | `--no-default-features` and `--all-features` divergence | step summary | — | PR-targeted (`cli/product` risk pack in PR #155) | policy |

## fuzzing.yml — touched-target smoke + nightly extended

| Job | Trigger | Default PR? | Blocking | Runner | Base LEM | Intent | Failure mode caught | Evidence | duplicate_of | Target lane | Owner |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `Quick Fuzz (CI)` | push main + PR | yes (all 9 targets today) | no | ubuntu-latest | 10 | Parser robustness | Crash on small inputs across 9 parser surfaces | crash artifacts on failure | — | PR fast (touched-target only at 30-90s in PR #154) + PR-targeted (`parsers` risk pack in PR #155) + nightly (full 9-target matrix in PR #155) | policy |
| `Extended Fuzz (matrix)` | nightly cron + dispatch | no | no | ubuntu-latest | 540 (60min × 9) | Parser robustness (deep) | Crash with significant fuzz budget | corpus + crash artifacts | — | nightly | policy |

## mutation-testing.yml — already correctly off PR

| Job | Trigger | Default PR? | Blocking | Runner | Base LEM | Intent | Failure mode caught | Evidence | duplicate_of | Target lane | Owner |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `Mutation Testing` | PR=skipped, Mon weekly cron, dispatch | no | no | ubuntu-latest | 120 | Test-strength evidence | Surviving mutants in trust surfaces | mutants report artifact (30d) | — | nightly + label / risk pack (targeted in PR #155) + release-readiness snapshot (PR #155) | policy |

## coverage.yml — already correctly off default PR

| Job | Trigger | Default PR? | Blocking | Runner | Base LEM | Intent | Failure mode caught | Evidence | duplicate_of | Target lane | Owner |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `Codecov Coverage` | push main + PR with `coverage`/`full-ci` label + dispatch | no | no | ubuntu-latest | 45 | Execution-surface measurement | Coverage drift on a flag | `coverage.json` + `lcov.info` + `coverage-receipt.json` | — | main + label + release | release/ci |

## droid-* — advisory automation

| Workflow | Job | Trigger | Default PR? | Blocking | Runner | Base LEM | Intent | Owner |
|---|---|---|---|---|---|---|---|---|
| `droid.yml` | `droid` | `@droid` mentions | no | no | ubuntu-latest (proxied to MiniMax) | 5 | Out-of-band interactive bot | policy |
| `droid-review.yml` | `droid-review` | PR open / sync / ready_for_review / reopened | yes | no | external_ai_review (4×) | 20 | Advisory bot review (review + security severity ≥ high) | policy |
| `droid-security-scan.yml` | `droid-security-scan` | Mon weekly + dispatch | no | no | external_ai_review (4×) | 20 | Advisory weekly security scan | policy |

See [`bot-review-policy.md`](bot-review-policy.md) for the bot review policy.

## Planned lanes (not yet present)

| Workflow / lane | Added in | Intent |
|---|---|---|
| `pr-plan.yml` | PR #146 | LEM forecast + lane selection visibility (writes `target/ci/ci-plan.json`) |
| `ripr.yml` | PR #153 | Reachable-mutant exposure analysis (advisory) |
| (new bounded-smoke jobs in `bdd-testing.yml` / `property-testing.yml` / `fuzzing.yml`) | PR #154 | Bounded stochastic PR-fast lane |
| (label-gated routing in `bdd-testing.yml` / `property-testing.yml` / `fuzzing.yml` / `mutation-testing.yml`) | PR #155 | Broad lane routing |
| ci-actuals emitter | PR #148 | LEM feedback loop (writes `target/ci/ci-actuals.json`) |

## See also

- [`policy/ci-lanes.toml`](../../policy/ci-lanes.toml) — machine-readable lane assignments
- [`labels.md`](labels.md) — what each PR label does
- [`risk-packs.md`](risk-packs.md) — what each risk pack auto-routes
- [`branch-protection.md`](branch-protection.md) — required-check migration rule
- [`cost-and-verification-policy.md`](cost-and-verification-policy.md) — why we route
- [`lem-budgeting.md`](lem-budgeting.md) — LEM unit and runner multipliers
- [`test-evidence-lanes.md`](test-evidence-lanes.md) — lane doctrine
