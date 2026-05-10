# LEM Budgeting

shiplog measures CI cost in **Linux Equivalent Minutes (LEM)**. This document
defines the unit, the runner multipliers, the budget tiers, and the labels
that escalate a PR from one tier to the next.

It is part of the [Rust 1.95 / 0.5.0 quality
rollout](rust-1.95-rollout.md). The TOML ledger
(`policy/ci-budget.toml`) is added in PR 6 and is advisory only in v0.5.0.
Hard enforcement is a follow-up release decision after PR 12 records actuals.

## The unit

```text
1 LEM = 1 wall-clock minute on an Ubuntu (Linux) GitHub-hosted runner.
```

Linux is the cost baseline because:

- Most shiplog CI is Linux.
- Linux runners are the cheapest GitHub-hosted runners (~$0.008/min).
- Other runners are expressed as multipliers of Linux time, which makes
  cross-platform cost comparisons trivial.

Wall-clock time is what matters for both spend (billed minutes) and
reviewer-feel latency.

## Runner multipliers

A wall-clock minute on a non-Linux runner converts to LEM by multiplier:

| Runner | Multiplier | Notes |
| ------ | ---------: | ----- |
| `ubuntu-24.04`, `ubuntu-latest` | **1.0×** | Baseline. |
| `windows-latest` | **2.0×** | GitHub-hosted Windows is ~2× Linux billing. |
| `macos-latest` | **10.0×** | GitHub-hosted macOS is ~10× Linux billing. |
| docker build / `docker buildx` | **6.0×** | Heavy IO + image fetch + layer cost. |
| external AI review (Droid, etc.) | **4.0×** | API + bot orchestration + reasoning latency. |

Cost calculation example. A Windows job that runs for 10 wall-clock minutes
contributes:

```text
10 min × 2.0 = 20 LEM ≈ $0.16
```

A `cargo install cargo-deny --locked` step that runs for 4 wall-clock minutes
on Linux contributes 4 LEM regardless of which workflow it lives in. (Hence
the policy preference for prebuilt-binary install actions.)

## Budget tiers

```toml
[budget]
preferred_default_lem = 25
default_limit_lem     = 35
elevated_limit_lem    = 75
hard_limit_lem        = 125
```

Tier mapping:

| Tier | Cap | Who can run | Required label |
| ---- | --: | ----------- | -------------- |
| Preferred default | 25 LEM | Every PR (target) | none |
| Default | 35 LEM | Every PR (limit) | none |
| Elevated | 75 LEM | High-risk PRs | `ci-budget-ack` |
| Hard | 125 LEM | Cross-cutting / release-prep PRs | `ci-budget-override` |

The preferred default is the design target for the typical PR. The default
limit is what a typical PR is allowed to spend without acknowledgment. Above
that, the PR author or reviewer must apply an escalation label.

In v0.5.0, these caps are advisory: the PR plan emits the projected total;
nothing blocks merge on it.

## What contributes to the budget

The PR plan (PR 6) sums LEM across:

- `default_pr = true` lanes (always run on PR).
- Lanes triggered by labels on the PR.
- Lanes triggered by risk packs that match the PR's changed paths.

It does not include:

- Nightly / scheduled lanes (they run regardless of PR existence).
- Release lanes (they run only on tag push).

## Reducing cost without losing signal

In rough order of payoff:

1. **Cache restore-only on PR; save on `main`.** PRs read caches built by
   `main`; PRs do not write new cache keys. This avoids cache fan-out and
   keeps `main` as the canonical cache producer. (PR 12.)
2. **Skip compile on docs-only PRs.** Path filters that exclude
   `docs/**` + `*.md` from compile-heavy lanes save 5–20 LEM per docs PR. (PR
   12.)
3. **Use `taiki-e/install-action` (or prebuilt cargo-binstall) for tools.**
   `cargo install cargo-deny --locked` from source is a 3–5 LEM step that
   prebuilt binaries reduce to seconds.
4. **Move large parity matrices off the default PR.** Per-crate test sweeps
   that exist for completeness, not for the changed code, are PR-targeted /
   nightly candidates.
5. **Collapse duplicates.** If two workflows compute the same intent (e.g.
   `security.yml` cargo-deny vs `ci.yml#deny`), the duplicate runs only on
   manifest changes / scheduled.

Each of these moves LEM out of the per-PR default into a lane that buys signal
elsewhere.

## Cost vs LEM, worked

A representative shiplog PR today (rough estimates, pre-rollout):

| Lane | Wall-clock (~) | Multiplier | LEM (~) |
| ---- | ------------: | ---------: | ------: |
| `ci.yml#check` Ubuntu | 12 min | 1.0 | 12 |
| `ci.yml#check` Windows | 18 min | 2.0 | 36 |
| `ci.yml#deny` | 4 min | 1.0 | 4 |
| `ci.yml#msrv` | 5 min | 1.0 | 5 |
| `bdd-testing.yml` (4 jobs avg 8 min each) | 32 min | 1.0 | 32 |
| `property-testing.yml` | 20 min | 1.0 | 20 |
| `fuzzing.yml` quick | 10 min | 1.0 | 10 |
| `security.yml` | 4 min | 1.0 | 4 |
| `droid-review.yml` | 5 min wall | 4.0 | 20 |
| **Total** | | | **~143 LEM** |

A representative shiplog PR after the rollout (target):

| Lane | LEM (~) | Notes |
| ---- | ------: | ----- |
| `ci.yml#check` Ubuntu | 12 | unchanged |
| `ci.yml#check` Windows | 36 | covered by exception entry; reviewable |
| `ci.yml#deny` | 4 | unchanged |
| `ci.yml#msrv` | 0–5 | candidate to drop in PR 12 (redundant with `check` while pin == MSRV) |
| `ripr.yml` advisory | 4 | added in PR 11 |
| `pr-plan.yml` | 1 | added in PR 6 |
| BDD critical-flow smoke (1–2 flows) | 4 | bounded (was ~32 LEM as full 4-job matrix on every PR) |
| Bounded proptest (selected invariants, 16–64 cases) | 5 | bounded (was ~20 LEM as 256-case all-crates sweep) |
| Quick-fuzz of touched target (30–90s) | 2 | bounded (was ~10 LEM as 9-target × 30s matrix) |
| `droid-review.yml` | 20 | unchanged |
| **Default-PR total** | **~93** | within `hard_limit_lem`; budget acknowledged |
| Coverage / full BDD / full property sweep / full fuzz matrix / mutation | 0 on default | label / risk-pack / nightly opt-in |
| Risk-pack-routed targeted mutation (when matched) | +30–60 | requires the matching label/pack to run |

The point is not the absolute number — the Default-PR total is roughly the
same as today; the *composition* changes. ~50 LEM of broad parity testing
(full BDD / full property / full quick-fuzz matrix) becomes ~11 LEM of
bounded stochastic + ~4 LEM of `ripr` advisory. Reviewers get oracle-gap
detection on every PR and keep critical-flow smoke for free; the broad
parity surfaces are still proved, just on the lanes that can afford them.

See [`verification-ladder.md`](verification-ladder.md) for the per-lane
signal-vs-cost ladder.

## Stochastic budget rule

Stochastic checks (proptest, fuzz) belong on PR fast **only when bounded**.
The cost driver is unscoped stochastic testing, not stochastic testing itself.
See [Bounded vs broad stochastic](test-evidence-lanes.md#bounded-vs-broad-stochastic)
for the doctrine; the budget caps below are the operational rule:

```text
proptest smoke:  16–64 cases per selected crate/test
quick fuzz:      30–90 seconds total, selected target only
hard cap:        fail the lane if the stochastic step exceeds its time budget
```

LEM budget mapping:

```text
small proptest lane (selected invariants):     usually ≤ 5 LEM, OK on PR fast
selected quick fuzz (touched-target, 30–90s):  usually 1–3 LEM,  OK on PR fast
full proptest sweep / 9-target fuzz matrix:    not default PR
extended fuzz / corpus expansion / sanitizers: not default PR
```

The stochastic lane on PR fast must:

- record a deterministic seed (or log the seed in the failure receipt so the
  failure can be replayed locally),
- name the selected tests / target explicitly (no globbed all-workspace
  fanout),
- enforce its own time cap,
- emit a step-summary receipt listing tests/cases run, seed, and result.

If a stochastic lane on PR fast cannot meet those rules, it is a PR-targeted
or nightly candidate, not a PR fast candidate.

## Labels (escalation and routing)

| Label | Effect |
| ----- | ------ |
| `ci-budget-ack` | PR may run elevated tier (≤75 LEM). |
| `ci-budget-override` | PR may run hard tier (≤125 LEM). |
| `full-ci` | Force every targeted lane (implies `ci-budget-override`). |

See [`cost-and-verification-policy.md`](cost-and-verification-policy.md) for
the full label table including routing labels (`ripr`, `ripr-waive`,
`coverage`, `mutation`, etc.).

## Anti-patterns

- **Pinning a Windows job to a never-cached `cargo install <thing>` step.**
  The install dominates the LEM and the multiplier is 2×.
- **Running a 5-minute Linux step per workspace crate as 22 separate jobs to
  parallelize "for free."** Job-startup overhead and cache-restore cost dominate
  per-job; consolidating into 2–3 grouped jobs usually reduces wall-clock and
  LEM.
- **Re-running the same `cargo deny check` in two workflows.** Charge once;
  declare the duplicate.
- **Using a macOS runner where Linux would suffice.** 10× multiplier; only
  worth it for genuine macOS-specific parity.

## See also

- [`cost-and-verification-policy.md`](cost-and-verification-policy.md) — the
  doctrine.
- [`verification-ladder.md`](verification-ladder.md) — which lanes buy how
  much signal per LEM.
- [`test-evidence-lanes.md`](test-evidence-lanes.md) — lane assignments.
- [`rust-1.95-rollout.md`](rust-1.95-rollout.md) — the rollout map.
