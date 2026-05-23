# Active Goals

Agents should read `.codex/goals/active.toml` first, then follow linked plans,
specs, and proposals before making changes.

Status values:

- `ready`
- `active`
- `blocked`
- `done`
- `superseded`

Use `commands` for current agent proof commands. Commands in active or ready
work items must be copyable repo commands and must start with `rtk `. Use
`receipts` for merged PRs, CI runs, or other durable proof refs when a work item
is already done. A blocked work item must name `blocked_by`; a superseded work
item must name `superseded_by`.
