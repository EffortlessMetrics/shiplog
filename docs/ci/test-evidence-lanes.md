# Test Evidence Lanes

This document describes how shiplog will route test and quality evidence
across four lanes:

```text
PR fast       → every PR, fast, blocking gate
PR-targeted   → opt-in via label or risk pack, blocking when present
Nightly       → scheduled, advisory unless promoted, drives debt visibility
Release       → tag-triggered, blocking, ship readiness
```

It is part of the [Rust 1.95 / 0.5.0 quality
rollout](rust-1.95-rollout.md).

**Important:** the lane assignments below are **tentative and advisory** in
v0.5.0. They describe the intended steady state. Encoding them as TOML
happens in PR #146 (`policy/ci-lanes.toml`, `policy/ci-budget.toml`,
`policy/ci-risk-packs.toml`). Phase 6 splits the wiring across three PRs:
PR #153 adds the `ripr.yml` advisory lane, PR #154 carves out the bounded
stochastic PR-fast lane, and PR #155 routes broad BDD/property/fuzz/mutation
to nightly + label + risk pack. Tightening cache economics happens in
PR #147; CI actuals (the LEM feedback loop) land in PR #148. Hard enforcement
of lane budgets is a follow-up release decision, not v0.5.0.

This document changes how the rollout PRs are reviewed; it does not by itself
move any workflow.

## Why lanes

Two failure modes drive the design:

1. **Putting expensive evidence on every PR.** Full mutation runs take
   ~120 minutes; running it on every PR makes review feel slow even when the
   change is trivial, so reviewers learn to ignore it.
2. **Hiding evidence behind "advisory" without a promotion path.** If
   advisory evidence is never converted to blocking, it stops being read.

Lanes give every kind of evidence a home that matches its cost and its claim
boundary. The economics are tracked in [Linux Equivalent
Minutes](lem-budgeting.md); the doctrine for routing is in
[`cost-and-verification-policy.md`](cost-and-verification-policy.md).

## PR fast lane

Runs on every PR. Required for merge. Should fit inside the
[`preferred_default_lem` budget](lem-budgeting.md) of ~25 LEM, leaving headroom
for the elevated label-driven lanes.

The PR fast lane mixes deterministic checks (compile, format, lint, unit tests)
with **bounded stochastic checks** (small proptest sweeps, tiny quick-fuzz of
touched targets, one or two BDD smoke flows). Stochastic on PR fast is good
when bounded — see [Bounded vs broad stochastic](#bounded-vs-broad-stochastic)
below.

| Workflow | What it claims | Posture today | Tentative posture |
| -------- | -------------- | ------------- | ----------------- |
| `ci.yml` (`check`) | Workspace builds, formats, lints (`-D warnings`), tests, doc-tests, doc build, release build, canary publish dry-run | PR + push (Ubuntu + Windows matrix) | unchanged |
| `ci.yml` (`deny`) | `cargo deny check` — bans, advisories, licenses, sources | PR + push | unchanged |
| `ci.yml` (`msrv`) | Workspace compiles on declared MSRV | PR + push | redundant with `check` while toolchain pin == MSRV; consider removing in PR #147 |
| **(planned, PR #153)** `ripr.yml` | Reachable-mutant exposure analysis | not present | added as advisory |
| **(planned, PR #154)** bounded proptest smoke | Selected high-value invariants (redaction alias stability, bundle manifest paths, coverage/gap accounting, intake.report.json shape, share.manifest checksum/path rules, source.failures.json classification, period/window resolution, journal/manual event ID stability) at 16–64 cases per selected test | full proptest sweep across all crates today | bounded smoke on PR fast; full sweep moves to nightly + risk-pack PR-targeted |
| **(planned, PR #154)** quick-fuzz of touched targets | Compile + 30–90s fuzz of the parser/serde target the PR touches (or one canonical anchor if PR is tooling) | quick fuzz × 9 targets on every PR (~15 min) today | touched-target smoke on PR fast; full 9-target matrix moves to nightly + label |
| **(planned, PR #154)** BDD critical-flow smoke | One or two CLI critical flows (e.g. `intake` happy path + `share verify public --strict`) | full 4-job BDD matrix on every PR today | smoke on PR fast; affected feature files routed by risk pack PR-targeted; full BDD suite moves to nightly |

PR fast lane claim boundary:

- A green PR fast lane proves: the workspace compiles cleanly on the declared
  toolchain, lints pass, unit + doc + canary publish work, the dependency tree
  is policy-clean, **and the bounded stochastic checks did not catch a
  regression in the touched surface**.
- It does not prove: full mutation adequacy, full Codecov coverage, BDD or
  property suite stability across all crates, multi-target release builds, or
  extended fuzz. Those live in higher-cost lanes.

## Bounded vs broad stochastic

Property tests, fuzzing, and BDD scenarios are not inherently expensive. The
cost driver is **unscoped** stochastic testing — running every test, every
target, at full sample size, on every PR.

Stochastic checks are good PR fast candidates when they are:

- changed-surface scoped (only tests reachable from the diff, or one canonical
  anchor for tooling diffs),
- deterministic (fixed seed, or seed logged so a failure can be replayed),
- small (proptest 16–64 cases per selected test; quick fuzz 30–90s total per
  selected target),
- self-contained (no network, no large corpus download, no model fetch),
- not part of a sanitizer matrix or all-workspace fanout.

Stochastic checks belong in higher-cost lanes when they are:

- broad (all proptests across the workspace, all fuzz targets at once),
- long-running (extended fuzz, sanitizer fuzz, corpus expansion),
- combined with another expensive lane (coverage + proptest, mutation +
  proptest).

The rollout splits this carve-out across two PRs: PR #154 adds the bounded
stochastic PR-fast lane, and PR #155 routes the broad forms to PR-targeted
(label / risk pack) and nightly. Today's shiplog runs the broad forms on
every PR; the rollout reshapes them rather than removing them.

## PR-targeted lane

Opt-in via PR label or via risk-pack auto-routing (PR #146 emits the
assignment in `policy/ci-risk-packs.toml`; PR #153 wires `ripr` advisory;
PR #155 wires broad lane routing). This is where the **broader scopes** of
the stochastic lanes live — risk-pack-scoped proptest sweeps, selected fuzz
targets at 30–90s, the BDD feature files relevant to the change, and
label-gated coverage / mutation. Blocking when present; otherwise the workflow
reports skipped-by-policy rather than just "not run".

| Workflow | Label / trigger | Posture today | Tentative posture |
| -------- | --------------- | ------------- | ----------------- |
| `bdd-testing.yml` (affected feature files) | label `bdd` or `full-ci`; risk pack `cli/product` | runs every-test on every PR (4 jobs, ≤30 min each) | scoped to affected feature files on PR-targeted; full BDD suite moves to nightly |
| `coverage.yml` | label `coverage` or `full-ci`; risk pack `coverage-impacting` | label-gated already | unchanged on PR; codify in TOML (PR #146) |
| `property-testing.yml` (risk-pack scoped) | label `property-tests` or `full-ci`; risk pack `schema/ids`, `redaction`, `bundle`, or `source-adapter` | runs all crates / 256 cases on every PR | risk-pack-scoped on PR-targeted with elevated case count; full sweep moves to nightly |
| `security.yml` | label `security-audit`; manifest changes (Cargo.toml / Cargo.lock) | runs on every PR (duplicates `ci.yml` `deny`) | route to manifest changes / security label / main / scheduled; remove duplicate from default PR |
| `fuzzing.yml` (selected target, 30–90s) | label `fuzz`; risk pack `parser` (parsers / serde) | runs all 9 targets on every PR (~15 min) | risk-pack-selected target at 30–90s on PR-targeted; full 9-target matrix moves to nightly |
| **(planned, PR #155)** mutation targeted (narrow) | label `mutation` or `full-ci`; risk pack matches `redaction`, `bundle`, `schema/ids`, `coverage`, or `source-adapter`; severe `ripr` finding | not present | added |

Risk packs (PR #146 inventory; PR #155 routing) auto-apply labels based on
which files a PR touches:

```text
redaction/privacy   → crates/shiplog-redact/, crates/shiplog-bundle/, profile/share code
bundle/integrity    → crates/shiplog-bundle/, share manifest writers/verifiers
schema/ids          → crates/shiplog-schema/, crates/shiplog-ids/, contracts/schemas/
source adapters     → apps/shiplog/src/ingest/
CLI/product         → apps/shiplog/
release/package     → scripts/, .github/workflows/release.yml, package metadata
policy/workflows    → policy/, .github/workflows/, xtask/
parsers             → fuzz/fuzz_targets/, *_parse.rs, serde derives at trust boundaries
docs-only           → docs/**, README.md, CHANGELOG.md
```

shiplog's bounded PR-fast stochastic candidates (the ones the PR fast lane
actually runs on every PR; the broader scopes here run on label/risk pack):

```text
proptest smoke:
  redaction alias stability
  bundle manifest path normalization
  coverage/gap accounting
  intake.report.json schema shape
  share.manifest checksum/path rules
  source.failures.json classification
  period/window resolution
  journal/manual event ID stability

quick-fuzz smoke (touched-target only):
  redaction input strings
  intake report JSON parser
  config TOML parser
  manual event YAML parser
  provider payload normalization
  share manifest verifier

BDD critical-flow smoke:
  intake happy path
  share verify public --strict
```

PR-targeted claim boundary:

- A green PR-targeted lane proves the named surface holds at the labeled
  level (e.g. mutation-clean for the touched crate, Codecov above the
  configured patch threshold, BDD scenarios pass for CLI/product diffs,
  proptest passes at elevated case count for the risk-pack scope).
- It does not prove the whole repo holds at that level. That is the nightly
  lane's job.

## Nightly / scheduled lane

Runs on cron. Advisory by default; failures produce visible receipts but do
not block PRs. Failures here are the early warning that policy ledgers may
need a refresh.

| Workflow | Schedule | What it claims | Posture today | Tentative posture |
| -------- | -------- | -------------- | ------------- | ----------------- |
| `coverage.yml` | push `main` | Codecov coverage on `main` post-merge (Codecov flag `rust-core`) | unchanged | unchanged |
| `bdd-testing.yml` | nightly cron (PR #155 adds) | BDD across all 4 jobs at full size | not scheduled | add nightly |
| `property-testing.yml` | nightly cron (PR #155 adds) | Property suites across all 13 crates at full sample size | not scheduled | add nightly |
| `fuzzing.yml` extended | daily 00:00 UTC | 60-minute fuzz per target × 9 targets | unchanged | unchanged |
| `mutation-testing.yml` | Mon 08:00 UTC | Mutation testing for all 22 workspace crates, three tiers, 120-minute cap, advisory | unchanged | unchanged |
| `droid-security-scan.yml` | Mon 08:00 UTC | Bot-driven 7-day security scan with medium severity threshold | unchanged | unchanged |

Nightly claim boundary:

- A clean nightly lane proves the repo's slower-cost evidence is stable.
- A failing nightly lane proves a regression has been observed; it is
  reviewer responsibility to file a follow-up PR (or accept the regression
  with a policy-ledger entry that explains why).
- Nightly evidence does not gate PRs unless explicitly promoted via
  PR-targeted routing or a risk pack.

## Release lane

Runs on tag push (`v*`). Blocking; a release fails to ship if any step fails.

| Workflow / step | What it claims | Posture today | Tentative posture |
| --------------- | -------------- | ------------- | ----------------- |
| `release.yml` `release-preflight` | `scripts/package-proof.sh` (package boundary + version alignment + cargo-deny) and `scripts/publish-dry-run.sh` (crates.io publish readiness, ordered) | unchanged | toolchain bumps to 1.95.0 in PR #145 |
| `release.yml` `build-binary` | Builds `shiplog` for `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc` | unchanged | toolchain bumps to 1.95.0 in PR #145 |
| `release.yml` `create-release` | Creates draft GitHub release with auto-generated release notes | unchanged | unchanged |
| `release.yml` `upload-assets` | Uploads platform binaries + `SHA256SUMS.txt` to the release | unchanged | unchanged |
| `release.yml` `release-validation` | Smoke tests downloaded Linux artifact + freshly-built release binary across the product help surface | unchanged | toolchain bumps to 1.95.0 in PR #145 |
| `release.yml` `release-test` | Runs `cargo test --release --workspace --exclude shiplog-testkit` | unchanged | toolchain bumps to 1.95.0 in PR #145 |
| **(planned, PR #155)** release-readiness mutation snapshot | Single-tier mutation snapshot for trust surfaces at tag time | not present | added |

Release lane claim boundary:

- A green release lane proves the tagged commit installs cleanly across the
  declared platforms, the package surface is consistent, and the publish-ready
  tree passes the dry-run.
- It does not prove the absence of latent bugs in surfaces not covered by
  smoke tests. The release-readiness doc (`docs/release/0.5.0-readiness.md`,
  PR #157) carries the explicit known-non-blockers list.

## How `ripr` fits (planned)

`ripr` is a fast oracle-exposure filter for PRs. It runs in the PR fast lane
as advisory only and does not block merges. Its job:

- Flag mutants that are reachable from the PR change, so the reviewer knows
  which targeted-mutation runs would be productive.
- Surface oracle weakness on shipping code without paying the full mutation
  bill on every PR.

### `ripr` and mutation economics

`ripr` is one reason full mutation testing does not need to run on every
ordinary PR.

Full mutation testing is still valuable: it executes concrete mutants and
proves whether the test suite rejects them. That evidence is stronger, but it
is expensive.

`ripr` sits earlier in the ladder. It does not run mutants and does not
report killed/survived outcomes. It statically asks a mutation-shaped
question: for the behavior changed in this diff, do current tests appear to
expose that behavior to a meaningful discriminator?

That makes `ripr` a PR-time oracle-gap detector. It helps decide when ordinary
tests are probably enough, when a PR needs targeted tests, and when expensive
mutation should be requested through a label, nightly lane, or release gate.
Mutation calibrates `ripr` over time, not the other way around.

Severe `ripr` findings are one of the auto-trigger conditions for the
PR-targeted mutation lane. The `ripr-waive` label suppresses the advisory
output for a PR (used when the finding is known and tracked). The `ripr`
label forces the analysis even on PRs that would otherwise skip it (e.g.
docs-only PRs that touch a build script).

## What "advisory" means here

For v0.5.0, advisory means:

- The check runs.
- Its result is recorded as a PR receipt or step summary.
- It does not block merge.
- It can be promoted to blocking by a follow-up release once actuals exist.

Advisory is not "ignored." Reviewers are expected to read the output and use
it. If a PR keeps producing the same advisory finding without action, the
reviewer is responsible for either a fix PR or a policy-ledger entry that
records the deferral.

## Promotion paths

Evidence does not stay advisory forever. The expected promotion paths:

- **Coverage**: project ratchet stays informational while baseline matures
  (`docs/ci/coverage.md`); per-flag patch coverage becomes blocking in a
  follow-up release once the baseline is stable.
- **Mutation**: per-tier baselines become blocking on a per-tier basis once
  scheduled runs show stable timings and stable survivor counts
  (`docs/ci/mutation.md`).
- **`ripr`**: the suppressions ledger (`policy/ripr-suppressions.toml`,
  skeleton in PR #141, wired in PR #153) exists so promotion to severity-gated
  blocking can happen without flag day.
- **File policy**: `--mode advisory` in PR #149; promotion to
  `--mode blocking-allowlist` in a follow-up release after ledger cleanup.
- **Lane budgets**: LEM caps stay advisory in v0.5.0 (PR #146 only emits
  forecasts). Hard enforcement is a follow-up release decision once actuals
  (PR #148) confirm the model.

## See also

- [`coverage.md`](coverage.md) — Codecov execution-surface coverage policy
  and current baseline.
- [`mutation.md`](mutation.md) — mutation testing baselines and claim
  boundary.
- [`rust-1.95-rollout.md`](rust-1.95-rollout.md) — the rollout map and the
  PRs that introduce ripr / mutation routing / lane policy.
- [`cost-and-verification-policy.md`](cost-and-verification-policy.md) — why
  we route, intent vs duplication.
- [`lem-budgeting.md`](lem-budgeting.md) — Linux Equivalent Minutes cost unit
  and runner multipliers.
- [`verification-ladder.md`](verification-ladder.md) — signal-vs-spend ladder
  per lane.
- [`../FILE_POLICY.md`](../FILE_POLICY.md) — non-Rust file allowlists.
- [`../CLIPPY_POLICY.md`](../CLIPPY_POLICY.md) — Clippy lint ledger model.
- [`../NO_PANIC_POLICY.md`](../NO_PANIC_POLICY.md) — exact-identity no-panic
  baseline.
