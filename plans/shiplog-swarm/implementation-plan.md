# Shiplog Swarm Cutover Implementation Plan

Plan artifact: SHIPLOG-PLAN-0011

## Current Preflight

Status: shared history repaired; branch protection enabled; normal development cutover documented; promotion cadence active; branch hygiene clean
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

Normal development has cut over to `shiplog-swarm`. Keep release authority on
`EffortlessMetrics/shiplog`, and keep normal work flowing through focused swarm
PRs plus merge-commit source promotions.

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

Add a `shiplog-swarm` workflow with conditional implementation jobs. The
current workflow includes CPX42, added after the initial lane landed:

```text
Route Shiplog Rust Small
Shiplog Rust Small on CPX42
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
self-hosted jobs: skipped
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

Same-repo PR #43 later found that 90GB was still too brittle for normal routing:

```text
workflow: EM CI Routed Shiplog Rust
run: 26282622550
route: cx43
reason: cx43_idle
result: blocked; /mnt/ci-scratch had 87GB free and the CX43 guard required 90GB
normalized result: failed because selected cx43 job failed
```

The CX43 scratch guard was lowered to 80GB to keep the route usable while still
requiring a large clean scratch budget before Rust work starts.

Forced CX43 proof on the PR branch then passed end to end with the lower guard:

```text
workflow: EM CI Routed Shiplog Rust
run: 26283549809
route: cx43
reason: forced_cx43
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

The routed lane was later extended through `shiplog-swarm` PR #25 so the
current route order is:

```text
CPX42 -> CX43 -> CX53 -> GitHub Hosted
```

The #25 PR and post-merge push both selected CPX42 and passed the normalized
result:

```text
PR run:         26241254974
post-merge run: 26241904165
route:          cpx42
result:         Shiplog Rust Small Result passed
```

The route contract was later refreshed to match the live CPX42 workflow:

```text
swarm PR:      EffortlessMetrics/shiplog-swarm#53
source PR:     EffortlessMetrics/shiplog#495
swarm head:    b531e007fac6c4a54bc4801d5bb3d15d5b06e72d
source merge:  ba61fbd95b24f36dca44c1e52c00b24b681b585d
result:        regular merge commit; source and swarm checks passed
```

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

Status: done
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011
Blocks: promotion-cadence
Blocked by: branch-protection-enable
Branch: docs/swarm-machine-cutover
Issue:
PR: EffortlessMetrics/shiplog-swarm#27

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

### Receipt

Machine and agent instructions are published in `AGENTS.md`.

The instruction surface says:

```text
normal development target: EffortlessMetrics/shiplog-swarm
source/release surface:    EffortlessMetrics/shiplog
normal swarm PRs:          squash merge
source promotion PRs:      regular merge commit
required swarm check:      Shiplog Rust Small Result
```

The source promotion model has already been exercised:

```text
source PR:      EffortlessMetrics/shiplog#469
swarm head:     3c318dbf661f0aa5fa1c9d0de3ebc2385441e04b
source merge:   b21636703b3cd89113f75532312308772a10e5d5
merge method:   regular merge commit
post-merge CI:  EM CI Routed Shiplog Rust passed; CI passed
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

Status: active
Linked proposal: SHIPLOG-PROP-0010
Linked spec: SHIPLOG-SPEC-0011
Linked ADR: SHIPLOG-ADR-0011
Blocks: none
Blocked by: none
Branch: promote/swarm-YYYYMMDD-SHA
Issue:
PR: EffortlessMetrics/shiplog-swarm#28

### Goal

Promote `shiplog-swarm/main` into `shiplog/main` by merge-commit PRs.

### Acceptance

- `plans/shiplog-swarm/promotion-runbook.md` defines the repeatable promotion
  path.
- Promotion PR title uses `merge(swarm): promote shiplog-swarm through <sha>`.
- Promotion PR is merged with a regular merge commit, not squash.
- PR body lists swarm head, included swarm PRs, swarm proof, and source proof.
- Source-only release changes are synced back into `shiplog-swarm` before more
  normal development lands.
- Receipt-refresh PRs record the latest completed source promotion available at
  the time they are authored; subsequent receipt-refresh promotions are tracked
  by their GitHub PR bodies until the next substantive swarm PR refreshes this
  ledger. This avoids an infinite loop of receipt-only updates.

### Receipt

The promotion model is active and has been exercised repeatedly:

```text
EffortlessMetrics/shiplog#469:
  swarm head: 3c318dbf661f0aa5fa1c9d0de3ebc2385441e04b
  source merge: b21636703b3cd89113f75532312308772a10e5d5
  result: regular merge commit; source post-merge routed CI and CI passed

EffortlessMetrics/shiplog#470:
  swarm head: aa4393b2c6eb9812bead86671353c32eabaa78c2
  source merge: 138b1237cce3468941b702fa433aacc70d6e0234
  result: regular merge commit; source post-merge routed CI and CI passed

EffortlessMetrics/shiplog#480:
  swarm head: c14276194965d33e0741c4ff6951105f08b33835
  source merge: 95a9ff41cd8ad1f3bcf5269342bf2437d89c2d69
  result: regular merge commit; source post-merge routed CI, CI, smoke,
          security, testing, and CI Actuals passed

EffortlessMetrics/shiplog#481:
  swarm head: f98bcb3ef4a28f56c095243276fd290f0d510645
  included swarm PRs: EffortlessMetrics/shiplog-swarm#38
  source merge: 930e134ee8cbf63c41d2d21eee0f0b95eeacc33b
  result: regular merge commit; source post-merge routed CI, CI, and
          CI Actuals passed

EffortlessMetrics/shiplog#482:
  swarm head: a7946243c2bbaf258ff0e36ce6a0c93acc866c70
  included swarm PRs: EffortlessMetrics/shiplog-swarm#39
  source merge: 1d27157bc7d76ce88f60e8b09c6a98bfc88d1196
  result: regular merge commit; source post-merge routed CI, CI, and
          CI Actuals passed

EffortlessMetrics/shiplog#483:
  swarm head: f5364f35e087299411bba0d572de85045069a746
  included swarm PRs: EffortlessMetrics/shiplog-swarm#40,
                       EffortlessMetrics/shiplog-swarm#41
  source merge: b6bd900c719cf7007fa53e04310517c8a6b953ad
  result: regular merge commit; source post-merge routed CI, CI, smoke,
          security, testing, coverage, and CI Actuals passed

EffortlessMetrics/shiplog#484:
  swarm head: 07316de0afb06ce4a1662263fe1d921ad8ee41e5
  included swarm PRs: EffortlessMetrics/shiplog-swarm#42
  source merge: 72aa89f75448ab93a361ef8810310ad176857b43
  result: regular merge commit; source post-merge routed CI, CI, and
          CI Actuals passed

EffortlessMetrics/shiplog#485:
  swarm head: 759562919a5475a4ba1cfbd8821b1a90d0a54e71
  included swarm PRs: EffortlessMetrics/shiplog-swarm#43
  source merge: 31bbbe9e50cd21c7caf9973b9978b811e2df0eae
  result: regular merge commit; source post-merge routed CI, CI, and
          CI Actuals passed

EffortlessMetrics/shiplog#486:
  swarm head: 1214b1c08db5ac6a974ec516aa20ed8a41813fbd
  included swarm PRs: EffortlessMetrics/shiplog-swarm#44
  source merge: fabfaf1586998eeabb35f6f1402bd8477bd5037e
  result: regular merge commit; source post-merge routed CI, CI, smoke,
          security, testing, and CI Actuals passed

EffortlessMetrics/shiplog#487:
  swarm head: e32cca5de8b2a281c3db53d5486cd9c885af6315
  included swarm PRs: EffortlessMetrics/shiplog-swarm#45
  source merge: 7cdce7b0229c573cbc754ed1e9b7cb9097261781
  result: regular merge commit

EffortlessMetrics/shiplog#488:
  swarm head: c6d837dca1d81bfab8d7a4667d0dd211a85a5e00
  included swarm PRs: EffortlessMetrics/shiplog-swarm#46
  source merge: c8565f04641a1215b6e38d903dfe121386c0e584
  result: regular merge commit

EffortlessMetrics/shiplog#489:
  swarm head: 7d3f11b7852c10dad0085293f39b0724c0b6d1e6
  included swarm PRs: EffortlessMetrics/shiplog-swarm#47
  source merge: c73484018a30cfe669d72449304611d15ce3b779
  result: regular merge commit

EffortlessMetrics/shiplog#490:
  swarm head: a01d2e4a8948b69e5a91a32b2dc44ce387ff9cdf
  included swarm PRs: EffortlessMetrics/shiplog-swarm#48
  source merge: 2baa15015017e7b34e7445291294f7bff6447466
  result: regular merge commit

EffortlessMetrics/shiplog#491:
  swarm head: 93a31d21b924cdac31cf5aaf67f92ab4838dc5d9
  included swarm PRs: EffortlessMetrics/shiplog-swarm#49
  source merge: 9d6ccbb2996aa023af076f95e693d0c62347d095
  result: regular merge commit; source identity validator landed

EffortlessMetrics/shiplog#492:
  swarm head: 62b769cd5dd98d830fc87d0e852284f20984fd26
  included swarm PRs: EffortlessMetrics/shiplog-swarm#50
  source merge: 5886ae2c3c2bc31e84fa1f8dda2f26fbf37e8581
  result: regular merge commit; CacheLookup stale-reporting receipt refreshed

EffortlessMetrics/shiplog#493:
  swarm head: f22e9c0fd64d1841a1e5018caa55264541ede364
  included swarm PRs: EffortlessMetrics/shiplog-swarm#51
  source merge: c80b05aa2d7b107cc6feb8f80a920d85902d4ded
  result: regular merge commit; source post-merge routed CI, CI, smoke,
          security, testing, coverage, and CI Actuals passed
  queue cleanup: source issues #205, #203, #201, #199, #197, #195, #193,
                 #191, and #189 closed as completed by existing merged PRs

EffortlessMetrics/shiplog#494:
  swarm head: f27c0f3a0b0e494b531d7efc2775f5a49619fa4a
  included swarm PRs: EffortlessMetrics/shiplog-swarm#52
  source merge: 57a76691e91a2e6625cab26665e59ba7da9601d6
  result: regular merge commit; clean-queue promotion receipts refreshed

EffortlessMetrics/shiplog#495:
  swarm head: b531e007fac6c4a54bc4801d5bb3d15d5b06e72d
  included swarm PRs: EffortlessMetrics/shiplog-swarm#53
  source merge: ba61fbd95b24f36dca44c1e52c00b24b681b585d
  result: regular merge commit; CPX42 route contract aligned with live
          workflow; source post-merge routed CI, CI, smoke, security, testing,
          and CI Actuals passed

EffortlessMetrics/shiplog#496:
  swarm head: 6c3eea01154c07ac431a3f1ce12b1cec4e29107c
  included swarm PRs: EffortlessMetrics/shiplog-swarm#54
  source merge: 6528b9d95599f3e0e8cdd05ea09a0863b6d9adf5
  result: regular merge commit; user-polish plan archived as completed;
          source post-merge routed CI, CI, smoke, security, testing, and
          CI Actuals passed

EffortlessMetrics/shiplog#497:
  swarm head: 10c16780479914b1ee84f6d750c3ef622947f9b4
  included swarm PRs: EffortlessMetrics/shiplog-swarm#55
  source merge: 386a5e0ea177d4565a257d3cbaf58ffa91d4c7d3
  result: regular merge commit; promotion ledger refreshed; source post-merge
          routed CI and CI passed

EffortlessMetrics/shiplog#498:
  swarm head: cbcd866adfe8f64ab476cb254e593bec86015807
  included swarm PRs: EffortlessMetrics/shiplog-swarm#56
  source merge: ecdd4d98e543857fb0f893043efe4be9439a3230
  result: regular merge commit; repo contract report now includes
          source/swarm git topology; source post-merge routed CI, CI, smoke,
          security, BDD/property testing, and CI Actuals passed

EffortlessMetrics/shiplog#499:
  swarm head: 8c502812b75778ddd6cc4a4844fd163996cea05a
  included swarm PRs: EffortlessMetrics/shiplog-swarm#57
  source merge: 84485cc17102a8282e9a9b903efe9c1307184fb2
  result: regular merge commit; promotion receipts refreshed; source
          post-merge routed CI, CI, smoke, security, testing, and CI Actuals
          passed

EffortlessMetrics/shiplog#500:
  swarm head: cced6c84bdd939e9f5c2212d08bdf992c4fbb064
  included swarm PRs: EffortlessMetrics/shiplog-swarm#58
  source merge: 426433fea87f4d9e3773b0ea4a9e6f59c92dc384
  result: regular merge commit; repo contract report classifies source-ahead
          promotion merge commits separately from source-only drift; source
          post-merge routed CI, CI, smoke, security, testing, and CI Actuals
          passed

EffortlessMetrics/shiplog#501:
  swarm head: f527febd61e1fc4ce83ff995a201a0594370d783
  included swarm PRs: EffortlessMetrics/shiplog-swarm#59
  source merge: bee7d0095a931025557717b654617d7694c82c8e
  result: regular merge commit; repo contract report classifies the full
          source-ahead promotion range; swarm PR routed through CX53 and
          source post-merge routed CI, CI, smoke, security, testing, and
          CI Actuals passed

EffortlessMetrics/shiplog#502:
  swarm head: e475fa8018deba7d020b81daa4017eb13d9cefd7
  included swarm PRs: EffortlessMetrics/shiplog-swarm#60
  source merge: f1f714e19cc07c134762f8e125d94cc71e173847
  result: regular merge commit; source-of-truth stack artifacts accepted;
          source post-merge routed CI, CI, smoke, security, testing, and
          CI Actuals passed

EffortlessMetrics/shiplog#503:
  swarm head: 96d1108cec3e3aa51ebe709abd6568cd00342bda
  included swarm PRs: EffortlessMetrics/shiplog-swarm#61
  source merge: 4f75a0a5689bb93361442dd2183b24624449f8ac
  result: regular merge commit; source-of-truth rollout accepted in the
          artifact ledger; source post-merge routed CI and CI passed

EffortlessMetrics/shiplog#504:
  swarm head: 1c97e8a8e2d1820688f73529bde242946eacc058
  included swarm PRs: EffortlessMetrics/shiplog-swarm#62
  source merge: def4967cbcaf7c91fab41b80a6f574a7e9cef54b
  result: regular merge commit; support-tier proof commands made copyable and
          validated; source post-merge routed CI and CI passed

EffortlessMetrics/shiplog#505:
  swarm head: 55dae9fc3ca8d3739e4fafc1a10f6346f972e084
  included swarm PRs: EffortlessMetrics/shiplog-swarm#63
  source merge: 9db8cd813a4e7de70a429d25a1fee6a53d4f3d50
  result: regular merge commit; workflow checker proof command docs were
          aligned with `cargo xtask check-workflows`; source post-merge routed
          CI and CI passed

EffortlessMetrics/shiplog#507:
  swarm head: f4fc2d5ba9172de1b313fbeebb33c5674a3dfea5
  included swarm PRs: EffortlessMetrics/shiplog-swarm#64
  source merge: 1a35a90d822ddc42b268443f6fdf22a57a19d8ff
  result: regular merge commit; `serde_json` bumped to 1.0.150 through the
          swarm development trunk; source-side Dependabot PR #506 was closed as
          superseded by the swarm PR plus source promotion; source post-merge
          routed CI and CI passed

EffortlessMetrics/shiplog#508:
  swarm head: 3c706389b518ddc68fa1888df35ab8e8185b0bd8
  included swarm PRs: EffortlessMetrics/shiplog-swarm#65
  source merge: 6de8fe4437866881b1e0a518fd8805ff2aaa2373
  result: regular merge commit; `repo-contract-report` now classifies both
          GitHub default promotion merge subjects and explicit
          `merge(swarm): promote shiplog-swarm through <sha>` subjects as
          promotion merges; source post-merge routed CI and CI passed

EffortlessMetrics/shiplog#509:
  swarm head: f0ca11a04ec59597d2470bde99ef3479fd224b5e
  included swarm PRs: EffortlessMetrics/shiplog-swarm#66
  source merge: b1cdf2f970a80d46bd1e29e8dee71ffb306f5e40
  result: regular merge commit; active goal proof commands now require `rtk`
          prefixes; source post-merge routed CI and CI passed

EffortlessMetrics/shiplog#510:
  swarm head: cf83e7206fbbd2671912a2cb51d7106141aae9ed
  included swarm PRs: EffortlessMetrics/shiplog-swarm#67
  source merge: d8cb7b4037eb06eeb23fb6387dd0d57ffe669ec2
  result: regular merge commit; PR body generation now prefers active goal
          proof commands; source post-merge routed CI and CI passed

EffortlessMetrics/shiplog#511:
  swarm head: 4425bbc2c8a1199a39302e8c7b5631ce09c030da
  included swarm PRs: EffortlessMetrics/shiplog-swarm#68
  source merge: 727ed24a8896db219ec7357fdf0c2f0478c648fe
  result: regular merge commit; support-tier proof commands now require
          copyable `rtk cargo xtask` prefixes; source post-merge routed CI, CI,
          smoke, security, testing, and CI Actuals passed

EffortlessMetrics/shiplog#513:
  swarm head: a51cf13068f8b410ca9844617242f72a38b0a7b1
  included swarm PRs: EffortlessMetrics/shiplog-swarm#70
  source merge: 740b2cfc839356ecd3bc88f416c4bbe60a0801c6
  result: regular merge commit; repo contract report Markdown now includes
          work item proof commands; source post-merge routed CI, CI, smoke,
          security, testing, and CI Actuals passed

EffortlessMetrics/shiplog#514:
  swarm head: bb243dee4e82651b64ee2938d2a18749a2dfbb3b
  included swarm PRs: EffortlessMetrics/shiplog-swarm#71
  source merge: c1a6d412448b5cbc7ee7d3952da5095f43d4f3e0
  result: regular merge commit; PR body generation now includes plan
          acceptance criteria and compact active-goal receipt refs; source
          post-merge routed CI, CI, smoke, security, testing, and CI Actuals
          passed

EffortlessMetrics/shiplog#515:
  swarm head: bd0610d302883d78cc938a8d1c6ad31c8cf3a922
  included swarm PRs: EffortlessMetrics/shiplog-swarm#72
  source merge: 02eb5b602436c86f0332948217912add7edfddeb
  result: regular merge commit; repo contract report Markdown now includes
          active goal objective and end-state context; source post-merge
          routed CI, CI, smoke, security, testing, and CI Actuals passed

EffortlessMetrics/shiplog#516:
  swarm head: 197f0d23154c63c21fc98237040eafc2ba4dd0e4
  included swarm PRs: EffortlessMetrics/shiplog-swarm#73
  source merge: 4d437db2dc30a39764f0804745429db3e6e3a52d
  result: regular merge commit; repo contract report Markdown now compacts
          long work-item receipt history while preserving full receipt data in
          JSON and the active goal manifest; source post-merge routed CI, CI,
          smoke, security, testing, and CI Actuals passed

EffortlessMetrics/shiplog#517:
  swarm head: b099bdb4a5e9e4aa135ebf8358738422f21e1d64
  included swarm PRs: EffortlessMetrics/shiplog-swarm#74
  source merge: cd54808b975585a5a6656b96997b99bd3ff7daca
  result: regular merge commit; repo contract report Markdown now compacts
          long promotion history while preserving full promotion data in JSON;
          source post-merge routed CI, CI, smoke, security, testing, and
          CI Actuals passed

EffortlessMetrics/shiplog#518:
  swarm head: 6f376c92cb44a5bac00ddb3cf6175b32600594d6
  included swarm PRs: EffortlessMetrics/shiplog-swarm#75
  source merge: 05a2d1a6ad56e00e6dd9819a9aae50eeb323453e
  result: regular merge commit; repo contract report JSON and Markdown now
          include git-topology next actions so agents can choose between normal
          swarm work, source promotion, or drift repair from repo evidence;
          source post-merge routed CI, CI, smoke, security, testing, and
          CI Actuals passed

EffortlessMetrics/shiplog#519:
  swarm head: 6147ab3843361f28617459c235248ef9eb0eec0f
  included swarm PRs: EffortlessMetrics/shiplog-swarm#76
  source merge: e4ac1c7315e7063c9777df0b9d825dc8031f0c65
  result: regular merge commit; repo contract report JSON and Markdown now
          include local checkout state so agents can see whether scoped work
          is clean before continuing; source post-merge routed CI, CI, smoke,
          security, testing, and CI Actuals passed

EffortlessMetrics/shiplog#520:
  swarm head: 1f23e19444344ed0152c38416eb931b490669362
  included swarm PRs: EffortlessMetrics/shiplog-swarm#77
  source merge: 5cf267b1778b09b3a7d404a29cc3b4c1d4ebc57f
  result: regular merge commit; repo contract report JSON and Markdown now
          include promotion receipt freshness so agents can see when source
          promotion and swarm PR receipts need refreshing; source post-merge
          routed CI, CI, smoke, security, testing, and CI Actuals passed

EffortlessMetrics/shiplog#521:
  swarm head: d68af2a687c346cc58081fb2d8dcff333d1d4124
  included swarm PRs: EffortlessMetrics/shiplog-swarm#78
  source merge: 34bcd6e9439edcc073f7f8f9dbbb53acfcd953f3
  result: regular merge commit; repo contract report JSON and Markdown now
          list stale promotion receipts explicitly so agents know which source
          promotion and swarm PR refs must be refreshed; source post-merge
          routed CI, CI, smoke, security, testing, and CI Actuals passed

EffortlessMetrics/shiplog#522:
  swarm head: 11b90411eed72fe1498213dd3c447fd67fbdbe5d
  included swarm PRs: EffortlessMetrics/shiplog-swarm#79
  source merge: f4e7d82b848cd97e1c27a432100986a25201ee16
  result: regular merge commit; repo contract report command provenance now
          uses the repo-required `rtk cargo xtask repo-contract-report`
          spelling; source post-merge routed CI, CI, smoke, security, testing,
          and CI Actuals passed

EffortlessMetrics/shiplog#523:
  swarm head: f0fdd45b3984be38d908de7efd2b82d94cc82204
  included swarm PRs: EffortlessMetrics/shiplog-swarm#80
  source merge: 0e90f5526f45f0f31eb195a3bf7e84abc998c408
  result: regular merge commit; `check-policy-ledgers` landed as the
          first-class policy-ledger proof command while preserving
          `check-policy-schemas` as a compatibility alias; source post-merge
          routed CI, CI, smoke, security, BDD/property testing, and
          CI Actuals passed

EffortlessMetrics/shiplog#524:
  swarm head: 84552f86c6109e6185cf8af634cdfd0f2ff360a2
  included swarm PRs: EffortlessMetrics/shiplog-swarm#81
  source merge: a8c173b2fe848a193d3903ba47a577da2bf58c55
  result: regular merge commit; policy-ledger gate command usage refreshed;
          post-merge swarm/main passed routed CI on CPX42, CI, smoke,
          security, BDD/property testing, Fuzzing, and CI Actuals

EffortlessMetrics/shiplog#525:
  swarm head: d2b20964996daf61c2b93b179c5979cfefaad3f1
  included swarm PRs: EffortlessMetrics/shiplog-swarm#82
  source merge: f02bbccee061cd942b0ab6234b87885ff8bf0957
  result: regular merge commit; workflow concurrency and cargo-deny
          install-action allowlist receipt landed; post-merge swarm/main
          passed routed CI on CPX42, policy gates, CI, smoke, security,
          BDD/property testing, Fuzzing, and CI Actuals

EffortlessMetrics/shiplog#526:
  swarm head: 8ff21a527354d59e22a9a86f6a3c4a04439b8bf7
  included swarm PRs: EffortlessMetrics/shiplog-swarm#83
  result: closed as superseded by EffortlessMetrics/shiplog#528 after the
          source fallback contract needed additional repair

EffortlessMetrics/shiplog#527:
  swarm head: e250c3472b05321dc0664f980b04c2be177a2683
  included swarm PRs: EffortlessMetrics/shiplog-swarm#83,
                       EffortlessMetrics/shiplog-swarm#84
  result: closed as superseded by EffortlessMetrics/shiplog#528 after the
          router-token-missing path needed the same hosted fallback behavior

EffortlessMetrics/shiplog#528:
  swarm head: de889e56d67ae7a11c37f47c7340464c073536b5
  included swarm PRs: EffortlessMetrics/shiplog-swarm#83,
                       EffortlessMetrics/shiplog-swarm#84,
                       EffortlessMetrics/shiplog-swarm#86
  source merge: c0e06a1540c0c18200f9c22858a27e8ecad0f851
  result: regular merge commit; source/release repo hosted fallback now covers
          runner-token, runner API, parse, and no-idle unavailable states;
          source post-merge routed CI, CI, smoke, security, testing, and
          CI Actuals passed

EffortlessMetrics/shiplog#529:
  swarm head: ae20816dddc6c921e4bce25d7176588120aa4c98
  included swarm PRs: EffortlessMetrics/shiplog-swarm#85,
                       EffortlessMetrics/shiplog-swarm#87
  source merge: 84eb6ca7b70ca9bb2e3f22659719695f39a74547
  result: regular merge commit; Codex CI-efficiency compatibility docs were
          aligned with current gates and non-PR hosted fallback was allowed for
          push/manual/merge-group routes; source post-merge routed CI, CI,
          smoke, security, testing, and CI Actuals passed

EffortlessMetrics/shiplog#530:
  swarm head: 37ad2c5deb17c35afe8fa0dbcec8f6d044a8f9b1
  included swarm PRs: EffortlessMetrics/shiplog-swarm#88
  source merge: 1793bd44c10e8c392bcf7243e7964e662fc9bf6d
  result: regular merge commit; promotion receipts refreshed through #88;
          source post-merge checks passed

EffortlessMetrics/shiplog#531:
  swarm head: 04ff63427ce4014c37b6b6d9dee21c74a0ddfd3d
  included swarm PRs: EffortlessMetrics/shiplog-swarm#89
  source merge: 40a3ce6a9cd4ea2dbd90918cfc24584a2aca968c
  result: regular merge commit; receipt-refresh loop status was deferred so
          future agents refresh receipts during substantive swarm PRs instead
          of creating an infinite receipt-only loop

EffortlessMetrics/shiplog#535:
  swarm head: 7415ab8118400d2f4f10e9477f67a796f16cf19e
  included swarm PRs: EffortlessMetrics/shiplog-swarm#90,
                       EffortlessMetrics/shiplog-swarm#91,
                       EffortlessMetrics/shiplog-swarm#92,
                       EffortlessMetrics/shiplog-swarm#93,
                       EffortlessMetrics/shiplog-swarm#96,
                       EffortlessMetrics/shiplog-swarm#94,
                       EffortlessMetrics/shiplog-swarm#95,
                       EffortlessMetrics/shiplog-swarm#97
  source merge: 50effd4c94848e91a64d5c24795438855888ff18
  result: regular merge commit; routed control-plane and promoted workflows
          were repaired so source fallback runs on GitHub-hosted runners and
          source-only dependency PRs #534 and #533 were closed as superseded by
          the swarm dependency PRs plus this source promotion

EffortlessMetrics/shiplog#536:
  swarm head: ec058bf6ba0f8832ff1862a9116dab3d13607637
  included swarm PRs: EffortlessMetrics/shiplog-swarm#98,
                       EffortlessMetrics/shiplog-swarm#99
  source merge: 9a3eabd9c257239ad8c94758052745f544091013
  result: regular merge commit; fuzz nightly and cargo-fuzz invocation were
          pinned after the rusqlite update exposed the old floating-nightly
          `cfg_select` failure; source Fuzzing proof passed before merge

EffortlessMetrics/shiplog#537:
  swarm head: 05abb57e5bc68e7326af88cfa2a114b195bdf06b
  included swarm PRs: EffortlessMetrics/shiplog-swarm#100
  source merge: 81d971a203566e30d6d58c6803af453cb9e6c6fd
  result: regular merge commit; source CI matrix routing was made
          GitHub-hosted-safe without adding self-hosted access to
          EffortlessMetrics/shiplog; stale source-only security report PR #532
          was closed rather than creating source-only divergence

EffortlessMetrics/shiplog#538:
  swarm head: e295d0c080f2c1e1df8671aeb979d2062212061d
  included swarm PRs: EffortlessMetrics/shiplog-swarm#101
  source merge: 3bd3bcc393b3709390afce857bc7bee5831e705b
  result: regular merge commit; CI lane display name was restored to the
          policy-ledger value while preserving source-safe runner selection;
          source post-merge routed CI, CI, Fuzzing, BDD Testing, and Property
          Testing passed; both PR queues were empty after merge

EffortlessMetrics/shiplog#539:
  swarm head: ec2a6d62e65d0647a432a4f46d687cb03226276b
  included swarm PRs: EffortlessMetrics/shiplog-swarm#102
  source merge: 82b205acecc48d6d09d46c8a76075270c63069df
  result: regular merge commit; promotion receipts were refreshed through
          source #538 and swarm #101; swarm proof included #102 PR run
          26717077588, swarm/main push run 26717191002, `check-goals`,
          `repo-contract-report`, and `git diff --check`

EffortlessMetrics/shiplog#540:
  swarm head: d8ea5515f60ce121bc6c2f7ae4ab802d98630433
  included swarm PRs: EffortlessMetrics/shiplog-swarm#103
  source merge: 31f3ec64d4381b4e15ddcbf1c3be72125e94c37d
  result: regular merge commit; raw self-hosted Rust-building workflows gained
          a shared native build dependency preflight for C++ and zlib; the
          observed Fuzzing `c++` failure and BDD Smoke `-lz` failure were both
          repaired on PR runs; source post-merge routed CI, CI, Fuzzing, BDD
          Testing, Property Testing, Coverage, smoke, and Security passed

EffortlessMetrics/shiplog#541:
  swarm head: b046873f79d9110d60e7bfc042f404140fc949ea
  included swarm PRs: EffortlessMetrics/shiplog-swarm#104
  source merge: 2a60835df9e5b164c2eb0cef22fac67a066c5f8c
  result: regular merge commit; promotion receipts were refreshed through
          source #540 and swarm #103; swarm/main routed proof passed in run
          26719505213, and source post-merge routed proof passed in run
          26719661881

EffortlessMetrics/shiplog#542:
  swarm head: 1046ae2eac0e7a99ad8af580fff2cce510a1ea7d
  included swarm PRs: EffortlessMetrics/shiplog-swarm#105
  source merge: e19e518246bbdff255d83b6878ebe12ca5614e57
  result: regular merge commit; repo-contract receipt freshness now defers
          scoped receipt-refresh loop subjects correctly; swarm PR proof ran on
          CPX42 in run 26719860670, swarm/main routed proof passed in run
          26720054356, and source post-merge routed proof passed in run
          26720251607

EffortlessMetrics/shiplog#543:
  swarm head: bf259dc9c6d668edca9cddc8fae4dc924a5a0932
  included swarm PRs: EffortlessMetrics/shiplog-swarm#106
  source merge: 2c83e1d16d954ce181dbf5730e1badb769ff1004
  result: regular merge commit; repo-contract report now surfaces remote
          branch hygiene cleanup candidates without deleting branches; swarm
          PR proof ran on CPX42 in run 26720449439, swarm/main routed proof
          passed in run 26720636561, and source post-merge routed proof passed
          in run 26720872910

EffortlessMetrics/shiplog#544:
  swarm head: 5887abc74cbbb35829be28f42334f8932046ec28
  included swarm PRs: EffortlessMetrics/shiplog-swarm#107
  source merge: 178bee3d6c03e531a83b2bb7e016c8161dd09118
  result: regular merge commit; BDD Smoke now runs the intended exact nested
          test names and has a 20-minute cold-build budget; swarm PR BDD Smoke
          passed in runs 26721132266 and 26721235327, swarm PR routed proof
          passed on CPX42 in run 26721235320, swarm/main routed proof passed in
          run 26721494988, and source post-merge routed proof passed in run
          26721679448

EffortlessMetrics/shiplog#545:
  swarm head: 97a3a9142509af41144159903389efd6c5214a38
  included swarm PRs: EffortlessMetrics/shiplog-swarm#108
  source merge: ea050623063ad7ea58900da855f9c6593eb79faf
  result: regular merge commit; repo-contract-report now splits remote branch
          hygiene candidates into merged and review buckets without deleting
          branches; swarm PR routed proof passed through GitHub-hosted fallback
          in run 26722181480, swarm/main routed proof passed in run
          26722350818, and source post-merge routed proof passed in run
          26722468484

EffortlessMetrics/shiplog#546:
  swarm head: 28d8acd2b787ea9cf8f793ae8f2d794966cd9896
  included swarm PRs: EffortlessMetrics/shiplog-swarm#109
  source merge: 34d343b32e477663ece08d4403047d4d8fdd4791
  result: regular merge commit; repo-contract-report docs and support-tier
          wording were aligned with the current report surface; swarm PR routed
          proof passed through GitHub-hosted fallback in run 26722668210,
          swarm/main routed proof passed in run 26722786184, and source
          post-merge routed proof passed in run 26722992603

EffortlessMetrics/shiplog#547:
  swarm head: 7daeb647b0f079f936a3504dba52545c0fdacd0a
  included swarm PRs: EffortlessMetrics/shiplog-swarm#110
  source merge: 732f24db3f26fb6412a2e31e8737d0a3574a7704
  result: regular merge commit; repo-contract-report help, docs, and receipt
          records were aligned with the inspection report surface; swarm PR
          routed proof passed through GitHub-hosted fallback in run
          26723178457, swarm/main routed proof passed in run 26723319011, and
          source post-merge routed proof passed in run 26723439772

EffortlessMetrics/shiplog#548:
  swarm head: e583f847803c68276a8f468218753d09753f8f56
  included swarm PRs: EffortlessMetrics/shiplog-swarm#111
  source merge: 1b819ec65c712f7bd354f7f7ae6b48b544e83f05
  result: regular merge commit; repo-contract-report title and output wording
          now describe inspection report artifacts while preserving the existing
          graph artifact paths; swarm PR routed proof passed through
          GitHub-hosted fallback in run 26723663178, swarm/main routed proof
          passed in run 26723723646, and source post-merge routed proof passed
          in run 26723925706

EffortlessMetrics/shiplog#549:
  swarm head: 4653396905926cda019238838dda73a9cffe932c
  included swarm PRs: EffortlessMetrics/shiplog-swarm#112
  source merge: ed8a4e1cdd7c56d46ff4396cac7d92765e5dfd97
  result: regular merge commit; repo-contract-report now emits
          non-destructive review commands for already-merged branch cleanup
          candidates; swarm PR routed proof passed on CX53 in run 26724134343,
          swarm/main routed proof passed through GitHub-hosted fallback in run
          26724290394, and source post-merge routed proof passed through
          GitHub-hosted fallback in run 26724558976

EffortlessMetrics/shiplog#550:
  swarm head: e86bcde481e175d77e83e20f6640375f0ed65ba2
  included swarm PRs: EffortlessMetrics/shiplog-swarm#113
  source merge: b99166a88a9bf912cf347122ce6686b758b8680d
  result: regular merge commit; repo-contract-report branch cleanup PR lookup
          was fixed so generated review commands use `--head <branch>` instead
          of the invalid `--head EffortlessMetrics:<branch>` shape; swarm PR
          routed proof passed through GitHub-hosted fallback in run
          26724794869, swarm/main routed proof passed through GitHub-hosted
          fallback in run 26725885443, and source post-merge routed proof
          passed through GitHub-hosted fallback in run 26726005951

Remote branch hygiene after EffortlessMetrics/shiplog#550:
  result: remote branch hygiene is clean; audited stale source and swarm
          branch candidates before deletion, preserved protected release
          branches, left no open PRs or issues in either repo, and reran
          `rtk cargo xtask repo-contract-report` with 0 source and 0 swarm
          cleanup candidates

EffortlessMetrics/shiplog#551:
  swarm head: f2cacaa612efdea3ecc9bff84a8fcb24698e25bf
  included swarm PRs: EffortlessMetrics/shiplog-swarm#114
  source merge: 8f2e802a4ea582db978acda248bd8ae4106930f4
  result: regular merge commit; repo-contract-report now reports scoped
          receipt-refresh heads as `pending-next-substantive-pr` even when the
          subject does not include the word `promotion`; swarm PR routed proof
          passed on CX43 in run 26726754000, swarm/main routed proof passed
          through GitHub-hosted fallback in run 26726941507, and source
          post-merge routed proof passed through GitHub-hosted fallback in run
          26727092150

EffortlessMetrics/shiplog#552:
  swarm head: e62fb36fb111792f123dc4d700cac346da3386d1
  included swarm PRs: EffortlessMetrics/shiplog-swarm#116
  superseded swarm PRs: EffortlessMetrics/shiplog-swarm#115
  source merge: 0cdedd1803e955fe1525ddf87c6d0ec31ad2c662
  result: regular merge commit; PR-body generation now fills Scope from
          Production delta or Goal and Non-goals from Non-goals or Claim
          boundary; trusted same-repo no-idle routes now fall back to the full
          GitHub-hosted Shiplog Rust Small proof instead of failing with
          `router_target=none`; swarm PR routed proof passed through
          GitHub-hosted fallback in run 26727445468, swarm/main routed proof
          passed through GitHub-hosted fallback in run 26727644817, source PR
          routed proof passed through GitHub-hosted fallback in run
          26727848375, and source post-merge routed proof passed through
          GitHub-hosted fallback in run 26728029584

EffortlessMetrics/shiplog#553:
  swarm head: 033751b5bb953d9298e069e2bb0c92b227c7404d
  included swarm PRs: EffortlessMetrics/shiplog-swarm#117
  source merge: ff2d9edc4bda168c5c810110b2518ce16741bc7f
  result: regular merge commit; repo-contract-report now emits a
          branch-protection contract section and reports whether
          `shiplog-swarm/main` requires only `Shiplog Rust Small Result`;
          swarm PR routed proof passed on CPX42 in run 26728631454,
          swarm/main routed proof passed through GitHub-hosted fallback in run
          26728820398, source PR routed proof passed through GitHub-hosted
          fallback in run 26729052733, and source post-merge routed proof
          passed through GitHub-hosted fallback in run 26729302413

EffortlessMetrics/shiplog#554:
  swarm head: 111234b907cbe1dfe74bac7b6e2c6b4b3c50bbc7
  included swarm PRs: EffortlessMetrics/shiplog-swarm#118
  source merge: ac7715ce5c179ce43e83ec0afe331c54f39b2447
  result: regular merge commit; support-tier and spec wording now describe
          the branch-protection report surface and its read-only GitHub
          inspection boundary; swarm PR routed proof passed through
          GitHub-hosted fallback in run 26729790699, swarm/main routed proof
          passed through GitHub-hosted fallback in run 26730025625, source PR
          routed proof passed through GitHub-hosted fallback in run
          26730266286, and source post-merge routed proof passed through
          GitHub-hosted fallback in run 26730531559

EffortlessMetrics/shiplog#555:
  swarm head: 402788580d321b8dc9b05a85477a2e9b43f97416
  included swarm PRs: EffortlessMetrics/shiplog-swarm#119
  source merge: a2edbbf67784b8a7d12cbbc7fcf135cf6895401d
  result: regular merge commit; repo-contract-report now includes remote PR
          and issue queue hygiene for both source and swarm repositories;
          swarm PR routed proof passed through GitHub-hosted fallback in run
          26731224265, swarm/main routed proof passed through GitHub-hosted
          fallback in run 26731460914, source PR routed proof passed through
          GitHub-hosted fallback in run 26731713593, and source post-merge
          routed proof passed through GitHub-hosted fallback in run 26731901009

EffortlessMetrics/shiplog#556:
  swarm head: 491dd34b4f3e2fb1c7588679d6832c09f6257924
  included swarm PRs: EffortlessMetrics/shiplog-swarm#120
  source merge: 474bf93ad7f120d173136f474e8e912b08005798
  result: regular merge commit; repo-contract-report now includes latest
          routed CI health for source and swarm main; swarm PR routed proof
          passed through GitHub-hosted fallback in run 26732583604,
          swarm/main routed proof passed through GitHub-hosted fallback in run
          26732848475, source PR routed proof passed through GitHub-hosted
          fallback in run 26733115282, and source post-merge routed proof
          passed through GitHub-hosted fallback in run 26733379570

EffortlessMetrics/shiplog#557:
  swarm head: 27d87fd4c3e050d6d2012b8339de78ae43d609c4
  included swarm PRs: EffortlessMetrics/shiplog-swarm#121
  source merge: f8d2f54373a710d42e0b66594cc10caa46a5377b
  result: regular merge commit; repo-contract-report now checks the latest
          source promotion PR title, body receipts, merge commit, swarm head,
          included swarm PRs, proof text, and merge-method boundary; swarm PR
          routed proof passed through GitHub-hosted fallback in run
          26733972910, swarm/main routed proof passed through GitHub-hosted
          fallback in run 26734224837, source PR routed proof passed through
          GitHub-hosted fallback in run 26734497696, and source post-merge
          routed proof passed through GitHub-hosted fallback in run 26734733784

EffortlessMetrics/shiplog#558:
  swarm head: 454d63dc95ee53613307c94c70309f4c76485483
  included swarm PRs: EffortlessMetrics/shiplog-swarm#122
  source merge: a03edddf3141298c77cf8195457c192b7c5bbbec
  result: regular merge commit; repo-contract-report docs now describe the
          current remote queue, routed CI, promotion PR contract,
          branch-protection, and receipt-freshness report surfaces; swarm PR
          routed proof passed through GitHub-hosted fallback in run
          26735223253, swarm/main routed proof passed through GitHub-hosted
          fallback in run 26735443961, source PR routed proof passed through
          GitHub-hosted fallback in run 26735720555, and source post-merge
          routed proof passed through GitHub-hosted fallback in run 26735969679

EffortlessMetrics/shiplog#559:
  swarm head: e29940fff8892129bddbd4ce629d598b79f9e1b0
  included swarm PRs: EffortlessMetrics/shiplog-swarm#123
  source merge: ca35121beb5126e2ba2b7d5918e287ca383daeb7
  result: regular merge commit; repo-contract-report now keeps open
          PR-backed remote branches out of cleanup candidate counts and
          reports them separately; swarm PR routed proof passed through
          GitHub-hosted fallback in run 26736586426, swarm/main routed proof
          passed through GitHub-hosted fallback in run 26736861651, source PR
          routed proof passed through GitHub-hosted fallback in run
          26737148791, and source post-merge routed proof passed through
          GitHub-hosted fallback in run 26737538802

EffortlessMetrics/shiplog#560:
  swarm head: 6f2c9d0d9aea889948de9243007b6ad891b20d8d
  included swarm PRs: EffortlessMetrics/shiplog-swarm#124
  source merge: 53faf292918cd957418d9381af55715beb439426
  result: regular merge commit; Droid BYOK workflows switched to
          `custom:MiniMax-M3-0` and the PR branch fix restored runtime
          `MINIMAX_API_KEY` expansion in the generated Factory
          `settings.json`; swarm PR routed proof passed through GitHub-hosted
          fallback in run 26738771780 after the branch update, swarm/main
          routed proof passed through GitHub-hosted fallback in run
          26739095859, source PR routed proof passed through GitHub-hosted
          fallback in run 26739477590, and source post-merge routed proof
          passed through GitHub-hosted fallback in run 26740543589

EffortlessMetrics/shiplog#561:
  swarm head: cb1e120ab88382b115226ccb6434134611e1c7a8
  included swarm PRs: EffortlessMetrics/shiplog-swarm#125
  source merge: eb07299eec2b7cc4b1b491ad31dd78e7d2943799
  result: regular merge commit; PR-body receipt wording now reports the last
          recorded receipt refs in manifest order rather than implying
          chronological ordering; swarm PR routed proof selected CX53 and
          passed in run 26741193351, swarm/main routed proof passed through
          GitHub-hosted fallback in run 26741406611, source PR routed proof
          passed through GitHub-hosted fallback in run 26741814649, and source
          post-merge routed proof passed through GitHub-hosted fallback in run
          26742339037

EffortlessMetrics/shiplog#562:
  swarm head: e5f3e0a2cd03e3ce5e64706ca548af92994287ae
  included swarm PRs: EffortlessMetrics/shiplog-swarm#126
  source merge: 5bd3aae89d789eca3397612a6ec88968a6825abe
  result: regular merge commit; repo-contract-report now reports local branches
          already merged into source or swarm so agents can clean up local branch
          state deliberately; swarm PR routed proof passed through
          GitHub-hosted fallback in run 26743582511, swarm/main routed proof
          passed through GitHub-hosted fallback in run 26743990955, source PR
          routed proof passed through GitHub-hosted fallback in run
          26744475115, Droid Auto Review completed successfully in run
          26744475073, and source post-merge routed proof passed through
          GitHub-hosted fallback in run 26745960547

EffortlessMetrics/shiplog#563:
  swarm head: e8964072d48fba68ec15e1f68f4f51eb0fef66de
  included swarm PRs: EffortlessMetrics/shiplog-swarm#127
  source merge: 041da91166231247904ef76200dcc3fd0231c6c4
  result: regular merge commit; local merged-branch cleanup review commands
          now check both source and swarm PR head refs before showing recent
          commits; swarm PR routed proof passed through GitHub-hosted fallback
          in run 26746985007, swarm/main routed proof passed through
          GitHub-hosted fallback in run 26747425140, source PR routed proof
          passed through GitHub-hosted fallback in run 26747901181, Droid Auto
          Review completed successfully in run 26747901204, and source
          post-merge routed proof passed through GitHub-hosted fallback in run
          26748501939

EffortlessMetrics/shiplog#565:
  swarm head: ec0c875381bf78494c747838effb6ff7d301e102
  included swarm PRs: EffortlessMetrics/shiplog-swarm#128
  source merge: ddd9c37d323b635f8d026f599ba41d5e50517d43
  result: regular merge commit; repo-contract-report now includes read-only
          review commands for unmerged remote cleanup candidates and the stale
          source-only Droid security-report branch was deleted after review;
          swarm PR routed proof passed through GitHub-hosted fallback in run
          26749487577, swarm/main routed proof passed through GitHub-hosted
          fallback in run 26749890755, source PR routed proof passed through
          GitHub-hosted fallback in run 26750318232, Droid Auto Review
          completed successfully in run 26750318209, and source post-merge
          routed proof passed through GitHub-hosted fallback in run 26750893579

EffortlessMetrics/shiplog#566:
  swarm head: c7d9a4663b0772fec0439be580b9a11758ae24ad
  included swarm PRs: EffortlessMetrics/shiplog-swarm#129
  source merge: 365cce5f74d958640b06f350437406b271038328
  result: regular merge commit; repo-contract-report now lists exact failed
          promotion contract receipt checks so agents can repair missing title,
          state, merge commit, swarm head, included PR, proof, or merge-method
          fields directly; swarm PR routed proof passed through GitHub-hosted
          fallback in run 26751744033, swarm/main routed proof passed through
          GitHub-hosted fallback in run 26752145294, source PR routed proof
          passed through GitHub-hosted fallback in run 26752554780, Droid Auto
          Review completed successfully in run 26752554743, and source
          post-merge routed proof passed through GitHub-hosted fallback in run
          26753212535

EffortlessMetrics/shiplog#567:
  swarm head: 094f4ba070dd29dc3ffe353af39866b89e9f3e63
  included swarm PRs: EffortlessMetrics/shiplog-swarm#130
  source merge: 6f53b3df8657f2e576b1b04007493c72aed17f49
  result: regular merge commit; repo-contract-report now requires promotion
          PR bodies to record both swarm proof and source proof before the
          promotion contract is aligned; swarm PR routed proof passed through
          GitHub-hosted fallback in run 26753970486, swarm/main routed proof
          passed through GitHub-hosted fallback in run 26754401083, source PR
          routed proof passed through GitHub-hosted fallback in run
          26754856700, Droid Auto Review completed successfully in run
          26754856620, and source post-merge routed proof passed through
          GitHub-hosted fallback in run 26755388308

EffortlessMetrics/shiplog#568:
  swarm head: 07f7b4f4bc7ad7919fd9861538bb5886514ce45b
  included swarm PRs: EffortlessMetrics/shiplog-swarm#131
  source merge: 6c3229738184017e05ec918e096b806b767e2bba
  result: regular merge commit; check-goals now rejects legacy
          .shiplog/goals/active.toml manifests so active execution state stays
          in .codex/goals/active.toml; swarm PR routed proof passed through
          GitHub-hosted fallback in run 26756481456, swarm/main routed proof
          passed through GitHub-hosted fallback in run 26756949390, source PR
          routed proof passed through GitHub-hosted fallback in run
          26757424047, Droid Auto Review completed successfully in run
          26757424097, and source post-merge routed proof passed through
          GitHub-hosted fallback in run 26758593852

EffortlessMetrics/shiplog#569:
  swarm head: 66befe0a19f91108c3df73df5fd80c6ff3fea470
  included swarm PRs: EffortlessMetrics/shiplog-swarm#132
  source merge: 83ac231f8d95c41f10ee917ed15acc4e7a0d2eca
  result: regular merge commit; non-routed shiplog-swarm workflow
          self-hosted selectors now require a non-PR event or a same-repo PR
          trust boundary before selecting self-hosted runners; swarm PR routed
          proof selected CPX42 and passed in run 26759855378, swarm/main routed
          proof passed through GitHub-hosted fallback in run 26760358230,
          source PR routed proof passed through GitHub-hosted fallback in run
          26760887261, Droid Auto Review completed successfully in run
          26760884780, and source post-merge routed proof passed through
          GitHub-hosted fallback in run 26762706895

EffortlessMetrics/shiplog#570:
  swarm head: 8efda786f4823e4a0461e852de81536d485df3c2
  included swarm PRs: EffortlessMetrics/shiplog-swarm#133
  source merge: 6acb78da4b3823785014799c2ff08aee105004cc
  result: regular merge commit; workflow-admission receipts were refreshed
          without opening a release-authority lane; swarm PR routed proof
          passed through GitHub-hosted fallback in run 26763777290, swarm/main
          routed proof passed through GitHub-hosted fallback in run
          26764314161, source PR routed proof passed through GitHub-hosted
          fallback in run 26764858981, Droid Auto Review completed
          successfully in run 26764858810, and source post-merge routed proof
          passed through GitHub-hosted fallback in run 26765810610

EffortlessMetrics/shiplog#571:
  swarm head: 471a5a9fd4c1640429611577e3e9e397fc6d45d9
  included swarm PRs: EffortlessMetrics/shiplog-swarm#134
  source merge: ff5d89d9f3bd83184be726697757fff39ad36683
  result: regular merge commit; repo-contract-report now treats record/update
          receipt and promotion-ledger titles as receipt-refresh heads for the
          self-referential loop guard; swarm PR routed proof passed through
          GitHub-hosted fallback in run 26766596591, swarm/main routed proof
          passed through GitHub-hosted fallback in run 26767070210, source PR
          routed proof passed through GitHub-hosted fallback in run
          26767557186, Droid Auto Review completed successfully in run
          26767557581, and source post-merge routed proof passed through
          GitHub-hosted fallback in run 26768552919
```

### Proof commands

```bash
rtk git fetch origin main --prune --tags
rtk git fetch swarm main --prune
rtk git merge-base origin/main swarm/main
rtk git log --oneline origin/main..swarm/main
rtk cargo xtask repo-contract-report
rtk gh pr create --base main --head promote/swarm-YYYYMMDD-SHA
rtk git diff --check
```

### Rollback

Revert the promotion merge commit in `shiplog` and pause further promotions
until the divergence is understood.

### Claim boundary

Promotion keeps the release/source repo current. It still does not move release
authority to `shiplog-swarm`.
