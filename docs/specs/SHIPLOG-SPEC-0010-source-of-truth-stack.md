# SHIPLOG-SPEC-0010: Source-of-truth stack contract

Status: proposed
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
- TBD
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
`cargo xtask check-support-tiers`, `cargo xtask policy-report`, and
`cargo xtask check-policy-schemas`.

## Implementation mapping

`docs/`, `plans/`, `.codex/goals/`, `policy/`.

## CI proof

Start advisory, then promote to blocking when dedicated doc/goals validators are
required in CI.

## Metrics / promotion rule

Promote from stabilizing to stable when `check-doc-artifacts`, `check-goals`,
and `check-support-tiers` are implemented and required.

## Failure modes

Unlinked artifacts, stale IDs, and unsupported stable claims.
