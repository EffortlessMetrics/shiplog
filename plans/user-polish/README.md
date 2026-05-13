# User-Polish Plan Lane

This directory owns implementation sequencing for the user-polish lane. The
lane is intentionally narrow: make the first-run review-pack path obvious,
receipt-backed, and safe to extend.

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

The first implementation plan should make the next Codex or Droid move clear
from the active goal manifest without rereading old conversations.
