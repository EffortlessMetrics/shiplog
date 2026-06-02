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
| Document artifact links | Stabilizing | Proposal/spec/ADR/plan artifacts are linked by policy ledger. | `rtk cargo xtask check-doc-artifacts` | Dedicated checker validates files, IDs, kinds, statuses, and linked artifact refs; `CI / Policy gates` runs it. |
| Goal manifests | Stabilizing | Codex execution state and archived goal receipts link work to proposal/spec/plan proof. | `rtk cargo xtask check-goals` | Dedicated checker validates `.codex/goals/active.toml`, `.codex/goals/archive/*.toml`, work-item status, refs, blockers, and proof commands; `CI / Policy gates` runs it. |
| Support-tier claim map | Stabilizing | Product and governance claims map to recognized proof commands. | `rtk cargo xtask check-support-tiers` | Dedicated checker validates tier names, required proof commands, and xtask proof refs; `CI / Policy gates` runs it. |
| Repo contract report | Stabilizing | Humans and agents can inspect the current source-of-truth graph, recommended next slice, source/swarm topology, local and remote branch hygiene, PR/issue queue hygiene, latest routed CI health, latest promotion PR traceability, branch-protection contract, and promotion receipt freshness from generated JSON and Markdown. | `rtk cargo xtask repo-contract-report` | Writes `target/source-of-truth/graph.json` and `target/source-of-truth/graph.md`; derives a top-level recommended next slice from existing report statuses, names exact missing receipt refs when receipt freshness drives the top recommendation, classifies source-ahead promotion merges, reports local branches already merged into source or swarm, surfaces large unprotected local branch inventories as non-destructive review hints with a bounded branch-name list, includes local cleanup review commands that check both GitHub repos before showing branch commits, splits remote branch cleanup candidates into merged/review buckets, includes non-destructive PR/log review commands for both remote cleanup buckets, keeps open PR-backed branches out of cleanup candidate counts, reports open source/swarm PRs and issues when GitHub inspection is available, reports the latest `EM CI Routed Shiplog Rust` run on source/swarm main, checks the latest source promotion PR title/body/merge commit against the promotion contract, verifies swarm/source proof sections, and lists failed receipt checks, checks whether `shiplog-swarm/main` requires only `Shiplog Rust Small Result`, and reports missing latest promotion receipts; derived report only, not a branch-protection mutator, validator, PR/issue mutator, CI rerunner, promotion mutator, or branch cleanup command. |
| PR body generator | Stabilizing | Agents can draft source-of-truth work-item PR bodies from the selected active-goal work item and linked plan without recreating that context by hand. | `rtk cargo xtask pr-body --work-item promotion-cadence --output target/source-of-truth/pr-body.md` | Writes `target/source-of-truth/pr-body.md`; derived work-item draft only, not a generic body for arbitrary local diffs; no GitHub API calls or PR creation. |
| Promotion body generator | Stabilizing | Agents can draft source promotion PR bodies with the swarm head, included swarm PRs, proof slots, and merge-commit boundary. | `rtk cargo xtask promotion-body --output target/source-of-truth/promotion-body.md` | Writes `target/source-of-truth/promotion-body.md`; supports explicit swarm head and included-PR overrides for final proof updates after source/swarm refs align; derived draft only, no GitHub API calls, PR creation, PR editing, branch mutation, merge, release, or publish action. |
| Closeout generator | Stabilizing | Agents can generate closeout and archived-goal drafts from the active goal manifest. | `rtk cargo xtask closeout --goal shiplog-swarm-control-plane --handoff-output target/source-of-truth/closeout.md --archive-output target/source-of-truth/active-goal-archive.toml` | Writes reviewable drafts under `target/source-of-truth/`; derived artifacts only, no GitHub API calls. |
| Shiplog swarm control plane | Stabilizing | Normal development targets `EffortlessMetrics/shiplog-swarm`, source promotion uses merge commits into `EffortlessMetrics/shiplog`, and release authority remains on source. | `rtk cargo xtask check-goals` | Active goal links current swarm promotion work to accepted `SHIPLOG-PROP-0010`, `SHIPLOG-SPEC-0011`, `SHIPLOG-ADR-0011`, and `plans/shiplog-swarm/implementation-plan.md`. |
| Workflow self-hosted admission | Stabilizing | Non-routed `shiplog-swarm` workflows only select self-hosted runners for non-PR events or same-repo pull requests. | `rtk cargo xtask check-workflows` | Workflow policy rejects repo-name-only self-hosted selectors so fork PRs fall back to GitHub-hosted runners unless a dedicated routed workflow applies stricter admission. |
| Package boundary | Stable | Published crate surface is policy-ledger controlled. | `rtk cargo xtask package-boundary` | Guardrail against durable unpublished production crates. |
| Policy ledgers | Stable | Policy files parse and are schema-audited. | `rtk cargo xtask check-policy-ledgers` | First-class policy-ledger proof command; delegates to the schema/header audit used by CI. |
