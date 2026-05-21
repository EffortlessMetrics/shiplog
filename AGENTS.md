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
rtk git fetch origin --prune
rtk git fetch swarm --prune
rtk git switch -c <branch> swarm/main
```

Push normal development branches to `swarm` and open PRs against
`EffortlessMetrics/shiplog-swarm/main`. Do not retarget an existing
`EffortlessMetrics/shiplog` clone in place for new normal work; clone
`shiplog-swarm` side-by-side when setting up a fresh agent or machine.

Promotion from swarm back to source uses a regular merge-commit PR into
`EffortlessMetrics/shiplog/main`, not a squash merge. A promotion PR should
include the swarm head SHA, included swarm PRs, and proof that
`Shiplog Rust Small Result` passed.

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
- Model: `custom:MiniMax-M2.7-0`
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
- Model: `custom:MiniMax-M2.7-0`
- Depth: `shallow`
- Secrets: `FACTORY_API_KEY` and `MINIMAX_API_KEY`; runs skip if either is unavailable

### Scheduled Security Scan

Weekly Monday 08:00 UTC full repository security scan.

- Trigger: Schedule + manual workflow dispatch
- Permissions: `contents: write` (for scan report publication)
- Model: `custom:MiniMax-M2.7-0`
- Threshold: Medium
- Secrets: `FACTORY_API_KEY` and `MINIMAX_API_KEY`; runs skip if either is unavailable
- Critical issues block; High issues reported only

## References

- Droid action: `EffortlessMetrics/droid-action-safe@01e76b659e4b1e5f23feedc8cfabf8dc14c7485f`
- MiniMax model: `custom:MiniMax-M2.7-0`
- LLM provider: Anthropic API (via MiniMax BYOK bridge)
