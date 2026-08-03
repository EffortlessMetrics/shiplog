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
git fetch origin --prune
git fetch swarm --prune

git merge-base origin/main swarm/main
git log --oneline origin/main..swarm/main
git diff --stat origin/main..swarm/main

$swarmSha = (git rev-parse --short swarm/main).Trim()
$branch = "promote/swarm-$(Get-Date -Format yyyyMMdd)-$swarmSha"

git push origin "swarm/main:refs/heads/$branch"
```

Stop if `git merge-base` prints nothing, if the log contains unintended work,
or if the diff is broader than the swarm PRs being promoted.

## Open The Source PR

```powershell
gh pr create --repo EffortlessMetrics/shiplog --base main --head $branch --title "merge(swarm): promote shiplog-swarm through $swarmSha" --body-file <body.md>
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
gh pr merge <number> --repo EffortlessMetrics/shiplog --merge --delete-branch
```

Do not use `--squash` for source promotion PRs. Swarm work is already squashed
at the normal development boundary; the source merge commit is the checkpoint.

## Post-Merge Verification

After merge, verify source `main`:

```powershell
gh run list --repo EffortlessMetrics/shiplog --branch main --limit 12 --json databaseId,workflowName,status,conclusion,headSha,createdAt,displayTitle

cargo xtask repo-contract-report

gh pr list --repo EffortlessMetrics/shiplog --state open --limit 50
gh pr list --repo EffortlessMetrics/shiplog-swarm --state open --limit 50
gh api repos/EffortlessMetrics/shiplog-swarm/branches/main/protection/required_status_checks --jq '{strict: .strict, contexts: .contexts, checks: .checks}'
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

## Transition Evidence And Resolution

An active transition receipt records two separate questions for each path:

1. What happened historically? `disposition = "missing_in_swarm"` records that
   the source PR changed the path and no swarm chain carries that change. It
   remains blocking evidence by itself.
2. What is this bounded promotion allowed to do? An exceptional source-side
   decision may explicitly set `resolution = "discard_source"`.

`discard_source` is valid only with a non-empty `decision_receipt`, its full
`decision_merge_sha` reachable from the current swarm target, a human-readable
`reason`, exact `source_tree_entry` and `swarm_tree_entry` bindings for the
immutable evidence targets, and differing source/swarm entries. The recorded
`source_target` and `swarm_target` values are evidence commits, not frozen
promotion refs: each must be an ancestor of the current promotion target, and
the recorded complete tree entries must still match at both the evidence
targets and the current targets. It selects the swarm tree entry for that path
for this promotion only; it does not create permanent source authority or
replace `source-only-paths.toml`. Unrelated tip advancement is therefore safe,
but a later change to the governed path fails closed and requires a new
reviewed decision.

For dependency-only lockfile transitions, use
`disposition = "dependency_equivalent"`. This is limited to `Cargo.lock` and
requires one swarm PR whose parsed package `name`/`version` from/to transitions
match the source PR exactly. It is narrower than whole-patch `equivalent` and
does not assert that unrelated lockfile resolutions or final blobs are equal;
those differences still require their own evidence or an explicit bounded
resolution.

Example shape:

```toml
[[transition.path]]
path = "docs/xtask.md"
disposition = "missing_in_swarm"
resolution = "discard_source"
decision_receipt = "EffortlessMetrics/shiplog-swarm#242"
decision_merge_sha = "<decision-receipt-merge-sha>"
reason = "Reviewed swarm state supersedes the source transition copy."
source_tree_entry = { mode = "100644", object_type = "blob", oid = "<source-target-oid>" }
swarm_tree_entry = { mode = "100644", object_type = "blob", oid = "<swarm-target-oid>" }
```

The promotion planner and overlay must consume this same per-path decision.
Decision receipts are bounded by the transition entry's `consumed_by`
promotion and must never be inferred from matching trees, commit subjects, or
source-only policy.

### Keeping source for a source-only path

The source-only policy remains blocking when swarm changes one of its paths.
When review confirms that the source copy must win for one bounded promotion,
record a separate top-level `[[source_authority]]` decision in
`promotion-state.toml`. The planner records this as a `kept-source` resolution
basis; it is not a change to `source-only-paths.toml` and does not grant
permanent source authority.

Example shape:

```toml
[[source_authority]]
path = ".github/workflows/release.yml"
source_target = "<immutable-source-evidence-sha>"
swarm_target = "<immutable-swarm-evidence-sha>"
decision_receipt = "EffortlessMetrics/shiplog-swarm#319"
decision_merge_sha = "<decision-receipt-merge-sha>"
reason = "The canonical source repository retains release-writer authority."
source_tree_entry = { mode = "100644", object_type = "blob", oid = "<source-target-oid>" }
swarm_tree_entry = { mode = "100644", object_type = "blob", oid = "<swarm-target-oid>" }
```

An active decision must name the exact immutable evidence `source_target` and
`swarm_target`, the merged decision receipt and full `decision_merge_sha`, a
human-readable `reason`, and complete `source_tree_entry` /
`swarm_tree_entry` values. The promotion verifies that the path is still
policy-listed, each evidence target is an ancestor of the current promotion
target, the entries still differ and match at both target pairs, and the
decision merge is reachable from the current swarm target. After the promotion
consumes it, set `consumed_by`; the historical record remains visible but
grants no further authority.

The same decision must appear in the structured path plan and determine the
overlay effect. Missing, stale, unmerged, unreachable, or target-mismatched
decisions fail closed. A later swarm change requires a new reviewed decision.

Verify the role boundary explicitly rather than inferring it from remote names:

```powershell
cargo xtask check-automation-authority --repository-role swarm
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
