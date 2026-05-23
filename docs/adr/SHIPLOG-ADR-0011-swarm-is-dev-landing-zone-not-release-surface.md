# SHIPLOG-ADR-0011: Swarm Is Dev Landing Zone, Not Release Surface

Status: accepted
Date: 2026-05-21
Owner: repo-infra/release
Linked proposal:
[`SHIPLOG-PROP-0010-shiplog-swarm-dev-landing-zone`](../proposals/SHIPLOG-PROP-0010-shiplog-swarm-dev-landing-zone.md)
Linked specs:
- [`SHIPLOG-SPEC-0011-shiplog-swarm-cutover-contract`](../specs/SHIPLOG-SPEC-0011-shiplog-swarm-cutover-contract.md)
Linked plan:
[`plans/shiplog-swarm/implementation-plan.md`](../../plans/shiplog-swarm/implementation-plan.md)

## Decision

Use `EffortlessMetrics/shiplog-swarm` as the normal development landing zone
after cutover, while `EffortlessMetrics/shiplog` remains the release surface
until an explicit release cutover moves that authority.

Normal PRs squash-merge into `shiplog-swarm/main`. Periodic promotions from
`shiplog-swarm/main` into `shiplog/main` use regular merge commits and must not
be squashed.

The two repositories must share Git history before cutover. If
`git merge-base origin/main swarm/main` is empty, `shiplog-swarm/main` must be
reseeded from `shiplog/main` before normal development moves there.

## Context

Shiplog now has a 0.9 release surface and a growing agent-driven development
model. The source repo carries release authority: tags, crates.io publish,
GitHub Releases, signing, release branches, announcement workflows, and
security-sensitive token workflows.

The swarm repo should provide a safer landing zone for normal development:
same-repo trusted branches, routed Linux CI, GitHub-hosted fallback, and fork
PRs kept off self-hosted runners. That should not imply release authority has
moved.

The initial remote audit found `shiplog-swarm/main` as an orphan initialization
commit, not a shared-history import from `shiplog/main`. A follow-up
remote-state operation on 2026-05-21 reseeded `shiplog-swarm/main` from
`shiplog/main`, making both refs point at
`48c0da1b9a5aeefe58a79c472a8c35d9590e3657`.

Follow-up receipts in the implementation plan now show routed CI, fork
admission, branch protection, machine cutover, and merge-commit source
promotions are proven. Promotion merges are the normal source-update model for
accepted swarm work. Release authority has not moved.

## Consequences

- Agents and maintainers get one normal development target after cutover.
- `shiplog` remains the public release checkpoint and authority until a later
  release-cutover ADR/spec says otherwise.
- Promotion PRs preserve the already-squashed swarm commits under one explicit
  merge checkpoint in the source repo.
- Source-only release changes are exceptional; if they happen, they must be
  synced back into `shiplog-swarm` before more normal development lands.
- Branch protection for `shiplog-swarm` can require one normalized routed result
  instead of conditional implementation jobs.

## Alternatives Considered

### Move release authority to swarm immediately

Rejected. Release tags, crates.io publish, GitHub Releases, signing, and
security-sensitive workflows need a separate cutover decision and proof.

### Keep normal development in `shiplog`

Rejected for the long term. It keeps everyday agent work coupled to the release
surface and does not exercise the routed swarm landing-zone model.

### Patch-copy or cherry-pick from swarm into source

Rejected. That creates fragile history and loses the clear promotion checkpoint.
Shared history plus merge-commit promotion is the durable model.

### Squash source promotion PRs

Rejected. Swarm PRs are already squashed. Squashing the promotion would hide the
sequence of accepted swarm commits from the release/source repo.

## Follow-up Specs / Plans

[`SHIPLOG-SPEC-0011`](../specs/SHIPLOG-SPEC-0011-shiplog-swarm-cutover-contract.md)
and [`plans/shiplog-swarm/implementation-plan.md`](../../plans/shiplog-swarm/implementation-plan.md)
now hold the accepted contract and receipts for:

- repaired shared history before normal swarm work started;
- the routed `Shiplog Rust Small Result` workflow in `shiplog-swarm`;
- same-repo PR, fallback, fork safety, and branch-protection proof;
- cutover instructions telling agents to clone `shiplog-swarm` side-by-side;
- merge-commit source promotions back into `EffortlessMetrics/shiplog`.
