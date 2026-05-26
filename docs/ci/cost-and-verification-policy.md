# Cost and Verification Policy

This document is the doctrine for how shiplog spends CI minutes. It is the
"why" behind the lane assignments in [`test-evidence-lanes.md`](test-evidence-lanes.md)
and the cost model in [`lem-budgeting.md`](lem-budgeting.md).

For Codex-authored CI-efficiency changes, also follow the compatibility
contract in [`codex-ci-efficiency-compatibility.md`](codex-ci-efficiency-compatibility.md).

It is part of the [Rust 1.95 / 0.5.0 quality
rollout](rust-1.95-rollout.md). The TOML ledgers it describes
(`policy/ci-budget.toml`, `policy/ci-lanes.toml`, `policy/ci-risk-packs.toml`,
`policy/ci-exceptions.toml`) are added in PR #146. Skeletons land earlier in
PR #141. Everything below describes the intended steady state.

## The framing

> We are not reducing CI because we want less verification. We are reducing
> wasted CI so we can afford more verification where it matters.

> Rust makes checks fast. ripr makes oracle gaps visible early. LEM budgeting
> makes spend visible. CI routing spends expensive lanes only where they buy
> signal.

> Agentic development makes generation cheap. Verification does not get cheap
> automatically. So CI economics becomes architecture.

This is the lens for every PR in this rollout. When a workflow moves off the
default PR gate, the question is not "are we skipping tests?" — the question
is "have we redirected that LEM budget to a higher-signal lane?"

## Why the budget target is aggressive

This repository targets CI cost per ordinary PR materially below common
high-volume defaults.

That is not because we want less verification. It is because agentic
development needs more verification than traditional software development, and
the cost curve does not work if every PR pays for the whole lab.

Our current development loop can produce roughly **1,000 useful repo
contributions per day** from the Claude 20x Max plan plus a temporarily
boosted ChatGPT Pro plan, with light PR review support from the Minimax token
plan. That is before pointing additional tooling such as Fireworks or other
model capacity at the system.

At that volume, even `$1` per PR becomes material quickly:

```text
1,000 PR-sized CI events/day × $1 = $1,000/day
                              ≈ $30,000/month
                              ≈ $365,000/year
```

At the OpenClaw-shaped comparison point — roughly `$20/commit` on Blacksmith
runners alone, using their published ~$511k Blacksmith spend mapped against
commit volume since February — the economics become obviously incompatible
with high-volume agentic work. That comparison is directional, and if PRs are
squash-merged then commit count is a reasonable proxy for merged PR count.
The lesson is not that OpenClaw verifies too much. The lesson is that serious
agentic development creates serious verification demand.

We believe that demand is real. In fact, we expect agentic development to
require **more** verification than conventional development, not less.

The constraint is that verification must be engineered.

Rust is central to that cost model because it lets us push a large share of
correctness checking into fast, deterministic, local proof:

- compile-time type and ownership checks,
- crate-local unit and oracle tests,
- feature-gated compile checks,
- bounded property tests,
- quick fuzz smoke,
- deterministic receipt/schema tests,
- Clippy and policy linting.

`ripr` is central for the same reason. Full mutation testing is valuable, but
too expensive to run on every ordinary PR. `ripr` gives mutation-testing-shaped
static oracle-gap signal at a much lower cost: it asks whether the behavior
changed by a diff appears exposed to a meaningful test discriminator. That
lets us reserve full runtime mutation for nightly, release, calibration, and
high-risk labeled lanes.

The goal is not cheap CI. The goal is **more proof per CI minute**.

## Operating rule

Every CI lane must justify its place in the stack by answering:

1. What failure mode does it catch?
2. Why does it belong on ordinary PRs, targeted PRs, nightly, main, or
   release?
3. What cheaper signal exists before it?
4. What evidence does it emit?
5. What is its estimated LEM cost?
6. What labels or risk packs should select it?

A lane that cannot answer those questions is not ready to be default PR CI.

## `ripr` and mutation economics

`ripr` is one reason full mutation testing does not need to run on every
ordinary PR.

Full mutation testing executes concrete mutants and proves whether the test
suite rejects them. That evidence is stronger, but it is expensive.

`ripr` sits earlier in the ladder. It does not run mutants and does not
report killed/survived outcomes. It statically asks a mutation-shaped
question: for the behavior changed in this diff, do current tests appear to
expose that behavior to a meaningful discriminator?

That makes `ripr` a PR-time oracle-gap detector. It helps decide when ordinary
tests are probably enough, when a PR needs targeted tests, and when expensive
mutation should be requested through a label, nightly lane, or release gate.
**Mutation calibrates `ripr` over time, not the other way around.**

This pairing — `ripr` advisory on every PR + bounded proptest / quick-fuzz of
touched targets + targeted mutation on label or nightly — is the central
trade. It buys oracle-gap signal on every PR for ~3–6 LEM that would otherwise
require a 90+ LEM full mutation run to obtain.

## Cheap default CI is the wrong framing

The docs should explicitly avoid this bad framing:

```text
cheap CI = fewer tests
```

Use this instead:

```text
cheap default CI = scoped tests, strong oracles, cheap stochastic checks, ripr, and good routing
deep CI         = main / nightly / release / label lanes
```

For shiplog specifically (see [`test-evidence-lanes.md`](test-evidence-lanes.md)
for the full mapping):

| Lane | Default PR? | Why |
| ---- | ----------: | --- |
| `cargo check` / Clippy / unit tests | yes | First-order Rust proof. |
| bounded proptest (selected high-value invariants) | yes, scoped | Cheap invariant pressure on the touched surface. |
| quick fuzz smoke (touched-target only) | yes, scoped | Cheap parser/input robustness signal. |
| BDD smoke (1–2 critical CLI flows) | yes, scoped | Critical CLI/user-flow proof without the full matrix. |
| `ripr` | advisory yes | Cheap oracle-gap detection; reason mutation can defer. |
| coverage | no, label / main / release | Execution-surface measurement. |
| mutation | no, label / nightly / release | Runtime adequacy confirmation. |
| full fuzz / corpus / sanitizers | no, nightly / label | Deeper robustness proof. |
| release / package proof | release / main / label | Publishability. |

## Three rules

### Rule 1 — Every workflow has one intent

Every CI workflow / job must declare a single primary intent in
`policy/ci-lanes.toml`:

```toml
[lane.bdd]
description = "BDD acceptance behavior checks."
intent = "CLI/user-flow behavior proof"
```

Examples of intents in shiplog:

```text
Rust correctness                    cargo build/check/clippy/test/doc-tests
package surface integrity           package boundary + version alignment
publish readiness                   crates.io publish dry-run
dependency policy                   cargo-deny advisories/licenses/bans/sources
CLI/user-flow behavior              BDD scenarios
schema/property invariant           proptest at trust boundaries
parser robustness                   fuzz targets
test-strength evidence              mutation
execution-surface measurement       coverage
oracle-gap detection                ripr
multi-platform release readiness    multi-target build + binary smoke
visibility                          PR plan, policy report
```

Intent is what a reviewer should be able to point at and say "this is the
thing failing." A workflow that mixes intents is a refactor candidate.

### Rule 2 — Duplicates declare themselves

If two workflows produce the same intent, the second declares
`duplicate_of = "lane.X"` in `policy/ci-lanes.toml`:

```toml
[lane.security_standalone]
description = "Standalone cargo-deny workflow (security.yml)."
intent = "dependency policy"
duplicate_of = "lane.ci_deny"
default_pr = false
reason = "ci.yml's deny job is the PR-blocking source of truth; this lane only re-runs on manifest changes / weekly."
```

Without this rule, "security.yml runs the same `cargo deny check` as ci.yml's
deny job" was free to ship without a reviewer noticing. With it, the duplicate
is named, scoped, and reviewable.

shiplog has at least one declared duplicate today (`security.yml` vs
`ci.yml#deny`); PR #146 records it.

### Rule 3 — Expensive defaults need an exception

A lane that costs more than `policy.ci-budget.preferred_default_lem` (~25 LEM
on Linux) and is `default_pr = true` must have a corresponding entry in
`policy/ci-exceptions.toml` with owner and reason:

```toml
[[exception]]
id = "ci-exception-NNNN"
lane = "lane.example_expensive_default"
owner = "release"
reason = "This lane is the cheapest available signal for a required PR-time release invariant."
review_after = "2026-09-09"
```

This is the policy answer to "why is this still on the default PR gate?" It
forces the cost-vs-signal trade-off to be reviewed.

## Routing primitives

PRs route through three mechanisms:

- **Risk packs.** A file-pattern → label mapping in
  `policy/ci-risk-packs.toml`. A PR that touches `apps/shiplog/src/redact/` gets
  the `redaction` label, which routes the redaction-scoped mutation lane.
- **Manual labels.** An author or reviewer can apply `full-ci`,
  `ci-budget-ack`, `ci-budget-override`, `ripr-waive`, etc. to override the
  default routing.
- **Branch / event.** `coverage` runs on push `main` regardless of label.
  Mutation runs Mon weekly regardless of label.

The PR plan (PR #146) emits `target/ci/ci-plan.json` listing which lanes a PR
will run, why (which risk packs / labels matched), and the projected LEM cost.
This makes routing decisions reviewable up front.

## What "skip a lane on PR" means

Skipping a lane on PR is not the same as deleting it. The lane still runs:

- on `main` post-merge (catches what slipped through),
- on schedule (catches drift),
- on opt-in label (when a reviewer wants the signal),
- on a release tag (when the cost is justified by ship readiness).

A PR that wants to ship code touching a high-risk surface should expect the
PR-targeted lane to run, either via the matching risk pack or via an explicit
label.

## Labels (canonical)

These labels are recognized by the PR plan and lane router:

| Label | Effect |
| ----- | ------ |
| `full-ci` | Force every targeted lane on the PR. Use when the change is high-risk or cross-cutting. |
| `ci-budget-ack` | Acknowledge that the PR exceeds `default_limit_lem`. Required for the elevated tier (~75 LEM). |
| `ci-budget-override` | Allow the PR to exceed `elevated_limit_lem`. Required for the hard tier (~125 LEM). |
| `ripr` | Force `ripr` analysis on a PR that would otherwise skip it. |
| `ripr-waive` | Suppress `ripr` advisory output for this PR (use when the finding is known and tracked). |
| `coverage` | Run the coverage lane on PR. |
| `mutation` | Run targeted mutation on PR. |
| `property-tests` | Run the property-test lane on PR. |
| `fuzz` | Run the quick-fuzz lane on PR. |
| `security-audit` | Run the standalone security workflow on PR. |
| `release-check` | Run release preflight (`package-proof.sh`, `publish-dry-run.sh`) on PR. |

See [`lem-budgeting.md`](lem-budgeting.md) for the budget tiers themselves.

## Promotion (advisory → blocking)

A lane is **advisory** until it produces stable signal at known cost. Promotion
to blocking happens deliberately, in its own PR, after:

- the lane has been observed across a representative window of `main` runs,
- a baseline / ledger exists (`policy/...-baseline.toml` or equivalent),
- the cost forecast is within the lane's tier budget,
- a reviewer has signed off the promotion in the relevant policy doc.

shiplog 0.5.0 ships with no hard CI budget enforcement. The PR plan emits
forecasts; nothing blocks on them. Hard enforcement is a follow-up release
decision after PR #148 confirms the model with actuals.

## Anti-patterns

- **Adding a workflow without an intent declaration.** Every workflow joins a
  lane in `policy/ci-lanes.toml` or it does not merge.
- **Adding a workflow that duplicates an existing intent silently.** Either
  collapse to one workflow or declare `duplicate_of` and explain the scope.
- **Using `if: false` as a soft skip.** A skipped-by-policy lane should report
  why through the PR plan, not vanish.
- **Putting `cargo install <thing>` on every PR.** Use prebuilt actions or
  `taiki-e/install-action` and cache; install steps that fetch the registry on
  every PR cost more LEM than the policy they enforce.
- **Splitting a workflow per crate to bypass the budget.** The policy is on
  intent + spend, not on file count. Per-crate workflows for parity testing
  are a refactor candidate.

## See also

- [`lem-budgeting.md`](lem-budgeting.md) — cost unit, runner multipliers,
  budget tiers.
- [`verification-ladder.md`](verification-ladder.md) — what each lane buys,
  ordered by signal-per-LEM.
- [`test-evidence-lanes.md`](test-evidence-lanes.md) — the lane assignments
  themselves.
- [`rust-1.95-rollout.md`](rust-1.95-rollout.md) — the rollout map.
