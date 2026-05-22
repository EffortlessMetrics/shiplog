# Support tiers

This file maps product and governance claims to proof commands.

## Tier definitions

| Tier | Meaning |
|---|---|
| Stable | User-facing claim is supported by required proof. |
| Stabilizing | Works for documented paths but is still maturing. |
| Experimental | Available for exploration; broad claims are not allowed. |
| Advisory | Informational only and not branch-protection blocking. |
| Not supported | Explicitly outside current support claims. |

## Claim map

| Surface | Tier | Claim | Proof command | Notes |
|---|---|---|---|---|
| Document artifact links | Stabilizing | Proposal/spec/ADR/plan artifacts are linked by policy ledger. | `cargo xtask check-doc-artifacts` | Dedicated checker validates files, IDs, kinds, statuses, and linked artifact refs; `CI / Policy gates` runs it. |
| Active goal manifest | Stabilizing | Codex execution state links current work to proposal/spec/plan proof. | `cargo xtask check-goals` | Dedicated checker validates `.codex/goals/active.toml`, work-item status, refs, blockers, and proof commands; `CI / Policy gates` runs it. |
| Support-tier claim map | Stabilizing | Product and governance claims map to recognized proof commands. | `cargo xtask check-support-tiers` | Dedicated checker validates tier names, required proof commands, and xtask proof refs; `CI / Policy gates` runs it. |
| Repo contract report | Stabilizing | Humans and agents can inspect the current source-of-truth graph from generated JSON and Markdown. | `cargo xtask repo-contract-report` | Writes `target/source-of-truth/graph.json` and `target/source-of-truth/graph.md`; derived report only, not a validator. |
| PR body generator | Stabilizing | Agents can draft PR bodies from the active work item and linked plan without recreating source-of-truth context by hand. | `cargo xtask pr-body --work-item pr-body-generator` | Writes `target/source-of-truth/pr-body.md`; derived draft only, no GitHub API calls or PR creation. |
| Closeout generator | Stabilizing | Agents can generate closeout and archived-goal drafts from the active goal manifest. | `cargo xtask closeout --goal shiplog-source-of-truth-stack` | Writes `docs/handoffs/*-closeout.md` and `.codex/goals/archive/*.toml`; derived artifacts only, no GitHub API calls. |
| Shiplog swarm control plane | Stabilizing | Normal development targets `EffortlessMetrics/shiplog-swarm`, source promotion uses merge commits into `EffortlessMetrics/shiplog`, and release authority remains on source. | `cargo xtask check-goals` | Active goal links current swarm promotion work to `SHIPLOG-PROP-0010`, `SHIPLOG-SPEC-0011`, `SHIPLOG-ADR-0011`, and `plans/shiplog-swarm/implementation-plan.md`. |
| Package boundary | Stable | Published crate surface is policy-ledger controlled. | `cargo xtask package-boundary` | Guardrail against durable unpublished production crates. |
| Policy ledgers | Stable | Policy files parse and are schema-audited. | `cargo xtask check-policy-schemas` | Requires ledger updates when claims change. |
