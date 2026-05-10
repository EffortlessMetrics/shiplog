# Rust 1.95 / shiplog 0.5.0 Quality Rollout

This is the local map for the Rust 1.95 toolchain bump and the policy /
CI-economics control plane that ships with shiplog 0.5.0.

It exists so the implementation work that follows lands as a sequence of small,
reviewable PRs instead of a single opaque diff that broadens scope from one
lint into toolchain, release, and policy in one pass.

## Release framing

```text
v0.5.0: Operational Review Rescue
       + Rust 1.95 quality floor
       + policy/CI economics foundation
```

Three things travel together in this release:

1. The operational hardening lane already merged after v0.4.0 (PRs #125–#139:
   release install smoke, no-network demo, intake report v1 + validate +
   summarize, repair classifiers, doctor `--repair-plan`, source failure
   receipts, share manifests + verification, stable fixup IDs, machine-readable
   intake actions, period inspection + comparison, agent-pack export). This is
   the bridge from the review-rescue CLI into a dependable local evidence
   control plane.
2. An MSRV bump from Rust 1.92 → 1.95. This was meant to land in v0.4.0 but
   slipped, so it ships now. The bump narrows the supported consumer set and
   so is semver-significant — minor release, not patch.
3. A first cut of policy ledgers, a thin Rust-native xtask runner, an
   advisory LEM-budgeted CI lane plan, and `ripr` advisory routing. None of
   this enforces hard limits in v0.5.0; it makes spend visible and reviewable
   so a later release can promote what works.

## Why bother

The verification-economics framing for this rollout:

> We are not reducing CI because we want less verification. We are reducing
> wasted CI so we can afford more verification where it matters.

> Rust makes checks fast. ripr makes oracle gaps visible early. LEM budgeting
> makes spend visible. CI routing spends expensive lanes only where they buy
> signal.

See [`cost-and-verification-policy.md`](cost-and-verification-policy.md) for
the doctrine, [`lem-budgeting.md`](lem-budgeting.md) for the cost model, and
[`verification-ladder.md`](verification-ladder.md) for the signal-vs-spend
ladder.

## Current vs target

| Layer | Current | Target | Status |
| ----- | ------: | -----: | ------ |
| Edition | 2024 | 2024 | done |
| MSRV | 1.92 | 1.95 | planned (PR 5) |
| Toolchain pin (`rust-toolchain.toml`) | 1.92 | 1.95.0 | planned (PR 5) |
| `release.yml` toolchain pin | 1.92 (preflight, build, validation, integration) | 1.95.0 | planned (PR 5; release moves with CI) |
| Workspace version | 0.4.0 | 0.5.0 | planned (PR 14) |
| Workspace lints (Rust) | `unsafe_code = deny`, `missing_debug_implementations = allow` | 1.95 lint floor (`unsafe_op_in_unsafe_fn`, `unused_must_use`, `unexpected_cfgs`, `const_item_interior_mutations`, `function_casts_as_integer`, `unused_visibilities`) | planned (PR 10) |
| Workspace lints (Clippy) | `enum_glob_use`, `flat_map_option` warn; `needless_pass_by_value`, `cloned_instead_of_copied` allow as debt | 1.94/1.95 ratchets activated; broad allows moved to receipted debt ledger | planned (PR 8, PR 10) |
| `clippy.toml` | absent | present, MSRV-aware, no test carveouts | planned (PR 5) |
| `xtask/` | absent | thin Rust-native policy runner | planned (PR 3) |
| `policy/` ledger files | absent | TOML skeletons present (PR 2), enforced by xtask (PR 7–9) | planned (PR 2 schema, PR 7+ enforcement) |
| `ci/lanes` policy | implicit | `policy/ci-budget.toml`, `policy/ci-lanes.toml`, `policy/ci-risk-packs.toml`, `pr-plan.yml` advisory | planned (PR 6) |
| No-panic baseline | absent | exact-identity (path + family + selector_kind + selector_callee + snippet + count), no-new-debt mode | planned (PR 9) |
| Non-Rust file policy | absent | non-Rust + companion ledgers (generated, executable, dependency, workflow, process, network) | planned (PR 7) |
| `ripr` | absent | advisory PR-time exposure filter, label-promotable | planned (PR 11) |
| Mutation routing | weekly cron + dispatch (off PR) | targeted PR mutation by label / risk pack + nightly + release readiness | planned (PR 11; reconciles existing `mutation-testing.yml`) |
| CI cache economics | mixed save/restore on every PR | PR caches restore-only, `main` saves canonical, docs-only does not compile | planned (PR 12) |
| Release proof | shell scripts (`package-proof.sh`, `publish-dry-run.sh`, `package-boundary-audit.sh`, `package-version-audit.sh`, `verify-release.sh`, `release-install-smoke.{sh,ps1}`) | shell scripts wrap `xtask` parity over time; never silently dropped | partial (PR 3 wraps; release-readiness in PR 14) |

## Existing CI / evidence inventory and tentative lanes

shiplog already runs 11 workflows. The gap this rollout closes is **policy
ledgers, xtask, ripr, and explicit lane economics** — not test infrastructure.

The lane assignments in this section are **tentative and advisory**. They are
this PR's read of where each workflow should sit; they are not enforced policy.
Enforcement comes after the lane whitelist (PR 6), the PR plan (PR 6), and
actuals (PR 12) exist.

| Workflow | Trigger today | Tentative lane | Default posture |
| -------- | ------------- | -------------- | --------------- |
| `ci.yml` (`check`, `deny`, `msrv`) | push `main` + PR | PR fast / required | blocking |
| `release.yml` | tag `v*` + dispatch | release | tag/manual |
| `bdd-testing.yml` | push + PR (4 jobs, ≤30 min each) | PR fast (bounded smoke: 1–2 critical CLI flows) + PR-targeted (affected feature files via risk pack) + nightly (full BDD) | bounded smoke on PR, broader on label/nightly |
| `coverage.yml` | push `main`, PR with label `coverage`/`full-ci`, dispatch | main / label / release | not default PR (already correct) |
| `fuzzing.yml` quick | push + PR (15 min, 9 targets) | PR fast (touched-target only at 30–90s) + PR-targeted (selected target via risk pack `parsers`) + nightly (full 9-target matrix) | bounded smoke on PR, full matrix on label/nightly |
| `fuzzing.yml` extended | nightly cron + dispatch (90 min × 9 targets) | nightly | scheduled |
| `mutation-testing.yml` | PR skipped, Mon weekly cron, dispatch | nightly / label (narrow targeted) / release | not default PR (already correct) |
| `property-testing.yml` | push + PR (256 cases, all crates) | PR fast (bounded smoke: high-value invariants at 16–64 cases per selected test) + PR-targeted (risk-pack-scoped at elevated case count) + nightly (full sweep) | bounded smoke on PR, broader on label/nightly |
| `security.yml` (cargo-deny) | push + PR + weekly cron + dispatch | manifest / security label / main / scheduled | targeted (duplicate of `ci.yml#deny` on default PR) |
| `droid-review.yml` | PR open / sync / ready | advisory automation | non-blocking unless promoted |
| `droid.yml` | `@droid` mentions | advisory automation | out-of-band |
| `droid-security-scan.yml` | Mon weekly + dispatch | advisory automation | scheduled |

What changes from "trigger today" to "tentative lane":

- **BDD, property, quick fuzz** currently run their broad form on every PR.
  The tentative routing keeps **bounded** versions on PR fast (changed-surface
  scoped, deterministic, small case/time caps) and routes the **broad**
  versions to PR-targeted (label / risk pack) and nightly. The cost driver is
  unscoped stochastic testing, not stochastic testing itself — see [Bounded vs
  broad stochastic](test-evidence-lanes.md#bounded-vs-broad-stochastic) in the
  lane doctrine. This frees LEM budget for `ripr` advisory + targeted mutation
  on every PR while preserving fast stochastic signal where it is cheap.
- **`security.yml` cargo-deny** is duplicated by `ci.yml`'s own `deny` job. The
  tentative routing keeps `cargo-deny` blocking on PRs (via `ci.yml#deny`)
  but routes the standalone `security.yml` to manifest/security label/main/
  scheduled rather than re-running on every PR.
- **`coverage` and `mutation-testing`** are already correctly off the default
  PR gate. PR 11 adds explicit label-gated PR-targeted entry points; the
  weekly mutation lane is unchanged. `ripr` (PR 11) becomes the cheap PR-time
  oracle-gap detector that justifies keeping full mutation off the default PR
  gate — see [`ripr` and mutation
  economics](test-evidence-lanes.md#ripr-and-mutation-economics).

These are read-only assignments in this PR. PR 6 encodes them as TOML in
`policy/ci-lanes.toml` (still advisory). PR 11 wires `ripr.yml` and the
targeted mutation routing. PR 12 measures actuals. Hard enforcement is a
follow-up release decision, not v0.5.0.

See [`test-evidence-lanes.md`](test-evidence-lanes.md) for the full lane
doctrine and per-lane claim boundaries.

## Rust 1.95 surfaces that pay for the bump in shiplog

| Rust 1.95 item | shiplog use |
| -------------- | ----------- |
| `Vec::push_mut` / `insert_mut` | Packet builders, coverage sections, workstream summaries, bundle manifests, intake-report `actions[]`, agent-pack exports. |
| `if let` guards | Source classification (skipped/partial), redaction profile branching, config migration, provider response parsing, period resolution. |
| Atomic `update` / `try_update` | Cache stats counters, throttling counters, future per-source success/skip counters. |
| `cfg_select!` | Windows vs Unix path handling, release artifact naming, local-git wrappers, `demo-review-rescue.{sh,ps1}` parity. |
| `cold_path` | Fail-closed redaction-key missing, malformed `shiplog.toml`, provider payload rejection, bundle verification failure. |
| Clippy 1.95 lints | `manual_checked_ops`, `manual_take`, `manual_pop_if`, `duration_suboptimal_units`, `unnecessary_trailing_comma`, future `disallowed_fields`. |

API cleanup happens in PR 13, after policy rails are in place to keep the diff
honest.

## PR ladder

Each PR starts from clean `origin/main`. PRs are independent unless explicitly
noted; do not stack unless the PR explicitly depends on prior policy/tooling
work. Every PR opens as draft and applies the per-PR self-review checklist
before being marked ready.

| # | Branch | Title | Depends on |
| - | ------ | ----- | ---------- |
| 1 | `docs/rust-1.95-rollout` | `docs(policy): map Rust 1.95 and 0.5.0 quality rollout` | — (this PR) |
| 2 | `chore/policy-toml-skeletons` | `chore(policy): add CI, lint, panic, and file policy ledgers` | — |
| 3 | `chore/xtask-policy-foundation` | `chore(xtask): add Rust-native policy runner` | PR 2 (schemas) |
| 4 | `probe/rust-1.95-compat` | `chore(msrv): probe Rust 1.95 compatibility` | — |
| 5 | `chore/msrv-rust-1.95` | `chore(msrv): raise workspace toolchain to Rust 1.95` | PR 4 evidence |
| 6 | `ci/lane-whitelist-pr-plan` | `ci: add advisory LEM PR plan` | PR 3 (xtask) |
| 7 | `policy/file-policy-checkers` | `policy(files): enforce repo surface ledgers` | PR 3 |
| 8 | `policy/clippy-ledger` | `policy(clippy): add strict lint ledger and checker` | PR 3 |
| 9 | `policy/no-panic-baseline` | `policy(panic): add exact no-panic baseline` | PR 3 |
| 10 | `policy/rust-1.95-lints-and-ratchets` | `policy(rust+clippy): enable Rust 1.95 floor and ratchets` | PR 5, PR 8 |
| 11 | `ci/ripr-and-evidence-lanes` | `ci: add ripr advisory and targeted evidence lanes` | PR 3, PR 6 |
| 12 | `perf/ci-cache-economics` | `perf(ci): normalize cache and scoped lane routing` | PR 6, PR 11 |
| 13 | `refactor/rust-1.95-api-cleanups` | `refactor: use Rust 1.95 APIs in report builders` | PR 5, PR 10 |
| 14 | `release/0.5.0-prep-rust-1.95` | `release: prepare v0.5.0 for Rust 1.95` | PR 5, PR 13 |

Notes on the ordering:

- TOML skeletons (PR 2) come before xtask (PR 3) so the runner has schemas to
  parse. Skeletons in PR 2 are parse-only and have no enforcement.
- xtask (PR 3) comes before the compatibility probe (PR 4) so the probe can
  use `cargo xtask policy-report` if useful, but PR 4 is otherwise standalone
  and does not require the xtask landing first if the probe is run by hand.
- The compatibility probe (PR 4) and the mechanical bump (PR 5) are split so
  the bump is a small, mechanical PR with all toolchain edits and no policy
  changes.
- The lane whitelist (PR 6) comes before file/clippy/no-panic checkers
  (PR 7–9) so the lane assignments those checkers run inside are explicit.
- Rust 1.95 lints and ratchets (PR 10) come after the MSRV bump (PR 5) and the
  Clippy ledger (PR 8), so activated lints can reference the ledger.
- API cleanup (PR 13) is intentionally last before release prep so the
  modernization diff is reviewed against fully active policy rails.

## Acceptance gates

### PR 1 (this PR)

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
git diff --check
```

PR 1 makes no Cargo, toolchain, workflow, or code changes. It is documentation
only.

### PR 4 (compatibility probe)

```bash
rustup toolchain install 1.95.0 --component rustfmt --component clippy
rustup override set 1.95.0
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo test --doc --workspace --locked
cargo doc --workspace --no-deps
scripts/package-boundary-audit.sh
scripts/package-version-audit.sh
git diff --check
```

PR 4 produces `docs/audits/rust-1.95-compatibility.md`. Code changes are
allowed only for concrete Rust 1.95 compatibility fallout. No MSRV bump, no
policy activation, no release version bump, no API cleanup.

### PR 14 (release prep + dry-run)

```bash
scripts/package-proof.sh
scripts/publish-dry-run.sh
scripts/release-install-smoke.sh v0.5.0   # or document pre-tag limitation
scripts/demo-review-rescue.sh
cargo xtask policy-report
cargo xtask check-lint-policy
cargo xtask check-clippy-exceptions
cargo xtask check-no-panic-family
cargo xtask check-file-policy --mode blocking-allowlist
cargo xtask ripr-pr || true
git diff --check
```

PR 14 produces `docs/release/0.5.0-readiness.md` with: MSRV, version,
package-boundary status, package-version status, publish dry-run status,
Clippy policy status, no-panic status, file-policy status, `ripr` status,
mutation status, known non-blockers, tag procedure, rollback path.

### Every PR

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
git diff --check
```

Plus any policy / xtask / release-proof gate the PR introduces.

## Per-PR operating contract

Apply on every PR in this rollout:

- Start from clean `origin/main`. Do not stack unless explicitly depending on a
  prior policy/tooling PR.
- One PR per objective. Open as draft.
- Do not push `main`. Do not force-push except to your own PR branch after
  rebase.
- Do not merge while required checks are pending. Do not claim green until
  post-merge `main` checks are green.
- Do not add Clippy test carveouts.
- Do not add bare `#[allow(clippy::...)]`. Use `#[expect(..., reason = "...")]`
  with a citation.
- Do not reset the no-panic baseline outside its dedicated PR.
- Do not make `ripr` branch-protection blocking. It stays advisory.
- Do not put full mutation on ordinary PRs. Use the targeted/nightly/release
  lanes.
- Do not enforce hard CI budget caps until lane inventory, PR plan, and
  actuals exist.
- Do not remove release/package proof without replacing the proof obligation.
  Keep shell scripts as wrappers until `xtask` parity is proven.

## Self-review checklist

Required as a PR comment before marking ready:

```markdown
## Self-review

- Scope matches PR title:
- Files touched are expected:
- No unrelated cleanup:
- Policy changes are intentional:
- No Clippy test carveouts added:
- No bare `#[allow(clippy::...)]` added:
- No-panic baseline handling is scoped:
- Non-Rust allowlist changes are narrow:
- Release/package proof preserved:
- Lane assignment changes still advisory (where pre-PR 11):
- Local validation:
- CI status:
- Bot comments addressed:
- Follow-ups:
```

If any item is not true, do not merge.

## Definition of done for v0.5.0

shiplog 0.5.0 is ready to tag when:

```text
Rust 1.95 is declared and used in CI/release.
Existing workflows are inventoried and mapped to lanes (TOML).
CI cost is visible through the LEM PR Plan.
Non-Rust surfaces are allowlisted in TOML.
Generated/executable/workflow/dependency/process/network policies exist.
Clippy policy/debt/exceptions are TOML-backed.
No-panic baseline is exact-identity and no-new-debt.
ripr runs advisory on Rust diffs.
Mutation/property/fuzz/coverage are routed, not default-spend.
Release proof includes package proof, publish dry-run, demo smoke, and
share/report/agent-pack commands.
```

## See also

Policy doctrine:

- [`CLIPPY_POLICY.md`](../CLIPPY_POLICY.md) — Clippy lint ledger model and
  suppression style.
- [`NO_PANIC_POLICY.md`](../NO_PANIC_POLICY.md) — exact-identity no-new-debt
  baseline.
- [`FILE_POLICY.md`](../FILE_POLICY.md) — non-Rust file allowlists and
  companion ledgers.
- [`POLICY_ALLOWLISTS.md`](../POLICY_ALLOWLISTS.md) — schema and suppression
  style across all policy ledgers.

CI economics:

- [`cost-and-verification-policy.md`](cost-and-verification-policy.md) — why
  we route, intent vs duplication, label model.
- [`lem-budgeting.md`](lem-budgeting.md) — Linux Equivalent Minutes cost unit
  and runner multipliers.
- [`verification-ladder.md`](verification-ladder.md) — signal-vs-spend ladder
  per lane.
- [`test-evidence-lanes.md`](test-evidence-lanes.md) — lane doctrine and claim
  boundaries.

Pre-existing CI docs (absorbed unchanged):

- [`coverage.md`](coverage.md) — Codecov execution-surface coverage policy
  and current baseline.
- [`mutation.md`](mutation.md) — mutation testing baselines and claim
  boundary; refreshed in PR 11.
