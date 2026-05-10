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
| MSRV | 1.92 | 1.95 | landed (#145) |
| Toolchain pin (`rust-toolchain.toml`) | 1.92 | 1.95.0 | landed (#145) |
| `release.yml` toolchain pin | 1.92 (preflight, build, validation, integration) | 1.95.0 | landed (#145; release moved with CI) |
| Workspace version | 0.4.0 | 0.5.0 | planned (PR #157) |
| Workspace lints (Rust) | `unsafe_code = deny`, `missing_debug_implementations = allow` | 1.95 lint floor (`unsafe_op_in_unsafe_fn`, `unused_must_use`, `unexpected_cfgs`, `const_item_interior_mutations`, `function_casts_as_integer`, `unused_visibilities`) | planned (PR #152) |
| Workspace lints (Clippy) | `enum_glob_use`, `flat_map_option` warn; `needless_pass_by_value`, `cloned_instead_of_copied` allow as debt | 1.94/1.95 ratchets activated; broad allows moved to receipted debt ledger | planned (PR #150 ledger, PR #152 ratchets) |
| `clippy.toml` | absent | present, MSRV-aware, no test carveouts | landed (#145) |
| `xtask/` | absent | thin Rust-native policy runner | planned (PR #143) |
| `policy/` ledger files | absent | TOML skeletons present (PR #141), enforced by xtask (PR #149–#152) | planned (PR #141 skeleton, PR #149+ enforcement) |
| `ci/lanes` policy | implicit | `policy/ci-budget.toml`, `policy/ci-lanes.toml`, `policy/ci-risk-packs.toml`, `pr-plan.yml` advisory | planned (PR #146) |
| CI operating contracts | implicit | `docs/ci/{ci-lane-map,labels,risk-packs,branch-protection,required-check-migration,skipped-by-policy,bot-review-policy,cache-policy,ci-plan-json,ci-actuals,ripr,per-pr-acceptance-contract,policy-ledgers}.md` + JSON schemas | planned (PR #142) |
| No-panic baseline | absent | exact-identity (path + family + selector_kind + selector_callee + snippet + count), no-new-debt mode | planned (PR #151) |
| Non-Rust file policy | absent | non-Rust + companion ledgers (generated, executable, dependency, workflow, process, network) | planned (PR #149) |
| `ripr` | absent | advisory PR-time exposure filter, label-promotable | landed (#153, v1 stub — real analysis is a follow-up release) |
| Bounded stochastic on PR fast | broad-on-every-PR today (BDD 4 jobs, property 256-case all crates, quick fuzz × 9 targets) | bounded BDD smoke + bounded proptest + touched-target quick fuzz; deterministic seeds and time caps | landed (#154 — broad sweeps still PR-on-every until #155 routes them) |
| Broad evidence routing | always-on parity testing on every PR | label / risk pack on PR-targeted, full sweeps on nightly; mutation snapshot on release | planned (PR #155; reconciles existing `mutation-testing.yml`) |
| CI cache economics | mixed save/restore on every PR | PR caches restore-only, `main` saves canonical, docs-only skips compile-heavy non-required lanes | landed (#147) |
| CI actuals feedback loop | absent | `target/ci/ci-actuals.json` against v1 schema; calibrates the LEM forecast | landed (#148) |
| Release proof | shell scripts (`package-proof.sh`, `publish-dry-run.sh`, `package-boundary-audit.sh`, `package-version-audit.sh`, `verify-release.sh`, `release-install-smoke.{sh,ps1}`) | shell scripts wrap `xtask` parity over time; never silently dropped | partial (PR #143 wraps; release-readiness in PR #157) |

## Existing CI / evidence inventory and tentative lanes

shiplog already runs 11 workflows. The gap this rollout closes is **policy
ledgers, xtask, ripr, and explicit lane economics** — not test infrastructure.

The lane assignments in this section are **tentative and advisory**. They are
this PR's read of where each workflow should sit; they are not enforced policy.
Enforcement comes after the lane whitelist (PR #146), the PR plan (PR #146),
and actuals (PR #148) exist.

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
  PR gate. PR #155 adds explicit label-gated PR-targeted entry points; the
  weekly mutation lane is unchanged. `ripr` (PR #153) becomes the cheap PR-time
  oracle-gap detector that justifies keeping full mutation off the default PR
  gate — see [`ripr` and mutation
  economics](test-evidence-lanes.md#ripr-and-mutation-economics).

These are read-only assignments in this PR. PR #146 encodes them as TOML in
`policy/ci-lanes.toml` (still advisory). PR #153 wires `ripr.yml`; PR #154
carves out the bounded stochastic PR-fast lane; PR #155 routes the broad
sweeps to nightly / label / risk pack. PR #148 measures actuals. Hard
enforcement is a follow-up release decision, not v0.5.0.

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

API cleanup happens in PR #156, after policy rails are in place to keep the
diff honest.

## PR ladder

18 PRs across 9 phases (#140 through #157). Each PR starts from clean
`origin/main`. PRs are independent unless explicitly noted; do not stack
unless the PR explicitly depends on prior policy/tooling work. Every PR opens
as draft and applies the per-PR self-review checklist before being marked
ready.

| # | Branch | Title | Phase | Depends on |
| - | ------ | ----- | ----- | ---------- |
| #140 | `docs/rust-1.95-rollout` | `docs(policy): map Rust 1.95 and 0.5.0 quality rollout` | 0 — close map | — (this PR) |
| #141 | `chore/policy-toml-skeletons` | `chore(policy): add CI, lint, panic, and file policy ledgers` | 1 — machine-readable | — |
| #142 | `docs/ci-operating-contracts` | `docs(ci): add lane contracts and acceptance templates` | 1 — machine-readable | — |
| #143 | `chore/xtask-policy-foundation` | `chore(xtask): add Rust-native policy runner` | 2 — control plane | #141 (schemas) |
| #144 | `probe/rust-1.95-compat` | `chore(msrv): probe Rust 1.95 compatibility` | 3 — Rust 1.95 | — |
| #145 | `chore/msrv-rust-1.95` | `chore(msrv): raise workspace toolchain to Rust 1.95` | 3 — Rust 1.95 | #144 evidence |
| #146 | `ci/lane-whitelist-pr-plan` | `ci: add advisory LEM PR plan` | 4 — CI economics | #143 |
| #147 | `perf/ci-cache-economics` | `perf(ci): normalize cache and scoped lane routing` | 4 — CI economics | #146 |
| #148 | `ci/ci-actuals` | `ci: emit CI actuals` | 4 — CI economics | #146 |
| #149 | `policy/file-policy-checkers` | `policy(files): enforce repo surface ledgers` | 5 — policy enforcement | #143 |
| #150 | `policy/clippy-ledger` | `policy(clippy): add strict lint ledger and checker` | 5 — policy enforcement | #143 |
| #151 | `policy/no-panic-baseline` | `policy(panic): add exact no-panic baseline` | 5 — policy enforcement | #143 |
| #152 | `policy/rust-1.95-lints-and-ratchets` | `policy(rust+clippy): enable Rust 1.95 floor and ratchets` | 5 — policy enforcement | #145, #150 |
| #153 | `ci/ripr-advisory` | `ci: add ripr advisory lane` | 6 — evidence lanes | #143, #146 |
| #154 | `ci/bounded-stochastic-pr` | `ci: add bounded stochastic PR-fast lane` | 6 — evidence lanes | #146 |
| #155 | `ci/broad-lane-routing` | `ci: route broad BDD/property/fuzz/mutation lanes` | 6 — evidence lanes | #146, #153 |
| #156 | `refactor/rust-1.95-api-cleanups` | `refactor: use Rust 1.95 APIs in report builders` | 7 — small 1.95 cleanup | #145, #152 |
| #157 | `release/0.5.0-prep-rust-1.95` | `release: prepare v0.5.0 for Rust 1.95` | 8 — release | #145, #156 |

(Actual GitHub PR numbers assume no unrelated PRs interleave between #140 and
the next slice's open. If Dependabot or another PR grabs a number, the
references shift; the `Branch` column is the stable identifier.)

Notes on the ordering:

- **Phase 1 splits the original "TOML + docs" PR into two** (#141 + #142)
  so the machine-readable layer (TOML skeletons) ships separately from the
  human-readable operating contracts (lane map, labels, risk packs, branch
  protection, required-check migration, skipped-by-policy semantics,
  bot-review policy, cache policy, ci-plan/actuals JSON schemas, ripr
  operating doc, per-PR acceptance contract template). Each is independently
  reviewable.
- **TOML skeletons (#141) come before xtask (#143)** so the runner has
  schemas to parse. Skeletons in #141 are parse-only and have no enforcement.
- **The compatibility probe (#144) and the mechanical bump (#145) are split**
  so the bump is a small, mechanical PR with all toolchain edits (Cargo.toml,
  rust-toolchain.toml, clippy.toml, ci.yml, release.yml, README/CLAUDE/docs
  references) and no policy changes.
- **Phase 4 splits CI economics into three PRs**: PR plan (#146), cache
  policy (#147), and CI actuals (#148). Actuals is its own PR because it
  turns LEM from a forecast into a feedback loop and deserves its own
  review.
- **The lane whitelist (#146) comes before file/clippy/no-panic checkers**
  (#149–#151) so the lane assignments those checkers run inside are
  explicit.
- **Rust 1.95 lints and ratchets (#152) come after the MSRV bump (#145) and
  the Clippy ledger (#150)**, so activated lints can reference the ledger.
- **Phase 6 splits the original ripr+lanes PR into three** (#153 + #154 +
  #155): ripr advisory only, bounded stochastic PR-fast carve-out, and
  broad lane routing. This keeps the blast radius narrow per PR.
- **API cleanup (#156) is intentionally last before release prep** so the
  modernization diff is reviewed against fully active policy rails.

## Acceptance gates

### PR #140 (this PR)

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
git diff --check
```

PR #140 makes no Cargo, toolchain, workflow, or code changes. It is
documentation only.

### PR #144 (compatibility probe)

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

PR #144 produces `docs/audits/rust-1.95-compatibility.md`. Code changes are
allowed only for concrete Rust 1.95 compatibility fallout. No MSRV bump, no
policy activation, no release version bump, no API cleanup.

### PR #157 (release prep + dry-run)

```bash
scripts/package-proof.sh
scripts/publish-dry-run.sh
scripts/release-install-smoke.sh v0.5.0   # post-tag only; pre-tag local smoke documented in readiness doc
scripts/demo-review-rescue.sh
cargo xtask policy-report
cargo xtask check-lint-policy
cargo xtask check-clippy-exceptions
cargo xtask check-no-panic-family
cargo xtask check-file-policy --mode blocking-allowlist
cargo xtask ripr-pr || true
git diff --check
```

PR #157 produces `docs/release/0.5.0-readiness.md` and
`RELEASE_HANDOFF_0.5.0.md` with: MSRV/toolchain proof, package-boundary
status, package-version status, publish dry-run status, Clippy policy status,
no-panic status, file-policy status, `ripr` status, mutation status, release
asset smoke (split: pre-tag local smoke vs post-tag downloaded artifact
smoke), demo review-rescue smoke, share/report/agent-pack command smoke,
known non-blockers, tag/publish order, rollback path.

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
- Lane assignment changes still advisory (PR #146 encodes; #153/#154/#155 wire):
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
  boundary; refreshed in PR #155 (broad lane routing).
