# User-Polish Plan Lane

This directory owns the completed implementation sequencing for the
user-polish lane. The lane was intentionally narrow: make the first-run
review-pack path obvious, receipt-backed, and safe to extend.

The target user path is:

```bash
shiplog intake --last-6-months --explain
shiplog open intake-report --latest
shiplog open packet --latest
```

After running it, a user should know where output went, which sources worked,
which sources were skipped, whether evidence was fresh or cached, whether an
artifact is safe to share, what needs repair, and what command to run next.

## What Belongs Here

Plans sequence PRs. They should include:

- dependency order;
- scoped PR titles;
- touched surfaces;
- proof commands;
- rollback notes;
- open questions that block sequencing.

Plans should link specs for behavior contracts and ADRs for architecture
decisions. They should not duplicate specs, user guides, or policy ledgers.

## Planned Files

The lane may add focused plan files as the source-of-truth stack lands:

```text
implementation-plan.md
cli-next-steps.md
open-latest.md
source-identity.md
freshness-stale.md
release.md
```

The active goal for this lane is archived at
[`../../.shiplog/goals/archive/2026-05-13-user-polish-0.6.0.toml`](../../.shiplog/goals/archive/2026-05-13-user-polish-0.6.0.toml).
Use the implementation plan as a historical receipt, not as the current Codex
work queue.
