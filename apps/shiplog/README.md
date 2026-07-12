# shiplog

[![crates.io](https://img.shields.io/crates/v/shiplog.svg)](https://crates.io/crates/shiplog)
[![docs.rs](https://docs.rs/shiplog/badge.svg)](https://docs.rs/shiplog)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

> Review evidence loop for people who need receipts, not review prose.

Current shipped release: `v0.11.0`.

shiplog turns work evidence into a review-readiness habit: capture context,
refresh receipts, inspect gaps, and share safely.

## Install

```bash
cargo install shiplog --locked
```

With optional LLM-assisted workstream clustering:

```bash
cargo install shiplog --locked --features llm
```

Prerequisites: Rust 1.95+. Token-backed sources use environment variables such
as `GITHUB_TOKEN`, `GITLAB_TOKEN`, `JIRA_TOKEN`, or `LINEAR_API_KEY`; local git,
JSON, and manual evidence can run without provider tokens.

## What you get

| Surface | Command |
|---------|---------|
| First packet | `shiplog intake` |
| Home screen | `shiplog` |
| Quick context | `shiplog add "what changed"` |
| Evidence refresh | `shiplog update` |
| Safe next action | `shiplog next` |
| Current packet | `shiplog open` |
| Manager/public share | `shiplog share manager` / `shiplog share public` |
| Review-loop status | `shiplog status --latest` |
| Full GitHub history | `shiplog github activity plan` |

## First useful loop

Start with the first packet. Setup files are created automatically when they
are needed, and provider credentials are optional:

```bash
shiplog intake
```

Open the packet when you want to read it immediately:

```bash
shiplog open
```

Normal recurring use:

```bash
shiplog add "Resolved the customer import retry incident" \
  --impact "Protected the next import window"
shiplog update
shiplog open
shiplog
```

For setup troubleshooting or agent automation, use `doctor --setup`,
`sources status`, and `status --latest --json` as read-only diagnostics.

Repair and compare when status says the packet needs evidence:

```bash
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog runs diff --latest
shiplog share explain manager --latest
```

## Read vs write

Read-only commands:

- `shiplog doctor --setup`
- `shiplog sources status`
- `shiplog doctor --setup --json`
- `shiplog status --latest`
- `shiplog status --latest --json`
- `shiplog next` / `shiplog next --json`
- `shiplog`
- `shiplog repair plan --latest`
- `shiplog repair diff --latest`
- `shiplog runs diff --latest`
- `shiplog share explain manager --latest`
- `shiplog github activity status --out ./out/github-full`

Write-producing commands:

- `shiplog init --guided`
- `shiplog intake --last-6-months --explain`
- `shiplog add "what changed"`
- `shiplog update`
- `shiplog journal add --from-repair <repair_id>`
- `shiplog github activity report --out ./out/github-full`
- `shiplog github activity merge --out ./out/github-full`
- explicit `shiplog share manager|public` rendering commands

`doctor`, `status`, and `share explain` do not render profile artifacts. Use
explicit share commands only after share explanation or verification says the
profile is ready.

## Agent-readable surfaces

- `shiplog doctor --setup --json` exposes setup readiness without provider
  probing, writes, or secret values.
- `shiplog status --latest --json` exposes the review-loop state, blockers,
  next actions, write posture, and receipt references without scraping terminal
  prose.
- `shiplog report export-agent-pack --latest` exports report receipts for
  support and automation workflows.

Agents should consume JSON receipts, not `packet.md` or terminal prose.

## Read next

| Need | Doc |
|------|-----|
| First run | [Rapid first-intake guide](https://github.com/EffortlessMetrics/shiplog/blob/main/docs/guides/rapid-first-intake.md) |
| Setup | [Guided setup and doctor guide](https://github.com/EffortlessMetrics/shiplog/blob/main/docs/guides/guided-setup-doctor.md) |
| Recurring use | [Recurring review-loop guide](https://github.com/EffortlessMetrics/shiplog/blob/main/docs/guides/recurring-review-loop.md) |
| Full GitHub history | [GitHub activity harvest guide](https://github.com/EffortlessMetrics/shiplog/blob/main/docs/guides/github-activity-harvest.md) |
| Repair | [Evidence repair loop guide](https://github.com/EffortlessMetrics/shiplog/blob/main/docs/guides/evidence-repair-loop.md) |
| Review-ready packet and share posture | [Review-ready packet guide](https://github.com/EffortlessMetrics/shiplog/blob/main/docs/guides/review-ready-packet.md) |
| Configuration | [Config reference](https://github.com/EffortlessMetrics/shiplog/blob/main/docs/config-reference.md) |
| Status JSON contract | [Review-loop status schema](https://github.com/EffortlessMetrics/shiplog/blob/main/docs/schemas/review-loop-status-v1.md) |
| Release history | [CHANGELOG](https://github.com/EffortlessMetrics/shiplog/blob/main/CHANGELOG.md) |

## What shiplog does not do

- Does not write performance-review prose.
- Does not score employees.
- Does not mutate provider records.
- Does not query providers from `doctor` or `status`.
- Does not render manager/public packets from `status` or `share explain`.
- Does not enable optional LLM clustering unless built with `--features llm`
  and configured explicitly.

## License

Dual licensed under [MIT](https://github.com/EffortlessMetrics/shiplog/blob/main/LICENSE-MIT) OR [Apache-2.0](https://github.com/EffortlessMetrics/shiplog/blob/main/LICENSE-APACHE), at your option.
