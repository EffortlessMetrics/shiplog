# SHIPLOG-PROP-0008: Source-of-truth stack rollout

Status: accepted
Owner: repo-infra
Created: 2026-05-20
Target milestone: 0.10.0
Linked specs:
- SHIPLOG-SPEC-0010-source-of-truth-stack
Linked ADRs:
- none
Linked plan:
- plans/0.10.0/implementation-plan.md
Support-tier impact: yes
Policy impact: yes

## Problem

Shiplog has strong docs and policy fragments, but lacks one linked artifact stack
that answers why/what/how/now/proof per initiative.

## Users and surfaces

Maintainers, Codex agents, and reviewers using roadmap/proposals/specs/plans,
policy ledgers, and support-tier claim boundaries.

## Success criteria

A linked and inspectable repository structure exists with stable artifact IDs,
templates, active goals, support tiers, and a doc artifact ledger.

## Proposed shape

Adopt the source-of-truth artifact taxonomy and wire links across proposal, spec,
plan, active goal manifest, support tiers, and policy/doc-artifacts.toml.

## Alternatives considered

Keep current ad hoc docs (rejected: weak machine-verifiable linkage).

## Specs to create or update

- SHIPLOG-SPEC-0010-source-of-truth-stack

## Architecture decisions needed

- none

## Implementation campaign shape

Scaffold artifacts first, then add/strengthen validators in follow-up slices.

## Evidence plan

`git diff --check`, plus existing policy checks until dedicated doc/goal validators land.

## Risks

Overstating enforcement before dedicated validators are implemented.

## Non-goals

Runtime code behavior changes.

## Exit criteria

Core source-of-truth documents and ledgers are present, linked, and reviewable.

## Acceptance receipt

Accepted on 2026-05-23 after the source-of-truth stack rollout landed through
the scaffold, doc artifact checker, active-goal checker, support-tier checker,
policy CI wiring, repo contract report, PR body generator, and closeout
generator slices.

Receipt anchors:

- `EffortlessMetrics/shiplog-swarm#29` through
  `EffortlessMetrics/shiplog-swarm#37`
- `EffortlessMetrics/shiplog#472` through `EffortlessMetrics/shiplog#480`

Claim boundary: the support-tier rows remain stabilizing. This acceptance means
the rollout contract is adopted and implemented for current repo-control use; it
does not claim README claim scanning, release execution, or stable
branch-protection coverage for every proof command.
