# Shiplog Swarm Cutover Implementation Plan

## Current Preflight

Status: blocked on shared-history reseed
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011

Observed on 2026-05-21 from the release/source checkout:

```bash
git fetch origin main --prune --tags
git fetch git@github.com:EffortlessMetrics/shiplog-swarm.git main:refs/remotes/swarm/main --prune
git merge-base origin/main swarm/main
git log --oneline -1 swarm/main
```

Result:

```text
merge-base: none
swarm/main: 0873151 Initialize repository
```

Cutover must not proceed until `shiplog-swarm/main` is reseeded from
`shiplog/main` or otherwise repaired to share history.

## Work item: repair-shared-history

Status: ready
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011
Blocks: routed-rust-small-workflow
Blocked by: none
Branch: infra/reseed-shiplog-swarm
Issue:
PR:

### Goal

Make `shiplog-swarm/main` a shared-history import of `shiplog/main` before
normal development lands there.

### Production delta

Remote repository state only:

```text
EffortlessMetrics/shiplog-swarm main
```

### Non-goals

- No product behavior changes.
- No release tags, crates.io publish, GitHub Releases, or signing movement.
- No branch protection yet.
- No self-hosted runner access changes yet.

### Acceptance

- `git merge-base origin/main swarm/main` prints a commit.
- `shiplog-swarm/main` matches the intended `shiplog/main` checkpoint.
- Any existing swarm-only commits are intentionally preserved elsewhere or
  confirmed disposable before force update.

### Proof commands

```bash
git fetch origin main --prune --tags
git fetch swarm main --prune
git merge-base origin/main swarm/main
git diff --stat origin/main..swarm/main
```

### Rollback

Restore `shiplog-swarm/main` to the previous remote SHA if the reseed points at
the wrong source checkpoint.

### Claim boundary

This proves only shared history. It does not prove routed CI, branch protection,
runner safety, or cutover readiness.

## Work item: routed-rust-small-workflow

Status: ready
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011
Blocks: routed-ci-proof
Blocked by: repair-shared-history
Branch: ci/routed-shiplog-rust-small
Issue:
PR:

### Goal

Add one routed Linux CI lane to `shiplog-swarm`:

```text
Shiplog Rust Small Result
```

### Production delta

Add a `shiplog-swarm` workflow with conditional implementation jobs:

```text
Route Shiplog Rust Small
Shiplog Rust Small on CX43
Shiplog Rust Small on CX53
Shiplog Rust Small on GitHub Hosted
Shiplog Rust Small Result
```

### Non-goals

- No Windows/macOS matrix expansion.
- No release/publish/signing workflow move.
- No fork PRs on self-hosted runners.
- No branch protection before proof.

### Acceptance

- Same-repo PRs may route to trusted self-hosted runners.
- Fork PRs do not execute on self-hosted runners.
- GitHub-hosted fallback runs the same proof as the self-hosted route.
- The result job succeeds only when the selected implementation job succeeds.

### Proof commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked -- --test-threads=4
cargo xtask check-no-panic-family --mode blocking-allowlist
git diff --check
```

### Rollback

Revert the workflow PR in `shiplog-swarm`.

### Claim boundary

This proves workflow shape, not final cutover or branch protection.

## Work item: routed-ci-proof

Status: ready
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011
Blocks: branch-protection-enable
Blocked by: routed-rust-small-workflow
Branch:
Issue:
PR:

### Goal

Record manual, PR, fallback, and fork-admission proof for the routed lane.

### Acceptance

- Manual dispatch on `shiplog-swarm/main` passes.
- A tiny same-repo PR passes.
- CX43 busy routes to CX53 when enabled, or to GitHub-hosted if CX53 is
  intentionally skipped.
- All enabled self-hosted runners busy routes to GitHub-hosted.
- Fork PRs stay off self-hosted runners.
- Result output includes router target, reason, repo, workflow, and run ID.

### Proof commands

```bash
gh run list --repo EffortlessMetrics/shiplog-swarm --workflow "EM CI Routed Shiplog Rust" --limit 10
gh pr checks --repo EffortlessMetrics/shiplog-swarm <proof-pr>
git diff --check
```

### Rollback

Leave branch protection disabled and keep normal development on
`EffortlessMetrics/shiplog` until proof is complete.

### Claim boundary

This proves routed CI behavior only. It does not move release authority.

## Work item: branch-protection-enable

Status: ready
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011
Blocks: machine-cutover
Blocked by: routed-ci-proof
Branch:
Issue:
PR:

### Goal

Enable branch protection on `shiplog-swarm/main` requiring only:

```text
Shiplog Rust Small Result
```

### Acceptance

- Conditional implementation jobs are not required checks.
- Auto-merge and squash merge are compatible with the required result.
- A tiny post-protection same-repo PR passes and can squash-merge.

### Proof commands

```bash
gh api repos/EffortlessMetrics/shiplog-swarm/branches/main/protection
gh pr checks --repo EffortlessMetrics/shiplog-swarm <post-protection-pr>
```

### Rollback

Disable the branch protection rule or remove the required status check.

### Claim boundary

This proves branch protection only. It does not authorize release work from
`shiplog-swarm`.

## Work item: machine-cutover

Status: ready
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011
Blocks: promotion-cadence
Blocked by: branch-protection-enable
Branch:
Issue:
PR:

### Goal

Move normal agent and maintainer development to `shiplog-swarm`.

### Acceptance

Machine instructions say:

```text
Old repo:
  EffortlessMetrics/shiplog

New normal development repo:
  EffortlessMetrics/shiplog-swarm

Clone shiplog-swarm side-by-side.
Do not retarget old shiplog clones in place.
Do not push directly to main.
All new normal work uses PRs into shiplog-swarm/main.
Wait for Shiplog Rust Small Result.
Release/publish/signing remains on shiplog until explicit release cutover.
```

### Proof commands

```bash
git diff --check
```

### Rollback

Announce that normal development remains on `EffortlessMetrics/shiplog` and
close or retarget open swarm PRs.

### Claim boundary

This is a development cutover only, not a release cutover.

## Work item: promotion-cadence

Status: ready
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011
Blocks: none
Blocked by: machine-cutover
Branch: promote/swarm-YYYYMMDD-SHA
Issue:
PR:

### Goal

Promote `shiplog-swarm/main` into `shiplog/main` by merge-commit PRs.

### Acceptance

- Promotion PR title uses `merge(swarm): promote shiplog-swarm through <sha>`.
- Promotion PR is merged with a regular merge commit, not squash.
- PR body lists swarm head, included swarm PRs, and proof.
- Source-only release changes are synced back into `shiplog-swarm` before more
  normal development lands.

### Proof commands

```bash
git fetch origin main --prune --tags
git fetch swarm main --prune
git merge-base origin/main swarm/main
git log --oneline origin/main..swarm/main
gh pr create --base main --head promote/swarm-YYYYMMDD-SHA
```

### Rollback

Revert the promotion merge commit in `shiplog` and pause further promotions
until the divergence is understood.

### Claim boundary

Promotion keeps the release/source repo current. It still does not move release
authority to `shiplog-swarm`.
