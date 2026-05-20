# SHIPLOG-SPEC-0010: Shiplog Swarm Cutover Contract

Status: proposed
Owner: repo-infra/release
Created: 2026-05-20
Related proposal:
[`SHIPLOG-PROP-0008-shiplog-swarm-dev-landing-zone`](../proposals/SHIPLOG-PROP-0008-shiplog-swarm-dev-landing-zone.md)
Planned ADR:
`SHIPLOG-ADR-0011-swarm-is-dev-landing-zone-not-release-surface`
Planned implementation plan:
`plans/shiplog-swarm/implementation-plan.md`

## Purpose

This spec defines the contract for moving normal shiplog development to a swarm
landing zone without moving release authority or exposing self-hosted runners to
untrusted code.

The target split is:

```text
source repo: EffortlessMetrics/shiplog
swarm repo:  EffortlessMetrics/shiplog-swarm
```

After cutover, normal development targets `shiplog-swarm/main`. During and after
the initial cutover, `shiplog` remains the release surface until a later
explicit release cutover says otherwise.

This spec does not create the swarm repo, configure runners, enable branch
protection, move machines, or execute release work. It defines the behavior that
the follow-up implementation plan must prove.

## Scope

This spec owns:

- repository roles before and after cutover;
- allowed and disallowed runner exposure;
- the first routed CI lane for shiplog swarm work;
- the normalized result check and branch-protection rule;
- final source sync and source-divergence controls;
- fork PR and same-repo PR admission rules;
- proof expectations for routing, fallback, cleanup, and release authority.

Out of scope:

- release tags, crates.io publish, GitHub Releases, signing, or announcement
  workflows;
- moving Windows/macOS release proof to self-hosted runners;
- OAuth, dashboards, TUI, scheduler, new adapters, or LLM summaries;
- changing product behavior in `shiplog`;
- running public fork PRs on self-hosted runners;
- enabling branch protection before the normalized result is proven.

## Repo Roles

Until a later release cutover, `EffortlessMetrics/shiplog` owns:

```text
release tags
crates.io publish
GitHub Releases
release signing
release branches
public announcement workflows
Windows/macOS release proof
security-sensitive token workflows
final sync source for shiplog-swarm
```

After swarm development cutover, `EffortlessMetrics/shiplog-swarm` owns:

```text
normal development PRs
same-repo trusted agent work
routed Linux CI proof
one normalized required result check
post-cutover development main
```

`shiplog-swarm` must be seeded from `shiplog/main`. It must not be
hand-recreated.

## Source-Divergence Boundary

The cutover must not happen while source work is still ambiguous.

Before final sync:

```text
source PRs are merged, closed, or explicitly checkpointed
release-bound source work is identified
new development work stops targeting shiplog
shiplog-swarm is resynced from shiplog/main
routed CI passes on synced shiplog-swarm/main
```

After final sync:

```text
new normal development targets shiplog-swarm
agents clone shiplog-swarm side-by-side
agents do not retarget existing shiplog clones in place
shiplog remains release authority until explicit release cutover
```

## Runner Access Contract

`shiplog-swarm` may be added to a small Rust runner group such as
`em-ci-small`. Repository access and router token access must be scoped narrowly
to `shiplog-swarm` for this lane.

Do not give `shiplog` self-hosted runner access as a bridge unless a later spec
changes the migration boundary.

Public fork PRs must not run on self-hosted runners. They may route to
GitHub-hosted fallback or to an explicit safe skipped result.

Same-repo PRs from trusted `shiplog-swarm` branches may use the self-hosted
route once the workflow is present.

## Routed Lane

The first routed lane is:

```text
Shiplog Rust Small Result
```

Implementation jobs are conditional and must not be branch-protection checks:

```text
Route Shiplog Rust Small
Shiplog Rust Small on CX43
Shiplog Rust Small on CX53
Shiplog Rust Small on GitHub Hosted
Shiplog Rust Small Result
```

CX33 may be added later only after it is attached, stable, and proven to have
enough disk for shiplog. Shiplog must not start as CX53-primary unless measured
runtimes prove it needs that route.

Initial route order:

```text
CX43 -> CX53 -> GitHub Hosted
```

An even simpler first burn-in route is acceptable:

```text
CX43 -> GitHub Hosted
```

The spec does not require CX53 fallback on day one if runner policy chooses to
preserve CX53 for heavier repositories.

## Required Proof Commands

Every selected route must run the same logical proof:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked -- --test-threads=4
cargo xtask check-no-panic-family --mode blocking-allowlist
git diff --check
```

The GitHub-hosted fallback must not run a weaker proof than the self-hosted
route.

## Result Contract

The normalized result job must print:

```text
router_target
router_reason
repo=shiplog-swarm
workflow=EM CI Routed Shiplog Rust
run_id
```

When available, routed proof should also report:

```text
fallback count
runtime
disk before/after
sccache hit rate
cleanup failures
```

The result job succeeds only when the selected implementation job succeeds:

| Router target | Required implementation result |
| --- | --- |
| `cx43` | `Shiplog Rust Small on CX43` succeeds |
| `cx53` | `Shiplog Rust Small on CX53` succeeds |
| `github` | `Shiplog Rust Small on GitHub Hosted` succeeds |

Skipped implementation jobs are expected and must not block the normalized
result.

## Branch Protection Contract

Branch protection must be deferred until routed CI is proven.

After proof, branch protection may require exactly:

```text
Shiplog Rust Small Result
```

It must not require conditional implementation jobs, because skipped jobs are
part of the routing model.

Additional required checks such as policy, no-panic, release proof, or Windows
matrix lanes may be added only after separate proof and policy decisions.

## Validation Sequence

The implementation plan must prove the lane in this order:

1. Workflow PR in `shiplog-swarm`:
   - router job succeeds;
   - one implementation job runs and succeeds;
   - non-selected implementation jobs skip;
   - `Shiplog Rust Small Result` succeeds.
2. Manual dispatch on `shiplog-swarm/main`:
   - normalized result passes;
   - router target and reason are printed;
   - cleanup runs.
3. Tiny same-repo PR:
   - workflow runs automatically;
   - self-hosted route is allowed for trusted same-repo branches;
   - normalized result passes.
4. Fallback proof:
   - if CX53 is enabled, CX43 busy routes to CX53;
   - if all enabled self-hosted runners are busy, route to GitHub-hosted;
   - normalized result passes in each route.
5. Fork PR proof:
   - public fork code does not execute on self-hosted runners;
   - fallback or explicit safe result is visible.
6. Final source sync:
   - source PRs are drained or checkpointed;
   - `shiplog-swarm/main` is synced from `shiplog/main`;
   - routed CI passes on synced `shiplog-swarm/main`.

Branch protection is allowed only after this sequence is recorded.

## Cutover Instructions Contract

Machine and agent instructions must say:

```text
Old repo:
  EffortlessMetrics/shiplog

New normal development repo:
  EffortlessMetrics/shiplog-swarm

Clone shiplog-swarm side-by-side.
Do not retarget existing shiplog clones in place.
Do not push directly to main.
All new normal work uses PRs into shiplog-swarm/main.
Wait for Shiplog Rust Small Result.
Release/publish/signing remains on shiplog until explicit release cutover.
```

## Failure Modes

The cutover must fail closed when:

- `shiplog-swarm` is not a clean import of `shiplog/main`;
- source PRs are neither drained nor checkpointed;
- fork PRs can reach self-hosted runners;
- branch protection requires conditional implementation jobs;
- the normalized result passes without the selected implementation job passing;
- fallback runs weaker proof than the self-hosted route;
- release/publish/signing workflows are moved without a release-cutover decision;
- agents are instructed to push directly to `shiplog-swarm/main`.

## Acceptance Criteria

- The swarm repo is public and seeded from `shiplog/main`.
- Runner and router token access are scoped to `shiplog-swarm`.
- Same-repo PRs and fork PRs have separate admission behavior.
- `Shiplog Rust Small Result` is the only initial branch-protection check.
- Route/fallback/manual/tiny-PR/fork-PR proof is recorded before protection.
- Source PR queue is drained or explicitly checkpointed before final sync.
- Release authority stays on `shiplog` until a later explicit release cutover.

## Proof Mapping

Future implementation PRs should link:

- the routed workflow file in `shiplog-swarm`;
- the manual dispatch run;
- the tiny same-repo PR run;
- the fallback proof runs;
- the fork PR safety proof;
- the branch-protection settings after proof;
- the final source-sync receipt;
- the cutover instruction update.

This spec has no direct code proof. It is validated by `git diff --check` until
the planned implementation artifacts exist.
