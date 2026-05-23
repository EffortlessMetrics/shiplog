# SHIPLOG-SPEC-0011: Shiplog Swarm Cutover Contract

Status: accepted
Owner: repo-infra/release
Created: 2026-05-20
Related proposal:
[`SHIPLOG-PROP-0010-shiplog-swarm-dev-landing-zone`](../proposals/SHIPLOG-PROP-0010-shiplog-swarm-dev-landing-zone.md)
Linked ADR:
[`SHIPLOG-ADR-0011-swarm-is-dev-landing-zone-not-release-surface`](../adr/SHIPLOG-ADR-0011-swarm-is-dev-landing-zone-not-release-surface.md)
Linked implementation plan:
[`plans/shiplog-swarm/implementation-plan.md`](../../plans/shiplog-swarm/implementation-plan.md)

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

This spec does not configure runners, enable branch protection, move machines,
or execute release work. It defines the behavior that the implementation plan
must prove.

Initial state observed while writing this spec: `EffortlessMetrics/shiplog-swarm`
was public, but `git merge-base origin/main swarm/main` did not return a commit
from the release checkout. Its `main` was an orphan `Initialize repository`
commit.

Follow-up receipt on 2026-05-21: `shiplog-swarm/main` was reseeded from
`shiplog/main`; both refs now point at
`48c0da1b9a5aeefe58a79c472a8c35d9590e3657`, and `git merge-base origin/main
swarm/main` prints that commit. Merge policy was also configured so normal
swarm PRs squash while source promotion can still use merge commits in
`shiplog`.

Acceptance receipt on 2026-05-23: the implementation plan now records shared
history repair, routed CI proof, fork admission proof, branch protection,
machine cutover instructions, and repeated merge-commit source promotions. The
contract is accepted as the current development-control-plane rule while
release authority remains on `EffortlessMetrics/shiplog`.

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

## History And Promotion Contract

`shiplog` and `shiplog-swarm` must share Git history before cutover:

```bash
git merge-base origin/main swarm/main
```

The command must print a commit. If it does not, `shiplog-swarm/main` must be
reseeded from `shiplog/main` before agents land normal work there.

Before cutover, direction is:

```text
shiplog/main -> shiplog-swarm/main
```

After cutover, direction is:

```text
shiplog-swarm/main -> shiplog/main
```

Normal development PRs into `shiplog-swarm/main` use squash merge. Promotion PRs
from `shiplog-swarm/main` into `shiplog/main` use regular merge commits. Do not
squash source-promotion PRs; the merge commit is the release/source checkpoint.

## Source-Divergence Boundary

The cutover must not happen while source work is still ambiguous.

Before final sync:

```text
source PRs are merged, closed, or explicitly checkpointed
release-bound source work is identified
new development work stops targeting shiplog
shiplog-swarm is resynced from shiplog/main
common history is proven
routed CI passes on synced shiplog-swarm/main
```

After final sync:

```text
new normal development targets shiplog-swarm
agents clone shiplog-swarm side-by-side
agents do not retarget existing shiplog clones in place
shiplog remains release authority until explicit release cutover
shiplog receives periodic promotion merge commits from shiplog-swarm
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
Shiplog Rust Small on CPX42
Shiplog Rust Small on CX43
Shiplog Rust Small on CX53
Shiplog Rust Small on GitHub Hosted
Shiplog Rust Small Result
```

Additional self-hosted routes may be added only after they are attached, stable,
and proven to have enough disk for shiplog. Shiplog must not start as
CX53-primary unless measured runtimes prove it needs that route.

Current route order after the CPX42 extension is:

```text
CPX42 -> CX43 -> CX53 -> GitHub Hosted
```

The initial burn-in allowed smaller route sets such as:

```text
CX43 -> CX53 -> GitHub Hosted
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
| `cpx42` | `Shiplog Rust Small on CPX42` succeeds |
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
   - the current self-hosted route order is exercised or explicitly forced;
   - busy enabled self-hosted runners fall through to the next route;
   - if all enabled self-hosted runners are busy, route to GitHub-hosted;
   - normalized result passes in each route.
5. Fork PR proof:
   - public fork code does not execute on self-hosted runners;
   - fallback or explicit safe result is visible.
6. Final source sync:
   - source PRs are drained or checkpointed;
   - `shiplog-swarm/main` is synced from `shiplog/main`;
   - `git merge-base origin/main swarm/main` prints a commit;
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
- `shiplog-swarm/main` and `shiplog/main` do not share a merge base;
- source PRs are neither drained nor checkpointed;
- fork PRs can reach self-hosted runners;
- branch protection requires conditional implementation jobs;
- the normalized result passes without the selected implementation job passing;
- fallback runs weaker proof than the self-hosted route;
- release/publish/signing workflows are moved without a release-cutover decision;
- agents are instructed to push directly to `shiplog-swarm/main`.

## Acceptance Criteria

- The swarm repo is public and seeded from `shiplog/main`.
- `git merge-base origin/main swarm/main` returns a commit before cutover.
- Normal development PRs squash in `shiplog-swarm`.
- Promotion PRs from `shiplog-swarm` into `shiplog` use merge commits.
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

This spec is enforced by the implementation-plan receipts and the active-goal
control-plane checks:

```bash
cargo xtask check-doc-artifacts
cargo xtask check-goals
cargo xtask repo-contract-report
git diff --check
```

Those checks do not replace live GitHub route or branch-protection inspection;
they keep the repo-native contract linked and current.
