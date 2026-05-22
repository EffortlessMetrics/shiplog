# Shiplog source-of-truth stack rollout closeout

Date: 2026-05-22
Goal: `shiplog-source-of-truth-stack`
Status: `done`
Owner: `codex`
Created: `2026-05-20`

## Objective

Establish a linked, enforceable repository artifact stack where initiatives
trace from roadmap intent through proposals/specs/plans/goals to proof and policy.

## End state

- Proposal/spec/ADR/plan/goals/support-tier artifacts are linked.
- Doc artifacts are recorded in policy/doc-artifacts.toml.
- Support tiers map claims to proof commands.

## Landed work items

| Work item | Status | Proposal | Spec | Plan |
|---|---|---|---|---|
| docs-source-of-truth-scaffold | done | SHIPLOG-PROP-0008 | SHIPLOG-SPEC-0010 | plans/0.10.0/implementation-plan.md |
| doc-artifact-checker | done | SHIPLOG-PROP-0008 | SHIPLOG-SPEC-0010 | plans/0.10.0/implementation-plan.md |
| active-goal-checker | done | SHIPLOG-PROP-0008 | SHIPLOG-SPEC-0010 | plans/0.10.0/implementation-plan.md |
| support-tier-map | done | SHIPLOG-PROP-0008 | SHIPLOG-SPEC-0010 | plans/0.10.0/implementation-plan.md |
| policy-ci-wiring | done | SHIPLOG-PROP-0008 | SHIPLOG-SPEC-0010 | plans/0.10.0/implementation-plan.md |
| repo-contract-report | done | SHIPLOG-PROP-0008 | SHIPLOG-SPEC-0010 | plans/0.10.0/implementation-plan.md |
| source-of-truth-closeout | done | SHIPLOG-PROP-0008 | SHIPLOG-SPEC-0010 | plans/0.10.0/implementation-plan.md |
| pr-body-generator | done | SHIPLOG-PROP-0008 | SHIPLOG-SPEC-0010 | plans/0.10.0/implementation-plan.md |
| closeout-generator | done | SHIPLOG-PROP-0008 | SHIPLOG-SPEC-0010 | plans/0.10.0/implementation-plan.md |

## Proof commands

### docs-source-of-truth-scaffold

```bash
git diff --check
```

### doc-artifact-checker

```bash
cargo test -p xtask check_doc_artifacts --locked
cargo test -p xtask --test cli check_doc_artifacts --locked
cargo xtask check-doc-artifacts
git diff --check
```

### active-goal-checker

```bash
cargo test -p xtask check_goals --locked
cargo test -p xtask --test cli check_goals --locked
cargo xtask check-goals
git diff --check
```

### support-tier-map

```bash
cargo test -p xtask check_support_tiers --locked
cargo test -p xtask --test cli check_support_tiers --locked
cargo xtask check-support-tiers
git diff --check
```

### policy-ci-wiring

```bash
cargo xtask check-doc-artifacts
cargo xtask check-goals
cargo xtask check-support-tiers
cargo xtask check-workflows --mode blocking-allowlist
cargo xtask check-lane-mappings --mode blocking-allowlist
cargo xtask check-actuals-coverage --mode blocking-allowlist
git diff --check
```

### repo-contract-report

```bash
cargo test -p xtask repo_contract_report --locked
cargo test -p xtask --test cli repo_contract_report --locked
cargo xtask repo-contract-report
cargo xtask check-goals
cargo xtask check-support-tiers
git diff --check
```

### source-of-truth-closeout

```bash
cargo xtask repo-contract-report
cargo xtask check-doc-artifacts
cargo xtask check-goals
cargo xtask check-support-tiers
git diff --check
```

### pr-body-generator

```bash
cargo test -p xtask pr_body --locked
cargo test -p xtask --test cli pr_body --locked
cargo xtask pr-body --work-item pr-body-generator
cargo xtask check-goals
git diff --check
```

### closeout-generator

```bash
cargo test -p xtask closeout --locked
cargo test -p xtask --test cli closeout --locked
cargo xtask closeout --goal shiplog-source-of-truth-stack
cargo xtask check-goals
git diff --check
```

## Receipts

### docs-source-of-truth-scaffold

- `EffortlessMetrics/shiplog#457`

### doc-artifact-checker

- `EffortlessMetrics/shiplog-swarm#29`
- `EffortlessMetrics/shiplog#472`

### active-goal-checker

- `EffortlessMetrics/shiplog-swarm#30`
- `EffortlessMetrics/shiplog#473`

### support-tier-map

- `EffortlessMetrics/shiplog-swarm#31`
- `EffortlessMetrics/shiplog#474`

### policy-ci-wiring

- `EffortlessMetrics/shiplog-swarm#32`
- `EffortlessMetrics/shiplog#475`

### repo-contract-report

- `EffortlessMetrics/shiplog-swarm#33`
- `EffortlessMetrics/shiplog#476`

### source-of-truth-closeout

- `EffortlessMetrics/shiplog-swarm#35`
- `EffortlessMetrics/shiplog#478`

### pr-body-generator

- `EffortlessMetrics/shiplog-swarm#36`
- `EffortlessMetrics/shiplog#479`

## Claim boundaries

### docs-source-of-truth-scaffold

Does not prove runtime feature behavior; only repo governance and traceability surfaces.

### doc-artifact-checker

This proves the document artifact ledger only. It does not prove active-goal
manifests or make the source-of-truth stack a required CI gate.

### active-goal-checker

This proves the active Codex goal manifest only. It does not make the
source-of-truth stack a required CI gate or validate archived legacy goals.

### support-tier-map

This proves the support-tier claim map syntax and proof-command refs only. It
does not scan README claims or make the source-of-truth stack a required CI gate.

### policy-ci-wiring

This proves the source-of-truth validators are wired into the existing policy
job. It does not enable GitHub branch protection or claim README coverage.

### repo-contract-report

This proves a generated repo-contract inspection report. It does not validate
every product claim, enable branch protection, or replace the dedicated
source-of-truth validators.

### source-of-truth-closeout

This records the rollout state. It does not prove branch protection,
README-claim scanning, release execution, or complete product-governance
automation.

### pr-body-generator

This would generate PR body drafts only. It would not open PRs, validate GitHub
state, replace reviewer judgment, or prove runtime product behavior.

### closeout-generator

This would generate closeout drafts and archived goal manifests only. It would
not infer PR state from GitHub, replace release handoffs, or prove runtime
product behavior.

## Remaining work

No active, ready, or blocked work items remain in this goal manifest.

## Generated boundary

This closeout is generated from `.codex/goals/active.toml` and linked plan text. It does not inspect GitHub state, infer merged PRs, replace release handoffs, mutate provider records, or prove runtime product behavior.
