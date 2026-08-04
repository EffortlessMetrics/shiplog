# Shiplog Swarm Promotion Runbook

This runbook promotes one proven, exact `shiplog-swarm/main` head into
`EffortlessMetrics/shiplog/main` without moving release authority.

Normal development remains authoritative in `EffortlessMetrics/shiplog-swarm`.
`EffortlessMetrics/shiplog` remains the public source and release surface.
Promotion is a regular merge-commit checkpoint, not a second development lane.

The canonical entrypoint is **`cargo xtask promote`**. Do not prepare a source
promotion by pushing raw `swarm/main`, hand-building an overlay, or manually
reconstructing the promotion PR body.

## When to promote

Promote a coherent batch of green swarm work when source must checkpoint the
current development state, especially before:

- release preparation;
- source-owned release workflow or publication work;
- a public-source handoff that expects current product behavior; or
- a bounded source-governance transaction that depends on the current tree.

Do not promote merely to keep a copied pending list cosmetically current. Batch
coherent proven work, then record the completed transaction during closeout.

## Preconditions

Before planning a promotion:

- open PRs and issues in both repositories are included, explicitly deferred,
  closed as superseded/duplicate, or identified as blockers;
- the exact `shiplog-swarm/main` head has a successful
  `Shiplog Rust Small Result` aggregate;
- `cargo xtask promotion-state --check` passes;
- `cargo xtask repo-contract-report` reports no unexplained source product
  commits or unknown source/swarm drift;
- source and swarm share the expected promotion ancestry;
- active transition and source-authority decisions are exact-target-bound,
  reviewable, and unconsumed;
- the source open-PR queue contains no incompatible promotion PR; and
- source `main` legacy branch protection or its active repository ruleset
  requires the exact `reject-routine-bot-pr` Source Automation Guard check.

A required-check failure, stale decision, unexplained source commit, incomplete
receipt range, or ambiguous path resolution is a stop condition. Treat the
failure as evidence to repair, not friction to route around.

## Maintainer checkout

Run from a clean tracked checkout with both repositories available:

```text
origin = git@github.com:EffortlessMetrics/shiplog.git
swarm  = git@github.com:EffortlessMetrics/shiplog-swarm.git
```

```bash
git fetch origin --prune
git fetch swarm --prune
git status --short --branch

git merge-base origin/main swarm/main
git log --oneline origin/main..swarm/main
git diff --stat origin/main..swarm/main

cargo xtask promotion-state --check
cargo xtask repo-contract-report
```

Stop if the merge base is missing, the source head is not the current source
main you intend to target, the log contains unintended work, or the diff does
not match the reviewed swarm batch and bounded source-governance decisions.

## Plan the exact promotion

Resolve the exact current swarm head after fetching both remotes:

```bash
swarm_head="$(git rev-parse swarm/main)"
```

Run two read-only plans and require deterministic output:

```bash
mkdir -p target/source-of-truth

cargo xtask promote --swarm-sha "$swarm_head" --dry-run \
  > target/source-of-truth/promote-plan-1.json
cargo xtask promote --swarm-sha "$swarm_head" --dry-run \
  > target/source-of-truth/promote-plan-2.json

cmp target/source-of-truth/promote-plan-1.json \
    target/source-of-truth/promote-plan-2.json
```

PowerShell equivalent:

```powershell
$swarmHead = (git rev-parse swarm/main).Trim()
New-Item -ItemType Directory -Force target/source-of-truth | Out-Null

cargo xtask promote --swarm-sha $swarmHead --dry-run |
  Set-Content -NoNewline target/source-of-truth/promote-plan-1.json
cargo xtask promote --swarm-sha $swarmHead --dry-run |
  Set-Content -NoNewline target/source-of-truth/promote-plan-2.json

if ((Get-FileHash target/source-of-truth/promote-plan-1.json).Hash -ne
    (Get-FileHash target/source-of-truth/promote-plan-2.json).Hash) {
  throw "promotion dry-runs are not deterministic"
}
```

Review the structured plan itself. Confirm:

- `source_head`, `swarm_head`, and merge base are the expected full SHAs;
- the routed workflow and terminal `Shiplog Rust Small Result` belong to the
  exact swarm SHA;
- every included merged swarm PR appears in the receipt list;
- every differing path records the exact source and swarm tree entries;
- each path effect is backed by the current policy, transition evidence,
  explicit discard-source decision, or bounded source-authority decision that
  actually entitles it;
- the overlay tree, resolution-plan identity, branch identity, and source PR
  action are deterministic; and
- neither dry-run created a branch, PR, worktree residue, durable object, or
  source mutation.

A non-zero dry-run is the exact repair queue. Do **not** make it pass by:

- supplying an older `--source-ref` while targeting current source `main`;
- using `--allow-historical` for an ordinary current promotion;
- pushing raw `swarm/main` to source;
- adding a path to source-only policy merely because the two repos differ;
- guessing a receipt from commit subjects or matching file names; or
- weakening exact tree-entry or ancestry checks.

Historical mode is for explicit historical diagnosis. It is not authority to
prepare a real PR against a different current source head.

## Prepare or update the source promotion PR

After the exact plan is deterministic and reviewed:

```bash
cargo xtask promote --swarm-sha "$swarm_head"
```

A real execution must re-resolve live source and swarm state and then:

- verify source merge control before any mutation;
- create or fast-forward one deterministic source-local promotion branch;
- create, update, or reuse one compatible source promotion PR;
- refuse a non-fast-forward branch or incompatible/duplicate PR;
- build the exact overlay from the same per-path decisions accepted by the
  planner;
- preserve source-owned release/governance paths only through current bounded
  authority;
- retain an always-present deterministic checkpoint commit, including when the
  final tree would otherwise be unchanged;
- emit the machine-readable promotion receipt; and
- stop before merge, tag, crates.io publication, GitHub Release publication,
  signing, or package-channel mutation.

Run the command again against unchanged state. It should reuse the same branch,
PR, overlay, title, body, and receipt identity rather than create duplicates.

Inspect the actual source PR diff and body. It must include:

- the exact swarm head SHA;
- the complete included swarm PR receipt list;
- a `## Swarm proof` section naming `Shiplog Rust Small Result` evidence;
- a `## Source proof` section naming the source PR proof boundary;
- an explicit instruction to use a regular merge commit, never squash; and
- a claim boundary stating that promotion does not move release authority.

The PR body is not independent proof. Review the exact source diff, exact-head
source CI, overlay trailers, and machine receipt.

## Merge the source checkpoint

Only merge after the exact source PR head is green and the actual diff matches
the reviewed plan.

```bash
gh pr merge <number> --repo EffortlessMetrics/shiplog --merge --delete-branch
```

Do not use `--squash` or `--rebase`. Swarm PRs are already squash-merged at the
normal development boundary; the source merge commit is the durable ancestry
checkpoint.

## Verify the landed promotion

After the regular merge lands:

```bash
git fetch origin --prune
git fetch swarm --prune

cargo xtask promote --swarm-sha "$swarm_head" --verify-only
cargo xtask repo-contract-report

gh run list --repo EffortlessMetrics/shiplog --branch main --limit 12 \
  --json databaseId,workflowName,status,conclusion,headSha,createdAt,displayTitle
```

Require evidence that:

- the exact swarm head landed through an accepted two-parent checkpoint shape;
- the overlay has exactly the expected source parent and carries matching
  `Shiplog-Source-Head`, `Shiplog-Swarm-Head`, and resolution-plan identities;
- the complete overlay tree matches the selected source/swarm tree entries,
  including file mode, object type, object ID, and absence;
- source post-merge routed CI succeeded at the exact landed source result;
- the source-ahead classification contains the promotion checkpoint and only
  approved source governance;
- unexplained source product commits are zero; and
- source/swarm open queues are empty or explicitly deferred.

`--verify-only` proves the Git topology transaction. Source post-merge CI,
queue state, source-governance drift, and current contract alignment remain
`repo-contract-report` responsibilities.

## Close out the bounded promotion state

The completed transaction must be recorded in a substantive closeout change on
swarm:

- update `latest_promotion` with the source promotion PR, regular-merge SHA,
  promoted swarm head, included swarm PRs, source-governance receipts, and
  source post-merge proof;
- consume transition and source-authority decisions at the landed checkpoint;
- establish the new pending range from actual merged swarm work after the
  promoted head;
- regenerate `plans/shiplog-swarm/current-promotion.md` with
  `cargo xtask promotion-state`; and
- run `promotion-state --check`, `repo-contract-report`, and the relevant
  transition tests.

This required closeout records the landed transaction. It is distinct from the
anti-pattern of opening repeated receipt-only PRs merely to chase every moving
swarm head before a promotion exists.

After closeout, a new current-head dry-run must recognize the just-landed
checkpoint and either produce a valid next plan or a precise fail-closed repair
queue.

## Transition evidence and per-path resolution

Promotion permission and overlay content come from one exact per-path plan.
The following rules are non-negotiable:

- Only a current bounded source-authority decision may select source for a
  source-only policy path that swarm also changed.
- A historical `missing_in_swarm` fact grants no authority by itself.
- Exceptional abandonment of a source change requires explicit
  `resolution = "discard_source"`, a merged and reachable decision receipt,
  a human-readable reason, exact immutable evidence targets, and complete
  source/swarm tree entries.
- `equivalent`, `tree_equivalent`, `dependency_equivalent`,
  `superseded_in_swarm`, `converged_at_target`, and self-referential cases keep
  their distinct evidence meanings; do not collapse them into tree similarity.
- Current target entries and ancestry must still satisfy the recorded evidence.
  A later change to the governed path fails closed and requires a new decision.
- `consumed_by` retires authority while preserving history.
- No authority is inferred from commit subjects, matching paths, branch names,
  PR-body prose, or an old receipt that happened to mention the path.

The durable human decision ledger is
[`transition-decisions.md`](transition-decisions.md); the exact active machine
bindings live in [`promotion-state.toml`](promotion-state.toml). Keep those two
roles separate: reviewed rationale first, exact merged receipt binding second.

## Source-only and emergency source changes

Routine dependency, workflow-update, security-remediation, documentation, and
product automation must propose changes in swarm. Source automation may verify,
fail a check, retain an artifact, comment, or link a remediation handoff; it
must not originate routine product branches or PRs.

For an emergency source hotfix:

1. pause ordinary promotion;
2. obtain explicit source release authority;
3. land the smallest source fix;
4. back-port the exact fix into swarm immediately;
5. record exact transition evidence and resolution where required; and
6. re-establish a passing current-head promotion plan before normal work
   resumes.

Emergency authorization does not become standing source-side product authority.

Verify role policy directly:

```bash
cargo xtask check-automation-authority --repository-role swarm
# Run with --repository-role source in the canonical source checkout.
```

## Release handoff

Once the exact candidate promotion and closeout are green, continue with the
living
[`docs/release/release-preparation.md`](../../docs/release/release-preparation.md)
procedure. Release decision, version bump, changelog freeze, tag, crates.io,
GitHub Release publication, signing, and package-channel work remain source
release-authority operations.

## Rollback

If a promotion merge is wrong, revert the regular merge commit in
`EffortlessMetrics/shiplog` and pause further promotion and release work until
the divergence is understood.

Do not rewrite `shiplog/main` history. Do not force-push a source promotion
branch after review has started. Do not move release tags to compensate for an
incorrect promotion.

## Claim boundary

Promotion keeps `shiplog/main` current with one exact, proven swarm transaction.
It does not tag, publish to crates.io, create or publish a GitHub Release, sign
artifacts, update package channels, or move security-sensitive release
credentials to `shiplog-swarm`.
