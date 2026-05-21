# SHIPLOG-PROP-0010: Shiplog Swarm Development Landing Zone

Status: proposed
Owner: repo-infra/release
Created: 2026-05-20
Target release: post-0.9.0 development cutover
Follow-up spec:
[`SHIPLOG-SPEC-0011-shiplog-swarm-cutover-contract`](../specs/SHIPLOG-SPEC-0011-shiplog-swarm-cutover-contract.md)
Architecture decision:
[`SHIPLOG-ADR-0011-swarm-is-dev-landing-zone-not-release-surface`](../adr/SHIPLOG-ADR-0011-swarm-is-dev-landing-zone-not-release-surface.md)
Linked plan:
[`plans/shiplog-swarm/implementation-plan.md`](../../plans/shiplog-swarm/implementation-plan.md)

## Summary

Shiplog should get a dedicated swarm landing zone:

```text
source repo: EffortlessMetrics/shiplog
swarm repo:  EffortlessMetrics/shiplog-swarm
```

The swarm repo should become the development landing zone only after it shares
history with the source repo, the source repo is drained or checkpointed,
synced, and proven through one normalized routed CI result. During the
transition, `EffortlessMetrics/shiplog` remains the release surface, public
source checkpoint, tag/publish authority, and final sync source.

This is not a release cutover proposal. It is a repo-infrastructure proposal for
safe development routing.

## Problem

Shiplog has reached a 0.9 release shape with review-ready packet quality,
Guided Setup / Doctor, review-loop status, GitHub activity harvest, redaction
correctness, and release discipline. Moving development work into
`shiplog-swarm` before the repos share history and the source repo is drained
would create source divergence risk at the exact point where future release
readiness depends on a clean promotion path.

The runner side is less risky than the source-of-truth side. Shiplog is a Rust
repo that can start on a small routed lane with GitHub-hosted fallback. The
danger is not that routed CI cannot be made to run; the danger is that agents or
humans start landing work in two repositories without a final sync, a normalized
required check, and a clear rule for where release/publish/signing authority
lives.

## Target Users

Primary users:

- maintainers moving day-to-day shiplog development into a swarm landing zone;
- Codex, Droid, and other agents that need one repo to target for normal work;
- reviewers who need a single normalized result check instead of conditional
  runner implementation jobs;
- release operators who need release/publish/signing authority to remain stable
  until deliberate cutover.

Secondary users:

- infrastructure maintainers adding repositories to `em-ci-small`;
- maintainers proving CX43/CX53/GitHub-hosted fallback behavior;
- future repos copying the same public-swarm migration shape.

## Product And Repo End State

After cutover, normal development should target:

```text
EffortlessMetrics/shiplog-swarm main
```

The source repo should still own, until a later explicit release cutover:

```text
release tags
crates.io publish
GitHub Releases
release signing
release branches
public announcement workflows
Windows/macOS release proof
security-sensitive token workflows
```

The swarm repo should provide:

```text
public repository
same-repo PRs from trusted branches
routed Linux CI
GitHub-hosted fallback
fork PRs kept off self-hosted runners
one normalized required result check
deferred branch protection until the normalized result is proven
```

## Proposed Shape

Use the existing swarm migration pattern:

```text
EffortlessMetrics/shiplog -> EffortlessMetrics/shiplog-swarm
```

Create `shiplog-swarm` as a public repo, seed it from `shiplog/main` with shared
Git history, add it narrowly to the `em-ci-small` runner group, scope
`EM_RUNNER_READ_TOKEN` to that repository, and prove routed CI before branch
protection.

Before serious cutover, prove:

```bash
git merge-base origin/main swarm/main
```

If that command does not print a commit, `shiplog-swarm/main` is not ready for
promotion-merge operation and must be reseeded from `shiplog/main` before
swarm-only work starts.

Normal development PRs in `shiplog-swarm` should squash-merge into
`shiplog-swarm/main`. Promotion PRs from `shiplog-swarm/main` into
`shiplog/main` should use regular merge commits so the release/source repo keeps
a clear checkpoint of already-squashed swarm work. Do not squash promotion PRs.

The first routed lane should be:

```text
Shiplog Rust Small Result
```

Implementation jobs are conditional and should not be branch-protection checks:

```text
Route Shiplog Rust Small
Shiplog Rust Small on CX43
Shiplog Rust Small on CX53
Shiplog Rust Small on GitHub Hosted
Shiplog Rust Small Result
```

CX33 may be added later if it is attached, stable, and has enough disk for the
shiplog workload. Shiplog should not start as CX53-primary unless measured
runtimes prove that it needs the heavier route.

The base proof should mirror the current contributor checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked -- --test-threads=4
cargo xtask check-no-panic-family --mode blocking-allowlist
git diff --check
```

## Success Criteria

- `shiplog-swarm` is public, seeded from `shiplog/main` with shared history,
  and not hand-recreated.
- `shiplog/main` and `shiplog-swarm/main` have a common merge base before
  cutover.
- Normal development PRs squash in `shiplog-swarm`; source promotion PRs merge
  into `shiplog` with merge commits.
- `shiplog-swarm` has routed Rust small CI with one normalized result check.
- Same-repo swarm PRs can use self-hosted runners; fork PRs cannot.
- GitHub-hosted fallback runs the same logical proof as self-hosted lanes.
- Branch protection, when enabled, requires only `Shiplog Rust Small Result`.
- Release/publish/signing stays on `shiplog` until an explicit release cutover.
- Source repo PRs are drained or checkpointed before final sync.
- Agents are instructed to clone `shiplog-swarm` side-by-side after cutover
  instead of retargeting existing `shiplog` clones in place.

## Safety Boundaries

Do not:

- cut over before the live source PR queue is drained or checkpointed;
- cut over while `git merge-base origin/main swarm/main` is empty;
- give `shiplog` self-hosted runner access just to bridge the transition;
- run public fork PRs on self-hosted runners;
- require conditional implementation jobs in branch protection;
- move release tags, crates.io publish, signing, or announcement workflows;
- route Shiplog as CX53-primary without measured need;
- split CX53 into multiple runner services;
- intentionally queue on busy self-hosted runners;
- start release execution from this migration lane.

## Alternatives Considered

### Keep all development on `shiplog`

This avoids migration risk but keeps normal development mixed with release-hold
and public release authority. It also does not exercise the swarm landing-zone
pattern needed for agent-heavy work.

### Cut over immediately

Rejected because Shiplog is in active 0.9 release motion. A cutover before PR
drain/checkpoint and final sync risks source divergence.

### Give `shiplog` self-hosted runner access first

Rejected because the migration target is `shiplog-swarm`. Exposing the source
repo to self-hosted runners during the bridge phase widens the trust boundary
without proving the actual target repository.

### Make CX53 the primary route

Rejected for the initial lane. Shiplog should start as a small/medium Rust repo
and preserve CX53 for heavier repos unless measured timings justify promotion.

## Specs To Create Or Update

- `SHIPLOG-SPEC-0011-shiplog-swarm-cutover-contract`: required repo setup,
  runner admission, normalized result behavior, branch-protection rules, final
  sync, and cutover acceptance.
- Runner policy docs should define `Shiplog Rust Small Result` as the only
  branch-protection check after proof.

## Architecture Decisions Needed

- `SHIPLOG-ADR-0011-swarm-is-dev-landing-zone-not-release-surface`: decide that
  `shiplog-swarm` owns normal development after cutover while `shiplog` keeps
  release/publish/signing authority until deliberate release cutover.

## Implementation Campaign Shape

The follow-up implementation plan should be PR-sized and ordered:

```text
1. Create and seed shiplog-swarm from shiplog/main.
2. Prove common history between shiplog/main and shiplog-swarm/main.
3. Add the routed Shiplog Rust Small workflow.
4. Prove workflow PR behavior.
5. Run manual dispatch on shiplog-swarm/main.
6. Prove a tiny same-repo PR.
7. Prove CX43 -> CX53 -> GitHub fallback, or CX43 -> GitHub if simpler.
8. Drain or checkpoint source repo PRs.
9. Final-sync shiplog-swarm from shiplog/main.
10. Enable branch protection requiring only Shiplog Rust Small Result.
11. Move agent/machine instructions to fresh shiplog-swarm clones.
```

## Evidence Plan

Required proof should include:

- router target, router reason, repo, workflow, and run id in result output;
- one selected implementation job succeeds while the other conditional jobs
  skip;
- fork PRs do not run on self-hosted runners;
- same-repo PRs run automatically;
- manual dispatch on synced `shiplog-swarm/main` passes;
- fallback proof for the selected route order;
- branch protection requires only the normalized result check;
- release/publish/signing remains absent from swarm cutover proof.

## Risks

- Source divergence if work lands in both repos before final sync.
- Hidden release authority transfer if release workflows are copied too early.
- Branch protection deadlocks if conditional runner jobs are required.
- Self-hosted runner exposure if public fork PRs are not gated correctly.
- CX53 contention if Shiplog is routed as heavy before timing evidence exists.

## Non-Goals

- OAuth, dashboards, TUI, or agent framework work.
- Moving crates.io publish, signing, release tags, or GitHub Releases.
- Moving Windows/macOS release proof to self-hosted runners.
- Replacing existing 0.9 release readiness work.
- Executing release or publish operations.
- Implementing branch protection before routed CI proof.

## Exit Criteria

This proposal is done when the follow-up spec, ADR, and implementation plan
exist and the repo has an explicit decision about when `shiplog-swarm` becomes
the development target. It does not itself execute the cutover.
