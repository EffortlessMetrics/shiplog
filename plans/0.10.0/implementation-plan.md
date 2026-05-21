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

Status: active
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Blocks: support-tier-map
Blocked by: doc-artifact-checker
Branch: infra/check-goals
Issue:
PR: EffortlessMetrics/shiplog-swarm#30

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
