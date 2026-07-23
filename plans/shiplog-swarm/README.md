# Shiplog Swarm Cutover Plan

This plan owns PR-sized execution for moving normal shiplog development to
`EffortlessMetrics/shiplog-swarm` while keeping release authority in
`EffortlessMetrics/shiplog`.

Linked artifacts:

- Proposal: [`SHIPLOG-PROP-0010`](../../docs/proposals/SHIPLOG-PROP-0010-shiplog-swarm-dev-landing-zone.md)
- Spec: [`SHIPLOG-SPEC-0011`](../../docs/specs/SHIPLOG-SPEC-0011-shiplog-swarm-cutover-contract.md)
- ADR: [`SHIPLOG-ADR-0011`](../../docs/adr/SHIPLOG-ADR-0011-swarm-is-dev-landing-zone-not-release-surface.md)
- Plan: [`implementation-plan.md`](./implementation-plan.md)
- Promotion runbook: [`promotion-runbook.md`](./promotion-runbook.md)
- Current promotion manifest: [`promotion-state.toml`](./promotion-state.toml)
  (generated view: [`current-promotion.md`](./current-promotion.md))

The plan is not release execution. It does not move tags, crates.io publish,
GitHub Releases, signing, release branches, or security-sensitive workflows.

## Promotion-state truth hierarchy

Current promotion state has one small authority after Git and GitHub. Highest
authority first:

1. **Git refs and ancestry** — what actually merged where.
2. **GitHub PR / check state** — PR identity, merge method, required checks.
3. **`promotion-state.toml`** — the single manually maintained bounded manifest
   for the latest completed promotion slice and the pending swarm range.
4. **Generated reports** — `target/source-of-truth/*` and the generated
   `current-promotion.md`.
5. **Historical archive** — `implementation-plan.md` and Git history.

`current-promotion.md` is generated from the manifest; do not edit it by hand:

```bash
cargo xtask promotion-state          # regenerate current-promotion.md
cargo xtask promotion-state --check  # validate manifest + verify the generated view is in sync
```

`repo-contract-report` reads the manifest (not the active goal or the plan) to
compute receipt freshness, reporting `current`, `pending-substantive-carry`, or
`stale`. A self-referential receipt-refresh commit and explicitly deferred
receipts are `pending-substantive-carry`, so refreshing receipts never requires
its own follow-up receipt PR.

Current operating state:

- Normal development targets `EffortlessMetrics/shiplog-swarm`.
- Normal swarm PRs squash-merge after `Shiplog Rust Small Result` passes.
- `EffortlessMetrics/shiplog` remains the release/public source surface.
- Source promotion PRs merge with regular merge commits and must not be
  squashed.
