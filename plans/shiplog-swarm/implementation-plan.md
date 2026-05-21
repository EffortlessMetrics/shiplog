# Shiplog Swarm Cutover Implementation Plan

## Current Preflight

Status: shared history repaired; branch protection enabled; machine cutover next
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011

Initial observation on 2026-05-21 from the release/source checkout:

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

Repair receipt on 2026-05-21:

```bash
git push --force-with-lease=refs/heads/main:08731519049bb30e9326ed33cfcc5aee7e8de767 \
  swarm origin/main:main
git fetch swarm main --prune
git merge-base origin/main swarm/main
git diff --stat origin/main..swarm/main
gh api repos/EffortlessMetrics/shiplog-swarm --jq '{allow_squash_merge,allow_merge_commit,allow_rebase_merge,allow_auto_merge,delete_branch_on_merge}'
```

Result:

```text
origin/main: 48c0da1b9a5aeefe58a79c472a8c35d9590e3657
swarm/main:  48c0da1b9a5aeefe58a79c472a8c35d9590e3657
merge-base:  48c0da1b9a5aeefe58a79c472a8c35d9590e3657
diff:        empty
merge policy: squash=true, merge=false, rebase=false, auto_merge=true, delete_branch_on_merge=true
```

Cutover still must not proceed until the protected PR path is proven and
machine cutover instructions are published.

## Work item: repair-shared-history

Status: done
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011
Blocks: routed-rust-small-workflow
Blocked by: none
Branch: none; remote-state operation
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

## Work item: configure-merge-policy

Status: done
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011
Blocks: routed-rust-small-workflow
Blocked by: none
Branch: none; remote-setting operation
Issue:
PR:

### Goal

Configure `shiplog-swarm` as a squash-merge development landing zone while
leaving `shiplog` able to accept promotion merge commits.

### Production delta

Remote repository settings:

```text
EffortlessMetrics/shiplog-swarm
EffortlessMetrics/shiplog
```

### Non-goals

- No branch protection.
- No release authority movement.
- No runner access changes.

### Acceptance

- `shiplog-swarm` allows squash merge.
- `shiplog-swarm` disables merge commits and rebase merge for normal PRs.
- `shiplog-swarm` enables auto-merge and delete-branch-on-merge.
- `shiplog` still allows merge commits for future promotion PRs.

### Proof commands

```bash
gh api repos/EffortlessMetrics/shiplog-swarm --jq '{allow_squash_merge,allow_merge_commit,allow_rebase_merge,allow_auto_merge,delete_branch_on_merge}'
gh api repos/EffortlessMetrics/shiplog --jq '{allow_merge_commit}'
```

### Rollback

Restore the previous `shiplog-swarm` merge settings:

```text
allow_merge_commit=true
allow_rebase_merge=true
allow_auto_merge=false
delete_branch_on_merge=false
```

### Claim boundary

This proves merge policy only. It does not prove routed CI or branch protection.

## Work item: routed-rust-small-workflow

Status: done
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011
Blocks: routed-ci-proof
Blocked by: none
Branch: ci/routed-shiplog-rust-small
Issue:
PR: EffortlessMetrics/shiplog-swarm#17

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

### Receipt

Landed in `shiplog-swarm` PR #17 on 2026-05-21.

```text
swarm/main: 8f01ae2e4b8a242f954136eecde319ed0c4fea81
pull_request run: 26214995888
post-merge push run: 26215587591
manual dispatch run: 26215622183
manual dispatch route: github
manual dispatch reason: no_idle_runner
manual dispatch result: Shiplog Rust Small Result passed
```

### Proof commands

```bash
cargo xtask check-workflows --mode blocking-allowlist
cargo xtask check-policy-schemas
git diff --check
```

The routed lane itself runs:

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

Status: done
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011
Blocks: branch-protection-enable
Blocked by: routed-rust-small-workflow
Branch: docs/swarm-routed-ci-proof-complete
Issue:
PR: EffortlessMetrics/shiplog-swarm#18, EffortlessMetrics/shiplog-swarm#16, EffortlessMetrics/shiplog-swarm#20, EffortlessMetrics/shiplog-swarm#22

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

### Current receipts

Manual dispatch on `shiplog-swarm/main` passed on 2026-05-21:

```text
workflow: EM CI Routed Shiplog Rust
run: 26215622183
route: github
reason: no_idle_runner
trusted: true
result: Shiplog Rust Small Result passed
```

Same-repo PR #18 proved that trusted pull requests can select self-hosted
runners:

```text
workflow: EM CI Routed Shiplog Rust
run: 26221468335
route: cx53
reason: cx53_idle
trusted: true
result: blocked; CX53 failed before Rust install because TMPDIR was created
        after the toolchain step
```

After moving scratch preparation before the Rust toolchain step, the same PR
passed through GitHub-hosted fallback:

```text
workflow: EM CI Routed Shiplog Rust
run: 26222097991
route: github
reason: no_idle_runner
trusted: true
result: Shiplog Rust Small Result passed
```

Forced CX53 proof passed on the same branch:

```text
workflow: EM CI Routed Shiplog Rust
run: 26222651751
route: cx53
reason: forced_cx53
result: Shiplog Rust Small on CX53 passed
normalized result: passed
```

Forced CX43 proof did not complete:

```text
workflow: EM CI Routed Shiplog Rust
run: 26222929499
route: cx43
reason: forced_cx43
result: Shiplog Rust Small on CX43 remained queued and was cancelled
normalized result: cancelled/failure after cancellation
```

Forced CX43 proof later reached the runner but failed the scratch preflight
before Rust/toolchain work:

```text
workflow: EM CI Routed Shiplog Rust
run: 26234698892
route: cx43
reason: forced_cx43
result: blocked; /mnt/ci-scratch had 97GB free and the CX43 guard required 100GB
normalized result: failed because selected cx43 job failed
```

Same-repo dependabot PR #16 gave a second normal `pull_request` proof after
the workflow fix and dependency compatibility patch:

```text
workflow: EM CI Routed Shiplog Rust
run: 26226906591
route: github
reason: no_idle_runner
trusted: true
result: Shiplog Rust Small Result passed
legacy CI: ubuntu, windows, policy, Droid, and smoke checks passed
```

The #16 squash merge also proved the `push` path on `shiplog-swarm/main`:

```text
workflow: EM CI Routed Shiplog Rust
run: 26227654459
swarm/main: 0b0afd4d23426b252b982d7f512bf4fdbcdd02d1
route: github
reason: no_idle_runner
trusted: true
result: Shiplog Rust Small Result passed
```

Temporary fork PR #20 proved that untrusted pull requests stay off
self-hosted runners and route to GitHub-hosted execution:

```text
workflow: EM CI Routed Shiplog Rust
run: 26231870924
fork: EffortlessSteven/shiplog-swarm
route: github
reason: untrusted_pr
trusted: false
cx43/cx53 jobs: skipped
github-hosted job: passed
normalized result: passed
disposition: closed without merge after proof capture
```

The #21 squash merge naturally selected CX53 on the `shiplog-swarm/main` push
path and passed:

```text
workflow: EM CI Routed Shiplog Rust
run: 26233051508
swarm/main: c48e459cbe916288d91758fa9eeb60ce567ed637
route: cx53
reason: cx53_idle
trusted: true
result: Shiplog Rust Small on CX53 passed
normalized result: passed
```

Same-repo PR #22 lowered only the CX43 scratch guard from 100GB to 90GB after
the forced CX43 failure found 97GB free. The normal pull request route then
selected CX43 and passed:

```text
workflow: EM CI Routed Shiplog Rust
run: 26234886542
route: cx43
reason: cx43_idle
trusted: true
result: Shiplog Rust Small on CX43 passed
normalized result: passed
```

The #22 squash merge also refreshed the `shiplog-swarm/main` push fallback
proof:

```text
workflow: EM CI Routed Shiplog Rust
run: 26235704712
swarm/main: ad2776b403fe694ee165a86c6b629559e33617fc
route: github
reason: no_idle_runner
trusted: true
result: Shiplog Rust Small on GitHub Hosted passed
normalized result: passed
```

The local GitHub CLI token cannot inspect org self-hosted runner state:

```text
gh api orgs/EffortlessMetrics/actions/runners?per_page=100
HTTP 403: runners and runner groups permission required
```

Same-repo PR, push fallback, CX43, CX53, and fork-admission proof now have green
receipts. Branch protection may proceed with only `Shiplog Rust Small Result`
as the required status check.

### Proof commands

```bash
gh run list --repo EffortlessMetrics/shiplog-swarm --workflow "EM CI Routed Shiplog Rust" --limit 10
gh pr checks --repo EffortlessMetrics/shiplog-swarm <proof-pr>
git diff --check
```

### Rollback

If routed proof regresses, leave branch protection disabled and keep normal
development on `EffortlessMetrics/shiplog` until the failed route is repaired.

### Claim boundary

This proves routed CI behavior only. It does not move release authority.

## Work item: branch-protection-enable

Status: done
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011
Blocks: machine-cutover
Blocked by: routed-ci-proof
Branch:
Issue:
PR: EffortlessMetrics/shiplog-swarm#24

### Goal

Enable branch protection on `shiplog-swarm/main` requiring only:

```text
Shiplog Rust Small Result
```

### Acceptance

- Conditional implementation jobs are not required checks.
- Auto-merge and squash merge are compatible with the required result.
- A tiny post-protection same-repo PR passes and can squash-merge.

### Receipt

Branch protection was enabled on `shiplog-swarm/main` after routed proof
completed:

```text
required_status_checks.contexts: ["Shiplog Rust Small Result"]
required_status_checks.strict: true
enforce_admins: false
allow_deletions: false
allow_force_pushes: false
```

This PR is the protected same-repo PR proof. It must pass
`Shiplog Rust Small Result` before squash merge.

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

Status: active
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
