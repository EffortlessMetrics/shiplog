# Agent Context for shiplog

This file provides guidance for AI agents and code review automation (Factory Droid) working in this repository.

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
