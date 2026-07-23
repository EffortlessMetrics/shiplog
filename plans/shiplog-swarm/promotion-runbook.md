# Shiplog Swarm Promotion Runbook

This runbook promotes proven `shiplog-swarm/main` work into
`EffortlessMetrics/shiplog/main` without moving release authority.

Normal development still happens in `EffortlessMetrics/shiplog-swarm`.
`EffortlessMetrics/shiplog` remains the release/public source surface.

## When To Promote

Promote after one or more green swarm PRs when the source/release repo should
checkpoint the current development state.

Promote before release preflight, release docs refreshes, source-only release
work, or any handoff that expects `shiplog/main` to include current swarm work.

Do not promote while either repo has an unexplained failing required check or
an ambiguous open release-blocking PR.

## Preconditions

- `EffortlessMetrics/shiplog-swarm` open PR queue is empty or explicitly
  deferred.
- `EffortlessMetrics/shiplog` open PR queue is empty or explicitly deferred.
- `shiplog-swarm/main` has green `Shiplog Rust Small Result`.
- `shiplog/main` and `shiplog-swarm/main` share history.
- The promotion branch contains only the intended `origin/main..swarm/main`
  range.

## Prepare The Promotion Branch

Run from a checkout that has:

```text
origin = git@github.com:EffortlessMetrics/shiplog.git
swarm  = git@github.com:EffortlessMetrics/shiplog-swarm.git
```

```powershell
rtk git fetch origin --prune
rtk git fetch swarm --prune

rtk git merge-base origin/main swarm/main
rtk git log --oneline origin/main..swarm/main
rtk git diff --stat origin/main..swarm/main

$swarmSha = (rtk git rev-parse --short swarm/main).Trim()
$branch = "promote/swarm-$(Get-Date -Format yyyyMMdd)-$swarmSha"

rtk git push origin "swarm/main:refs/heads/$branch"
```

Stop if `git merge-base` prints nothing, if the log contains unintended work,
or if the diff is broader than the swarm PRs being promoted.

## Open The Source PR

```powershell
rtk gh pr create --repo EffortlessMetrics/shiplog --base main --head $branch --title "merge(swarm): promote shiplog-swarm through $swarmSha" --body-file <body.md>
```

The PR body must include:

```text
swarm head SHA
included swarm PRs
`## Swarm proof` section with Shiplog Rust Small Result evidence
`## Source proof` section with Shiplog Rust Small Result evidence
explicit note to merge with a regular merge commit, not squash
claim boundary: no release authority movement
```

`repo-contract-report` validates the exact `Swarm proof` and `Source proof`
section labels, and each section must mention `Shiplog Rust Small Result`.

## Merge

Only merge after source PR checks are green.

```powershell
rtk gh pr merge <number> --repo EffortlessMetrics/shiplog --merge --delete-branch
```

Do not use `--squash` for source promotion PRs. Swarm work is already squashed
at the normal development boundary; the source merge commit is the checkpoint.

## Post-Merge Verification

After merge, verify source `main`:

```powershell
rtk gh run list --repo EffortlessMetrics/shiplog --branch main --limit 12 --json databaseId,workflowName,status,conclusion,headSha,createdAt,displayTitle

rtk cargo xtask repo-contract-report

rtk gh pr list --repo EffortlessMetrics/shiplog --state open --limit 50
rtk gh pr list --repo EffortlessMetrics/shiplog-swarm --state open --limit 50
rtk gh api repos/EffortlessMetrics/shiplog-swarm/branches/main/protection/required_status_checks --jq '{strict: .strict, contexts: .contexts, checks: .checks}'
```

Expected:

```text
source post-merge EM CI Routed Shiplog Rust: success
source post-merge CI: success
repo-contract-report git topology: tree-aligned
repo-contract-report source ahead classification: promotion-merge-only
repo-contract-report source other commits: 0
shiplog open PR queue: empty or explicitly deferred
shiplog-swarm open PR queue: empty or explicitly deferred
shiplog-swarm required checks: Shiplog Rust Small Result only
```

## Source-Only Changes

Avoid source-only product, docs, or CI changes after cutover. If emergency
release work lands directly in `EffortlessMetrics/shiplog`, back-sync that
change into `EffortlessMetrics/shiplog-swarm` before more normal development
lands there.

Routine dependency, workflow-update, security-remediation, and documentation
automation must propose changes in `shiplog-swarm`. Source security automation
may fail a check, retain an artifact, or link a remediation handoff to swarm,
but it must not create a product branch or pull request in `shiplog`.

For an emergency hotfix, create and prove the fix in swarm first. If explicit
release authority requires an immediate source hotfix, pause normal promotion,
land the authorized source change, back-port the exact fix into swarm, and
re-establish tree alignment before promotion resumes. Emergency authorization
does not become standing permission for source-side product automation.

Verify the role boundary explicitly rather than inferring it from remote names:

```powershell
rtk cargo xtask check-automation-authority --repository-role swarm
# Run with `--repository-role source` in the canonical source checkout.
```

## Rollback

If a promotion merge is wrong, revert the merge commit in
`EffortlessMetrics/shiplog` and pause further promotions until the divergence is
understood.

Do not rewrite `shiplog/main` history. Do not force-push source promotion
branches after review has started.

## Claim Boundary

Promotion keeps `shiplog/main` current with proven swarm work. It does not move
tags, crates.io publish, GitHub Releases, signing, release branches, release
workflows, or security-sensitive token operations to `shiplog-swarm`.
