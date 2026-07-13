# Current shiplog-swarm Promotion

**Status:** source promotion in progress
**Swarm head before receipt refresh:** `2862863b38ce5fe078ac440085648c4198a460be`
**Source base:** `a3a15edcdc03d667f6e0152b86455c067c5d6509`
**Source promotion:** `EffortlessMetrics/shiplog#652`

## Included work

- `EffortlessMetrics/shiplog-swarm#225` — canonicalize unknown source-system names to lowercase.
- `EffortlessMetrics/shiplog-swarm#226` — enforce half-open Git and Jira date windows.
- `EffortlessMetrics/shiplog-swarm#233` — make hosted routing deterministic and refresh the current promotion receipts.

## Proof

### Swarm proof

- `Shiplog Rust Small Result` passed for `shiplog-swarm/main` at `2862863b38ce5fe078ac440085648c4198a460be` in run `29218878179`.
- Branch protection is strict and requires only `Shiplog Rust Small Result`.
- PR #233 makes `allow-github-hosted` and `ci-budget-ack` route directly to GitHub-hosted CI before self-hosted runner discovery.
- The stale pre-reconciliation security report PR `EffortlessMetrics/shiplog-swarm#200` was closed unmerged.

### Source proof

- `EffortlessMetrics/shiplog#652` is the stable source-local regular-merge promotion PR.
- Source `Shiplog Rust Small Result` passed at the pre-receipt head in run `29230991924` on closed transport PR `EffortlessMetrics/shiplog#649`.
- After PR #233 merges, the source-local promotion branch will fast-forward to the exact resulting swarm main SHA and all source checks must rerun and pass there.
- The source-only regex Dependabot PR `EffortlessMetrics/shiplog#632` was closed; dependency work remains on the swarm trunk.

## Merge boundary

- Swarm PRs use squash merge.
- Source promotion uses a regular merge commit; do not squash.
- Release authority, tags, publishing, signing, and release workflows remain in `EffortlessMetrics/shiplog`.

## Verification

```bash
rtk git fetch origin --prune
rtk git fetch swarm --prune
rtk git merge-base origin/main swarm/main
rtk git log --oneline origin/main..swarm/main
rtk gh run list --repo EffortlessMetrics/shiplog-swarm --branch main --limit 10
rtk gh run list --repo EffortlessMetrics/shiplog --branch main --limit 10
rtk cargo xtask repo-contract-report
rtk git diff --check
```

## Next action

Merge PR #233 after exact-head proof, import the resulting swarm main object into the source repository, fast-forward the source-local branch behind `EffortlessMetrics/shiplog#652`, require green source proof at that exact head, and merge #652 with a regular merge commit.
