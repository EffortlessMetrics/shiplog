# Per-PR Acceptance Contract

Every PR in the v0.5.0 rollout (#140–#157) — and likely every
policy/release PR going forward — declares an acceptance contract in its
PR description. The contract is a self-binding receipt: if the PR's
declared scope and the actual diff diverge, the reviewer (and the
author's own self-review) catches it immediately.

This is the human contract template. It exists to keep agents and
contributors from quietly broadening scope, missing the "advisory not
blocking" framing, or claiming behavior changes the PR did not make.

## The block

```markdown
## PR acceptance contract

- **Scope:** <one sentence describing what this PR does>
- **Files expected:** <list or globs>
- **Behavior change:** yes / no
- **Advisory vs blocking:** advisory / blocking / none
- **New artifacts:** <list or "none">
- **Validation commands:** <exact commands>
- **Rollback path:** <how to revert; e.g. "git revert <sha>; no ledger reset needed">
- **Follow-up PR:** <#NNN or "none">
```

Add this as a first-class section in every rollout PR description,
alongside Summary and Test plan.

## Field-by-field

### Scope

One sentence. Not a list of bullet points. If the scope cannot fit in
one sentence, the PR is too big — split it.

Good: `"Adds 18 parse-only TOML policy ledgers + a policy/README.md orientation doc."`

Bad: `"Adds policy ledgers and refactors the existing release scripts and updates the workflow file and..."`

### Files expected

List the files (or globs) the PR will touch. After the PR is open,
reviewers can compare against the actual diff. A divergence is a
scope-creep signal.

Reasonable shapes:

- Explicit list: `policy/ci-budget.toml, policy/ci-lanes.toml, ...`
- Globs: `policy/*.toml + policy/README.md (19 files)`
- Areas: `docs/ci/* (13 files) + contracts/schemas/* (2 files)`

A glob with a count is the most reviewable form for medium-sized PRs.

### Behavior change

`yes` or `no`.

- `no` for docs-only PRs, TOML skeleton PRs, schema docs, etc.
- `yes` for any PR that changes runtime behavior, CI behavior,
  enforcement, or the release pipeline.

If the PR has both, split it. A single PR should have one answer.

### Advisory vs blocking

What CI lane status does this PR introduce or change?

- `advisory` — adds an advisory lane, or adds a check that does not
  block merge
- `blocking` — adds or modifies a required-merge gate
- `none` — adds neither (e.g. docs PR)

Most PRs in the v0.5.0 rollout are `advisory` or `none`. The first
`blocking` PR is #146 (adds `pr-plan / forecast` as required), and
hard CI budget enforcement is deferred past v0.5.0.

### New artifacts

What persistent artifacts does this PR introduce? Examples:

- TOML ledger files
- JSON schema files
- New CI workflow
- New step-summary output
- New release artifact

`none` is acceptable. If listing more than ~6 artifacts, summarize:
`"18 TOML ledgers + 1 README"`.

### Validation commands

Exact commands a reviewer can run locally to confirm the PR's claims.
Should at minimum include:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
git diff --check
```

Plus PR-specific gates. For #141 (TOML skeletons):

```text
python -c "import tomllib, glob; [tomllib.load(open(p,'rb')) for p in sorted(glob.glob('policy/*.toml'))]"
```

For PRs that add `xtask` commands (#143 onward):

```text
cargo xtask <command> --check
```

### Rollback path

A single command (or short procedure) that reverts the PR's changes.

For docs / TOML PRs: `git revert <merge-sha>` is usually sufficient. No
ledger reset, no schema migration.

For PRs that change enforcement: include any required follow-up steps
(e.g. "git revert; then `cargo xtask no-panic baseline --reset` to
clear the post-revert baseline state").

For PRs that change required checks: include the branch-protection
adjustment needed.

### Follow-up PR

Which subsequent PR depends on this PR's work, or which subsequent PR
addresses a deferred concern. Use `#NNN` for known PRs, or describe
("PR #142 — adds the operating-contract docs that reference these
ledgers"). `none` is acceptable.

## Main-branch post-merge audit

After every rollout PR merges to `main`, the agent (or maintainer)
runs this checklist:

```text
- [ ] `git switch main && git pull` — local main fast-forwarded
- [ ] `gh run list --branch main --limit 5` — `main` post-merge CI green
- [ ] No open PRs stuck on the merged branch
- [ ] (if applicable) `cargo xtask policy-report` clean — once xtask exists (#143)
- [ ] (if applicable) `python -c "import tomllib, glob; [tomllib.load(open(p,'rb')) for p in sorted(glob.glob('policy/*.toml'))]"` clean — until xtask exists
- [ ] Memory updated to reflect merge state (project memory: ladder progress)
```

This audit is what closes the loop on a merged PR. Without it, the
next slice can start from a stale assumption.

## Self-review correspondence

The per-PR acceptance contract overlaps with the self-review checklist
in [`rust-1.95-rollout.md`](rust-1.95-rollout.md#self-review-checklist).
The two have different purposes:

- The **acceptance contract** is in the PR description (for reviewers).
  It states what the PR will do.
- The **self-review** is a PR comment (for the author + reviewers).
  It confirms what the PR actually did and that gates passed.

Both must be filled out. The contract is the design intent; the
self-review is the verification.

## What this contract is not

- Not a substitute for a real description. The PR still needs Summary,
  Files added (with semantic grouping), and Test plan sections.
- Not a one-time gate. Reviewers can update the contract during review
  if the PR's scope legitimately evolves; the diff between the original
  and updated contract is itself reviewable.
- Not enforced by tooling in v0.5.0. The contract is a discipline.
  PR #146 may add a check that the section exists (lint-style); strict
  parsing is a follow-up release.

## See also

- [`rust-1.95-rollout.md`](rust-1.95-rollout.md#per-pr-operating-contract) — per-PR operating contract (the rules the PR must follow)
- [`rust-1.95-rollout.md`](rust-1.95-rollout.md#self-review-checklist) — self-review checklist (post-CI verification)
- [`branch-protection.md`](branch-protection.md) — what merge actually gates on
- [`labels.md`](labels.md) — labels that influence the contract (spend, routing)
