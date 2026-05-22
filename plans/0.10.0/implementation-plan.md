# 0.10.0 Source-of-Truth Rollout Plan

Plan artifact: SHIPLOG-PLAN-0010

## Work item: docs-source-of-truth-scaffold

Status: done
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Blocks: policy-doc-artifact-ledger
Blocked by: none
Branch: docs/source-of-truth-stack
Issue:
PR: EffortlessMetrics/shiplog#457

### Goal

Add linked scaffolding artifacts for proposals, specs, ADRs, plans, active goals,
support tiers, and policy ledgers.

### Production delta

Documentation, templates, goals manifests, and policy ledger files only.

### Non-goals

No runtime behavior changes or broad refactors.

### Acceptance

Artifact structure exists and files cross-link to the source-of-truth stack.

### Proof commands

```bash
git diff --check
```

### Rollback

Revert this PR as a single documentation/policy slice.

### Claim boundary

Does not prove runtime feature behavior; only repo governance and traceability surfaces.

## Work item: doc-artifact-checker

Status: done
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Blocks: active-goal-checker
Blocked by: policy-doc-artifact-ledger
Branch: infra/check-doc-artifacts
Issue:
PR: EffortlessMetrics/shiplog-swarm#29, EffortlessMetrics/shiplog#472

### Goal

Add `cargo xtask check-doc-artifacts` so the document artifact ledger can be
validated directly instead of only summarized by `policy-report`.

### Production delta

`xtask` command, tests, support-tier proof command, and source-of-truth docs.

### Non-goals

No CI requirement change, no active-goal validator, and no runtime product
behavior changes.

### Acceptance

- `policy/doc-artifacts.toml` parses.
- Artifact IDs are unique.
- Listed files exist.
- Listed IDs are mentioned in their files.
- Artifact kind/status values are recognized.
- Linked proposal/spec/ADR/plan IDs resolve.

### Proof commands

```bash
cargo test -p xtask check_doc_artifacts
cargo test -p xtask --test cli check_doc_artifacts
cargo xtask check-doc-artifacts
git diff --check
```

### Rollback

Revert the checker PR and keep the support-tier proof command on
`cargo xtask policy-report`.

### Claim boundary

This proves the document artifact ledger only. It does not prove active-goal
manifests or make the source-of-truth stack a required CI gate.

## Work item: active-goal-checker

Status: done
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Blocks: support-tier-map
Blocked by: doc-artifact-checker
Branch: infra/check-goals
Issue:
PR: EffortlessMetrics/shiplog-swarm#30, EffortlessMetrics/shiplog#473

### Goal

Add `cargo xtask check-goals` so the active Codex goal manifest can be
validated directly instead of relying on chat memory or manual inspection.

### Production delta

`xtask` command, tests, support-tier proof command, `.codex/goals/active.toml`,
and source-of-truth docs.

### Non-goals

No CI requirement change, no runtime product behavior changes, and no validation
of legacy `.shiplog/goals/archive/` manifests.

### Acceptance

- `.codex/goals/active.toml` parses.
- Goal and work-item statuses are recognized.
- Work item IDs are unique.
- At most one work item is active.
- Proposal/spec references resolve through `policy/doc-artifacts.toml`.
- Plan references are safe repo-relative paths and ledgered plan artifacts.
- Ready/active work items carry proof commands.
- Blocked work items name blockers.
- Done work items carry proof commands or receipt refs.

### Proof commands

```bash
cargo test -p xtask check_goals --locked
cargo test -p xtask --test cli check_goals --locked
cargo xtask check-goals
git diff --check
```

### Rollback

Revert the checker PR and keep active goal manifest validation manual.

### Claim boundary

This proves the active Codex goal manifest only. It does not make the
source-of-truth stack a required CI gate or validate archived legacy goals.

## Work item: support-tier-map

Status: done
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Blocks: policy-ci-wiring
Blocked by: active-goal-checker
Branch: infra/check-support-tiers
Issue:
PR: EffortlessMetrics/shiplog-swarm#31, EffortlessMetrics/shiplog#474

### Goal

Add `cargo xtask check-support-tiers` so product and governance support-tier
claims must carry recognized proof commands.

### Production delta

`xtask` command, tests, support-tier proof command, `.codex/goals/active.toml`,
and source-of-truth docs.

### Non-goals

No CI requirement change, no README claim scanner, and no runtime product
behavior changes.

### Acceptance

- `docs/status/SUPPORT_TIERS.md` parses as the claim-map source.
- Claim map rows have recognized support-tier values.
- Stable and stabilizing claims carry backticked proof commands.
- `cargo xtask ...` proof commands reference known xtask subcommands.
- Existing stale proof-command names are corrected.

### Proof commands

```bash
cargo test -p xtask check_support_tiers --locked
cargo test -p xtask --test cli check_support_tiers --locked
cargo xtask check-support-tiers
git diff --check
```

### Rollback

Revert the checker PR and keep support-tier validation manual.

### Claim boundary

This proves the support-tier claim map syntax and proof-command refs only. It
does not scan README claims or make the source-of-truth stack a required CI gate.

## Work item: policy-ci-wiring

Status: active
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Blocks: repo-contract-report
Blocked by: support-tier-map
Branch: infra/policy-ci-source-truth
Issue:
PR: EffortlessMetrics/shiplog-swarm#32

### Goal

Run the source-of-truth validators in the existing `CI / Policy gates` job so
doc artifacts, active goals, and support-tier claims are continuously checked.

### Production delta

`ci.yml` policy-gates steps, CI lane policy/docs, workflow allowlist receipt,
support-tier notes, `.codex/goals/active.toml`, and source-of-truth plan/spec.

### Non-goals

No branch-protection settings change, no new workflow, no release authority
change, and no runtime product behavior changes.

### Acceptance

- `CI / Policy gates` runs `cargo xtask check-doc-artifacts`.
- `CI / Policy gates` runs `cargo xtask check-goals`.
- `CI / Policy gates` runs `cargo xtask check-support-tiers`.
- CI lane docs and policy receipts describe source-of-truth validation without
  stale check counts.
- Source-of-truth status remains stabilizing until branch-protection policy is
  enabled separately.

### Proof commands

```bash
cargo xtask check-doc-artifacts
cargo xtask check-goals
cargo xtask check-support-tiers
cargo xtask check-workflows --mode blocking-allowlist
cargo xtask check-lane-mappings --mode blocking-allowlist
cargo xtask check-actuals-coverage --mode blocking-allowlist
git diff --check
```

### Rollback

Revert the CI wiring PR; the dedicated validators remain available for local
and manual CI use.

### Claim boundary

This proves the source-of-truth validators are wired into the existing policy
job. It does not enable GitHub branch protection or claim README coverage.
