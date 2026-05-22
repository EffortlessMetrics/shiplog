# Source-of-truth stack closeout

Date: 2026-05-22
Goal: `shiplog-source-of-truth-stack`
Plan: `plans/0.10.0/implementation-plan.md`
Proposal: `SHIPLOG-PROP-0008`
Spec: `SHIPLOG-SPEC-0010`

## Landed

The 0.10 source-of-truth rollout established a linked, enforceable repository
artifact stack for repo-control work:

- proposal/spec/plan scaffolding and templates;
- `policy/doc-artifacts.toml` as the document artifact ledger;
- `.codex/goals/active.toml` as the current Codex execution manifest;
- `docs/status/SUPPORT_TIERS.md` as the product/governance claim map;
- dedicated validators for doc artifacts, active goals, and support tiers;
- `CI / Policy gates` wiring for the dedicated source-of-truth validators;
- `cargo xtask repo-contract-report` as a generated inspection report for
  humans and agents.

## PR receipts

| Slice | Swarm PR | Source promotion |
|---|---|---|
| Source-of-truth scaffold | - | `EffortlessMetrics/shiplog#457` |
| Document artifact checker | `EffortlessMetrics/shiplog-swarm#29` | `EffortlessMetrics/shiplog#472` |
| Active goal checker | `EffortlessMetrics/shiplog-swarm#30` | `EffortlessMetrics/shiplog#473` |
| Support-tier map checker | `EffortlessMetrics/shiplog-swarm#31` | `EffortlessMetrics/shiplog#474` |
| Policy CI wiring | `EffortlessMetrics/shiplog-swarm#32` | `EffortlessMetrics/shiplog#475` |
| Repo contract report | `EffortlessMetrics/shiplog-swarm#33` | `EffortlessMetrics/shiplog#476` |
| Close completed report work item | `EffortlessMetrics/shiplog-swarm#34` | `EffortlessMetrics/shiplog#477` |

Latest promoted source commit observed before this closeout:

```text
f74ca71b71408cc957333b4f824b0f5df00dba07 Merge pull request #477 from EffortlessMetrics/promote/swarm-20260521-fac7875
```

## Proof observed

Local proof commands used across the rollout:

```bash
cargo test -p xtask check_doc_artifacts --locked
cargo test -p xtask --test cli check_doc_artifacts --locked
cargo xtask check-doc-artifacts
cargo test -p xtask check_goals --locked
cargo test -p xtask --test cli check_goals --locked
cargo xtask check-goals
cargo test -p xtask check_support_tiers --locked
cargo test -p xtask --test cli check_support_tiers --locked
cargo xtask check-support-tiers
cargo test -p xtask repo_contract_report --locked
cargo test -p xtask --test cli repo_contract_report --locked
cargo xtask repo-contract-report
cargo xtask check-workflows --mode blocking-allowlist
cargo xtask check-lane-mappings --mode blocking-allowlist
cargo xtask check-actuals-coverage --mode blocking-allowlist
git diff --check
```

CI proof observed after the latest source promotion:

- `CI / Policy gates`: passed.
- `CI / cargo-deny`: passed.
- `CI / Check (ubuntu-latest)`: passed.
- `CI / Check (windows-latest)`: passed.
- `EM CI Routed Shiplog Rust / Shiplog Rust Small Result`: passed on
  GitHub-hosted for the source repo.
- `BDD Smoke`, `BDD Testing`, `Property Smoke`, `Property Testing`,
  `Fuzz Smoke`, `Fuzzing`, and `Security Checks`: passed.

## Claims changed

The source-of-truth stack may now claim stabilizing support for:

- linked document artifact validation via `cargo xtask check-doc-artifacts`;
- active Codex goal validation via `cargo xtask check-goals`;
- support-tier claim-map validation via `cargo xtask check-support-tiers`;
- generated graph inspection via `cargo xtask repo-contract-report`.

These claims remain stabilizing, not stable, because branch-protection policy
for source-of-truth validation has not been separately promoted.

## Policy/support-tier updates

Policy and support-tier surfaces updated during the rollout:

- `policy/doc-artifacts.toml`
- `policy/workflow-allowlist.toml`
- `policy/ci-lane-whitelist.toml`
- `.codex/goals/active.toml`
- `docs/status/SUPPORT_TIERS.md`
- `docs/xtask.md`
- `docs/ci/policy-ledgers.md`
- `docs/specs/SHIPLOG-SPEC-0010-source-of-truth-stack.md`

## Claim boundary

This closeout records repo-control infrastructure. It does not prove:

- branch protection has been enabled for source-of-truth checks;
- README claim scanning exists;
- support-tier promotion from stabilizing to stable is justified;
- release execution has happened;
- release authority moved to `shiplog-swarm`;
- runtime product behavior changed.

## Remaining work

Recommended next control-plane slices:

- add `cargo xtask pr-body --work-item <id>` so PR descriptions can be
  generated from active goal, plan, spec, and proposal links;
- add a closeout generator once the handoff format has one more dogfood pass;
- decide whether and when source-of-truth checks graduate from stabilizing to
  stable support-tier claims;
- keep promotion PRs from `shiplog-swarm` to `shiplog` as regular merge commits
  with swarm SHA, included PRs, and proof notes.

## Archive actions

This closeout PR marks `source-of-truth-closeout` done in the active goal
manifest and points the next agent at the PR body generator slice.
