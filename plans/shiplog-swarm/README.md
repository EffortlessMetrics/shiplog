# Shiplog Swarm Cutover Plan

This plan owns PR-sized execution for moving normal shiplog development to
`EffortlessMetrics/shiplog-swarm` while keeping release authority in
`EffortlessMetrics/shiplog`.

Linked artifacts:

- Proposal: [`SHIPLOG-PROP-0010`](../../docs/proposals/SHIPLOG-PROP-0010-shiplog-swarm-dev-landing-zone.md)
- Spec: [`SHIPLOG-SPEC-0011`](../../docs/specs/SHIPLOG-SPEC-0011-shiplog-swarm-cutover-contract.md)
- ADR: [`SHIPLOG-ADR-0011`](../../docs/adr/SHIPLOG-ADR-0011-swarm-is-dev-landing-zone-not-release-surface.md)
- Plan: [`implementation-plan.md`](./implementation-plan.md)

The plan is not release execution. It does not move tags, crates.io publish,
GitHub Releases, signing, release branches, or security-sensitive workflows.
