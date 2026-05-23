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

Status: done
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Blocks: support-tier-map
Blocked by: doc-artifact-checker
Branch: infra/check-goals
Issue:
PR: EffortlessMetrics/shiplog-swarm#30, EffortlessMetrics/shiplog#473

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

## Work item: support-tier-map

Status: done
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Blocks: policy-ci-wiring
Blocked by: active-goal-checker
Branch: infra/check-support-tiers
Issue:
PR: EffortlessMetrics/shiplog-swarm#31, EffortlessMetrics/shiplog#474

### Goal

Add `cargo xtask check-support-tiers` so product and governance support-tier
claims must carry recognized proof commands.

### Production delta

`xtask` command, tests, support-tier proof command, `.codex/goals/active.toml`,
and source-of-truth docs.

### Non-goals

No CI requirement change, no README claim scanner, and no runtime product
behavior changes.

### Acceptance

- `docs/status/SUPPORT_TIERS.md` parses as the claim-map source.
- Claim map rows have recognized support-tier values.
- Stable and stabilizing claims carry backticked proof commands.
- `cargo xtask ...` proof commands reference known xtask subcommands.
- Existing stale proof-command names are corrected.

### Proof commands

```bash
cargo test -p xtask check_support_tiers --locked
cargo test -p xtask --test cli check_support_tiers --locked
cargo xtask check-support-tiers
git diff --check
```

### Rollback

Revert the checker PR and keep support-tier validation manual.

### Claim boundary

This proves the support-tier claim map syntax and proof-command refs only. It
does not scan README claims or make the source-of-truth stack a required CI gate.

## Work item: policy-ci-wiring

Status: done
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Blocks: repo-contract-report
Blocked by: support-tier-map
Branch: infra/policy-ci-source-truth
Issue:
PR: EffortlessMetrics/shiplog-swarm#32, EffortlessMetrics/shiplog#475

### Goal

Run the source-of-truth validators in the existing `CI / Policy gates` job so
doc artifacts, active goals, and support-tier claims are continuously checked.

### Production delta

`ci.yml` policy-gates steps, CI lane policy/docs, workflow allowlist receipt,
support-tier notes, `.codex/goals/active.toml`, and source-of-truth plan/spec.

### Non-goals

No branch-protection settings change, no new workflow, no release authority
change, and no runtime product behavior changes.

### Acceptance

- `CI / Policy gates` runs `cargo xtask check-doc-artifacts`.
- `CI / Policy gates` runs `cargo xtask check-goals`.
- `CI / Policy gates` runs `cargo xtask check-support-tiers`.
- CI lane docs and policy receipts describe source-of-truth validation without
  stale check counts.
- Source-of-truth status remains stabilizing until branch-protection policy is
  enabled separately.

### Proof commands

```bash
cargo xtask check-doc-artifacts
cargo xtask check-goals
cargo xtask check-support-tiers
cargo xtask check-workflows --mode blocking-allowlist
cargo xtask check-lane-mappings --mode blocking-allowlist
cargo xtask check-actuals-coverage --mode blocking-allowlist
git diff --check
```

### Rollback

Revert the CI wiring PR; the dedicated validators remain available for local
and manual CI use.

### Claim boundary

This proves the source-of-truth validators are wired into the existing policy
job. It does not enable GitHub branch protection or claim README coverage.

## Work item: repo-contract-report

Status: done
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Blocks: source-of-truth-closeout
Blocked by: policy-ci-wiring
Branch: infra/repo-contract-report
Issue:
PR: EffortlessMetrics/shiplog-swarm#33, EffortlessMetrics/shiplog#476

### Goal

Add `cargo xtask repo-contract-report` so humans and agents can inspect the
current source-of-truth graph without manually opening every proposal, spec,
plan, goal manifest, support-tier row, and policy ledger.

### Production delta

`xtask` command and tests, `docs/xtask.md`, support-tier proof command, active
goal manifest, and source-of-truth plan/spec references.

### Non-goals

No branch-protection settings change, no CI requirement change, no release
authority move, and no runtime product behavior changes.

### Acceptance

- `cargo xtask repo-contract-report` reads `policy/doc-artifacts.toml`.
- `cargo xtask repo-contract-report` reads `.codex/goals/active.toml`.
- `cargo xtask repo-contract-report` reads `docs/status/SUPPORT_TIERS.md`.
- The command writes `target/source-of-truth/graph.json`.
- The command writes `target/source-of-truth/graph.md`.
- The report includes active goal, work items, artifacts, support tiers, and
  graph edges.
- The report is derived only; dedicated validators remain the enforcement
  commands.

### Proof commands

```bash
cargo test -p xtask repo_contract_report --locked
cargo test -p xtask --test cli repo_contract_report --locked
cargo xtask repo-contract-report
cargo xtask check-goals
cargo xtask check-support-tiers
git diff --check
```

### Rollback

Revert the report command PR; the source-of-truth validators and policy-gate
checks remain intact.

### Claim boundary

This proves a generated repo-contract inspection report. It does not validate
every product claim, enable branch protection, or replace the dedicated
source-of-truth validators.

## Work item: source-of-truth-closeout

Status: done
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Blocks: none
Blocked by: repo-contract-report
Branch: docs/source-of-truth-closeout
Issue:
PR: EffortlessMetrics/shiplog-swarm#35, EffortlessMetrics/shiplog#478

### Goal

Record the 0.10 source-of-truth stack rollout closeout so humans and agents can
see what landed, what proof passed, what support-tier claims changed, and what
work remains before the next control-plane slice starts.

### Production delta

Closeout/handoff documentation, active-goal status, and generated
repo-contract report evidence only.

### Non-goals

No runtime behavior changes, no branch-protection settings change, no release
authority move, and no new source-of-truth validator.

### Acceptance

- Closeout records landed PRs and promotion receipts.
- Closeout records proof commands and CI signals.
- Closeout records current claim boundaries and remaining work.
- Active goal manifest no longer points agents at completed report work.
- Generated repo-contract report reflects the current active work item.

### Proof commands

```bash
cargo xtask repo-contract-report
cargo xtask check-doc-artifacts
cargo xtask check-goals
cargo xtask check-support-tiers
git diff --check
```

### Rollback

Revert the closeout PR; the source-of-truth validators, support-tier map, and
repo-contract report command remain available.

### Claim boundary

This records the rollout state. It does not prove branch protection,
README-claim scanning, release execution, or complete product-governance
automation.

## Work item: pr-body-generator

Status: done
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Blocks: closeout-generator
Blocked by: source-of-truth-closeout
Branch: infra/pr-body-generator
Issue:
PR: EffortlessMetrics/shiplog-swarm#36, EffortlessMetrics/shiplog#479

### Goal

Add a repo-native PR body generator that reads the active goal manifest and
linked implementation plan so agents can open traceable PRs without recreating
the source-of-truth stack by hand.

### Production delta

`xtask` command, tests, `docs/xtask.md`, and source-of-truth plan/goal updates.

### Non-goals

No GitHub API calls, no PR creation, no branch protection changes, no release
authority move, and no runtime product behavior changes.

### Acceptance

- `cargo xtask pr-body --work-item <id>` reads `.codex/goals/active.toml`.
- The command verifies the work item exists and links to a plan.
- The command writes `target/source-of-truth/pr-body.md`.
- The generated body includes proposal, spec, plan item, scope, non-goals,
  support-tier impact, policy impact, proof commands, claim boundary, and
  rollback when present in the plan.
- Missing work items fail clearly.
- The command does not mutate source artifacts.

### Proof commands

```bash
cargo test -p xtask pr_body --locked
cargo test -p xtask --test cli pr_body --locked
cargo xtask pr-body --work-item pr-body-generator
cargo xtask check-goals
git diff --check
```

### Rollback

Revert the generator PR; existing validators and manual PR bodies remain
available.

### Claim boundary

This would generate PR body drafts only. It would not open PRs, validate GitHub
state, replace reviewer judgment, or prove runtime product behavior.

## Work item: closeout-generator

Status: done
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Blocks: none
Blocked by: pr-body-generator
Branch: infra/closeout-generator
Issue:
PR: EffortlessMetrics/shiplog-swarm#37, EffortlessMetrics/shiplog#480

### Goal

Add a source-of-truth closeout generator so completed active goals can be
archived with consistent proof, claim-boundary, and remaining-work sections.

### Production delta

`xtask` command, tests, `docs/xtask.md`, and source-of-truth plan/goal updates.

### Non-goals

No GitHub API calls, no PR creation, no release authority move, no branch
protection changes, and no runtime product behavior changes.

### Acceptance

- `cargo xtask closeout --goal <goal-id>` reads `.codex/goals/active.toml`.
- The command writes a Markdown closeout under `docs/handoffs/`.
- The command writes an archived goal TOML under `.codex/goals/archive/`.
- The closeout includes landed work items, proof commands, receipts, claim
  boundaries, and remaining work.
- Missing goal IDs fail clearly.
- The command does not mutate provider state or call GitHub.

### Proof commands

```bash
cargo test -p xtask closeout --locked
cargo test -p xtask --test cli closeout --locked
cargo xtask closeout --goal shiplog-source-of-truth-stack
cargo xtask check-goals
git diff --check
```

### Rollback

Revert the closeout generator PR; closeout documents can still be written
manually.

### Claim boundary

This would generate closeout drafts and archived goal manifests only. It would
not infer PR state from GitHub, replace release handoffs, or prove runtime
product behavior.
