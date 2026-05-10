# Verification Ladder

This document orders shiplog's CI lanes by **signal per LEM** — the question
"what evidence do I get for this minute of CI?" — and explains the claim
boundary at each step.

It is the third leg of the [Rust 1.95 / 0.5.0 quality
rollout](rust-1.95-rollout.md) CI doctrine, alongside
[`cost-and-verification-policy.md`](cost-and-verification-policy.md) (why we
route) and [`lem-budgeting.md`](lem-budgeting.md) (how we measure cost).

## The ladder

Ordered roughly best-signal-per-LEM first, given a typical Rust diff:

| Rank | Lane | Cost (~LEM) | Buys | Does not buy |
| ---: | ---- | ----------: | ---- | ------------ |
| 1 | `cargo check --workspace` | 8–12 | Type-correct compile across the workspace | Lint correctness, runtime correctness |
| 2 | `cargo fmt --check` | 1 | Formatting drift | Anything else |
| 3 | `cargo clippy -- -D warnings` | 8–12 | Lint correctness at policy level | Runtime correctness, oracle adequacy |
| 4 | `cargo test` (unit + integration) | 10–18 | Local test oracles pass | Property-space corners, behavior across CLI flows |
| 5 | `cargo test --doc` | 2–4 | Doc examples compile and pass | Anything outside the example |
| 6 | `cargo deny check` | 2–4 | Dependency policy (advisories, licenses, bans, sources) | Code correctness |
| 7 | `cargo doc --no-deps` (RUSTDOCFLAGS=`-D warnings`) | 4–8 | Public docs compile cleanly | Doc accuracy |
| 8 | `package-boundary-audit` + `package-version-audit` | 2 | Workspace publishability invariant | Build correctness |
| 9 | `cargo publish --dry-run` (canary) | 4–6 | crates.io readiness for one anchor crate | Full publish ordering |
| 10a | bounded proptest smoke (PR fast, selected high-value invariants, 16–64 cases) | 3–5 | Property invariants hold for the touched trust surface | Property pressure for unselected crates / corner inputs |
| 10b | quick-fuzz smoke (PR fast, touched-target only, 30–90s) | 1–3 | Parser/serde robustness signal on the touched target | Other parser targets, deep corners |
| 10c | BDD critical-flow smoke (PR fast, 1–2 anchor flows) | 3–5 | Critical CLI/user-flow behavior unbroken | Affected feature files, full BDD matrix |
| 10d | `ripr` (planned, advisory PR fast) | 3–6 | Reachable mutants with weak oracles in the diff | Mutation adequacy itself (rung 17 calibrates) |
| 11a | risk-pack proptest (PR-targeted, elevated case count for the matched scope) | 8–15 | Property invariants for the risk-pack surface at higher confidence | Whole-workspace property sweep |
| 11b | selected fuzz target 30–90s (PR-targeted, label or `parsers` risk pack) | 2–5 | Parser robustness for the targeted parser at PR scope | Other targets, longer fuzz, sanitizers |
| 11c | affected-feature BDD (PR-targeted, by `cli/product` risk pack) | 8–18 | CLI behavior across the feature files the diff touches | Full BDD suite |
| 11d | targeted mutation (PR-targeted, label / risk pack / severe `ripr`) | 30–60 | Mutation-clean for the touched crate | Whole-repo adequacy |
| 12a | full BDD suite (nightly) | 32 | All 4 BDD job categories at full scope | n/a |
| 12b | full property sweep (nightly) | 25–35 | Proptest at 256 cases across all 13 testable crates | Out-of-band corner inputs |
| 13 | full coverage (push `main` / label) | 30–45 | Execution-surface coverage measurement | Test adequacy itself (use mutation) |
| 14 | quick fuzz × 9 targets (nightly / `fuzz` label) | 8–12 | Parser robustness on small inputs across all 9 targets | Deep parser corners |
| 15 | extended fuzz × 9 targets (nightly cron) | 540 | Parser robustness with budget | n/a |
| 16 | full mutation across 22 crates (weekly cron) | 90–120 | Mutation-clean baseline for the workspace | Coverage breadth (use coverage); calibrates `ripr` |
| 17 | release multi-platform build + smoke (tag) | 60–90 | Tagged commit installs and runs across declared platforms | Latent bugs not in smoke surface |
| 18 | downloaded-artifact + binary smoke (tag) | 10–15 | Released artifact actually launches | Behavior beyond `--help` / `--version` |

(Costs are approximate steady-state estimates for shiplog as of the v0.5.0
cut. The PR plan in PR #146 will record actual observed costs and update the
table.)

## Reading the ladder

The ladder is not a "do them all in order" recipe. It is a way to answer two
questions during PR review:

1. **"Was the right evidence run for this change?"** A docs-only PR needs
   rungs 1–2; a parser change needs rungs 1–4 + 10b + 11b; a redaction-trust
   PR needs 1–4 + 10a + 10d + 11d; a release tag needs 1–9 + 17–18.
2. **"What does the default PR fast lane buy?"** Rungs 1–9 + 10a–10d. That
   gives type/lint correctness, unit oracles, dependency policy, package
   integrity, *plus* bounded stochastic pressure on the touched surface and
   `ripr` advisory. The rollout's central move is adding rung 10d (`ripr`)
   to the default PR gate; the bounded forms of 10a–10c replace today's
   broad-on-every-PR variants of 12a, 12b, and 14.

## Where the rollout shifts spend

The rollout's net effect on PR composition (see
[`lem-budgeting.md`](lem-budgeting.md) for the worked example):

```text
Today's PR default: 1–9 + 12a (full BDD) + 12b (full property) + 14 (full fuzz matrix)
Target PR default:  1–9 + 10a (bounded proptest) + 10b (quick fuzz of touched target)
                          + 10c (BDD critical-flow smoke) + 10d (ripr advisory)

Broader scopes route by label / risk pack / nightly:
  12a (full BDD)         → nightly + risk pack `cli/product` on PR-targeted (rung 11c)
  12b (full property)    → nightly + risk pack scope on PR-targeted (rung 11a)
  14 (full fuzz matrix)  → nightly + `fuzz` label / `parsers` risk pack on PR-targeted (rung 11b)
  11d (targeted mutation) → label / risk pack / severe ripr finding (newly added)
  13 (coverage)          → push main / label (already correct)
  16 (full mutation)     → weekly (already correct; calibrates ripr)
```

This trades ~50 LEM of always-on broad parity testing for ~11 LEM of bounded
stochastic + 4 LEM of `ripr` advisory. The total per-PR cost is similar; the
*composition* changes. Reviewers get oracle-gap detection on every PR and
keep critical-flow smoke for free; the broad parity surfaces still get
proved, on the lanes that can afford them.

This pairing — `ripr` advisory + bounded proptest/quick-fuzz of touched
targets + targeted mutation on label/nightly — is the central trade. It is
why full mutation can defer off the default PR gate without losing
oracle-adequacy signal at PR time.

## Claim boundaries (per rung, condensed)

A green rung does **not** prove what is in the "Does not buy" column. The
ladder makes that explicit so reviewers do not over-claim:

- A green `cargo test` does not prove property invariants hold for unseen
  inputs.
- A green bounded proptest (rung 10a) does not prove invariants for the
  unselected crates or for case counts beyond the bounded budget — the full
  sweep (12b, nightly) is the broader claim.
- A green BDD critical-flow smoke (rung 10c) does not prove the affected
  feature files pass at full scope — that is the PR-targeted (11c) or
  nightly (12a) claim.
- A green quick-fuzz of the touched target (rung 10b) does not prove other
  parser targets are robust — the full matrix (14, nightly) is the broader
  claim.
- A green `ripr` (rung 10d) does not prove mutation adequacy itself — full
  mutation (16, weekly) calibrates `ripr` and is the stronger claim.
- A green coverage run does not prove tests have strong oracles (use
  mutation).
- A clean mutation tier does not prove source adapters are complete against
  provider APIs.
- A clean release smoke does not prove the binary handles inputs beyond
  `--help` / `--version`.

The release-readiness doc (`docs/release/0.5.0-readiness.md`, PR #157) records
the explicit known-non-blockers list.

## Promotion rules (advisory → required)

A rung promotes from advisory to required when:

- it has been observed across a representative window of `main` runs,
- its cost stays inside the relevant budget tier,
- a baseline / ledger exists,
- a reviewer signs off the promotion in the matching policy doc.

shiplog 0.5.0 promotes nothing automatically. The promotion paths are listed
in [`test-evidence-lanes.md`](test-evidence-lanes.md#promotion-paths).

## See also

- [`cost-and-verification-policy.md`](cost-and-verification-policy.md) — why
  we route, intent and duplication rules, label model.
- [`lem-budgeting.md`](lem-budgeting.md) — cost unit, runner multipliers,
  budget tiers.
- [`test-evidence-lanes.md`](test-evidence-lanes.md) — lane assignments and
  per-lane claim boundaries.
- [`coverage.md`](coverage.md) — Codecov execution-surface coverage policy.
- [`mutation.md`](mutation.md) — mutation testing baselines and claim
  boundary.
- [`rust-1.95-rollout.md`](rust-1.95-rollout.md) — the rollout map.
