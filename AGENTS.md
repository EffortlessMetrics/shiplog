# Agent Context for shiplog

This file provides guidance for AI agents and code review automation (Factory Droid) working in this repository.

## Development Control Plane

Normal development now targets `EffortlessMetrics/shiplog-swarm`.

Use `EffortlessMetrics/shiplog-swarm` for focused product, docs, tests, CI,
policy, and cleanup PRs. Squash-merge normal swarm PRs after
`Shiplog Rust Small Result` passes. Do not push directly to
`shiplog-swarm/main`.

`EffortlessMetrics/shiplog` remains the release/public source surface until an
explicit release-authority cutover. It owns tags, crates.io publish, GitHub
Releases, signing, release branches, release workflows, and security-sensitive
token operations.

When working from a checkout that has both remotes:

```bash
git fetch origin --prune
git fetch swarm --prune
git switch -c <branch> swarm/main
```

Push normal development branches to `swarm` and open PRs against
`EffortlessMetrics/shiplog-swarm/main`. Do not retarget an existing
`EffortlessMetrics/shiplog` clone in place for new normal work; clone
`shiplog-swarm` side-by-side when setting up a fresh agent or machine.

Promotion from swarm back to source uses `cargo xtask promote` and a regular
merge-commit PR into `EffortlessMetrics/shiplog/main`, never a squash merge.
Do not push raw `swarm/main` as a source promotion branch or rebuild the
per-path overlay manually. Run the current exact-head dry-run twice, inspect the
machine path decisions and included receipts, execute the idempotent command,
and verify the landed checkpoint with `--verify-only`. A fail-closed result is
a repair queue; do not bypass it with an older source ref or historical mode.
Use
[`plans/shiplog-swarm/promotion-runbook.md`](plans/shiplog-swarm/promotion-runbook.md)
for the complete promotion and closeout sequence.

Shared release-candidate preparation also belongs in swarm. Finish the release
scope, version and lockfile, changelog freeze, README/guides, release decision,
readiness ledger, and handoff skeleton on `shiplog-swarm/main`, then promote the
exact proven candidate. Source work begins only after that checkpoint and is
limited to release execution, tagging, publication, and explicitly authorized
source-owned writer configuration. Use
[`docs/release/release-preparation.md`](docs/release/release-preparation.md) for
the current preparation, preflight, tag, staged-artifact, publication,
rollback, and closeout procedure. Versioned readiness/decision files and root
`RELEASE_HANDOFF_*` files are historical receipts, not standing command
references. A product or shared-documentation defect found on source returns to
swarm and is promoted again.

## Code Review Standards

### Finding Format

Use this structure for actionable findings:

```
[P0|P1|P2] Short title

Failure mode:
Why here:
Fix direction:
Validation:
Confidence:
```

### Clean Review Format

When no actionable findings are emitted:

```
No actionable findings emitted.
Inspected surfaces:
Checks performed:
Why no comments:
Residual risk:
Validation signal:
  Observed:
  Reported:
  Not verified:
```

### Review Principles

- **No naked LGTM**: Approval requires explicit validation signals
- **No arbitrary comment cap**: All identified issues are reported
- **No extra @mentions**: Findings are directed only to the PR author and reviewers
- **Actionable findings**: Issues are repair packets with clear fix direction
- **Clean reviews**: Include inspection record with evidence provenance
- **Evidence split**: Observed (from running code/tests) / Reported (from tool/CI output) / Not verified (unconfirmed claims)
- **PR-body validation claims**: Not treated as independently verified; require confirmation

## Droid Automation

### Auto Review

Droid auto-reviews all non-draft PRs from the same repository.

- Trigger: `pull_request` (opened, synchronize, ready_for_review, reopened)
- Guard: Same-repo origin only; [skip-review] tag bypasses
- Permissions: `contents: write` (for review publication)
- Model: `custom:MiniMax-M3-0`
- Depth: `shallow`
- Secrets: `FACTORY_API_KEY` and `MINIMAX_API_KEY`; runs skip if either is unavailable
- No raw debug artifacts uploaded

### Manual @droid Commands

Trusted actors (OWNER, MEMBER, COLLABORATOR) can invoke Droid manually:

```
@droid review       # Request code review
@droid security     # Request security analysis
```

- Guard: Author must be trusted actor
- Permissions: `contents: read` (manual requests are read-only)
- Model: `custom:MiniMax-M3-0`
- Depth: `shallow`
- Secrets: `FACTORY_API_KEY` and `MINIMAX_API_KEY`; runs skip if either is unavailable

### Scheduled Security Scan

Weekly Monday 08:00 UTC full repository security scan.

- Trigger: Schedule + manual workflow dispatch
- Permissions: `contents: write` (for scan report publication)
- Model: `custom:MiniMax-M3-0`
- Threshold: Medium
- Secrets: `FACTORY_API_KEY` and `MINIMAX_API_KEY`; runs skip if either is unavailable
- Critical issues block; High issues reported only

## References

- Droid action: `EffortlessMetrics/droid-action-safe@7c1377ccbacddc95560d1570547a5baa51de01ec`
- MiniMax model: `custom:MiniMax-M3-0`
- LLM provider: Anthropic API (via MiniMax BYOK bridge)
