# SHIPLOG-SPEC-0010: Source-of-truth stack contract

Status: accepted
Owner: repo-infra
Created: 2026-05-20
Linked proposal: SHIPLOG-PROP-0008-source-of-truth-stack
Linked ADRs:
- none
Linked plan:
- plans/0.10.0/implementation-plan.md
Linked issues:
- none
Linked PRs:
- EffortlessMetrics/shiplog-swarm#29
- EffortlessMetrics/shiplog-swarm#30
- EffortlessMetrics/shiplog-swarm#31
- EffortlessMetrics/shiplog-swarm#32
- EffortlessMetrics/shiplog-swarm#33
- EffortlessMetrics/shiplog-swarm#34
- EffortlessMetrics/shiplog-swarm#35
- EffortlessMetrics/shiplog-swarm#36
- EffortlessMetrics/shiplog-swarm#37
- EffortlessMetrics/shiplog#472
- EffortlessMetrics/shiplog#473
- EffortlessMetrics/shiplog#474
- EffortlessMetrics/shiplog#475
- EffortlessMetrics/shiplog#476
- EffortlessMetrics/shiplog#477
- EffortlessMetrics/shiplog#478
- EffortlessMetrics/shiplog#479
- EffortlessMetrics/shiplog#480
Support-tier impact: stabilizing
Policy impact:
- policy/doc-artifacts.toml
- .codex/goals/active.toml

## Problem

Without linked artifact layers, maintainers and agents cannot reliably answer why
an initiative exists, what behavior is required, and what proves the claim.

## Behavior

The repository must maintain linked roadmap/proposal/spec/ADR/plan/goal/support-tier/policy artifacts.

## Non-goals

Spec does not define runtime feature contracts.

## Required evidence

Policy ledger entries, support-tier claim map, and proof command execution logs.

## Acceptance examples

Proposal and spec IDs resolve in `policy/doc-artifacts.toml`; active goal links to plan and spec.

## Test mapping

`cargo xtask check-doc-artifacts`, `cargo xtask check-goals`,
`cargo xtask check-support-tiers`, `cargo xtask repo-contract-report`,
`cargo xtask policy-report`, and `cargo xtask check-policy-schemas`.

## Implementation mapping

`docs/`, `plans/`, `.codex/goals/`, `policy/`.

## CI proof

The `CI / Policy gates` job runs the dedicated source-of-truth validators:
`cargo xtask check-doc-artifacts`, `cargo xtask check-goals`, and
`cargo xtask check-support-tiers`.

## Metrics / promotion rule

Promote from stabilizing to stable when `check-doc-artifacts`, `check-goals`,
and `check-support-tiers` are implemented, run in CI, and covered by the
branch-protection target policy.

Current status: accepted but still stabilizing. The dedicated validators,
policy-gates wiring, repo-contract report, PR body generator, and closeout
generator have landed. Stable support-tier promotion is intentionally separate
from spec acceptance.

## Failure modes

Unlinked artifacts, stale IDs, and unsupported stable claims.
