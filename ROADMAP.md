# Roadmap

This roadmap is organized by product posture, not calendar date.

## Shipped

- **v0.10.0 Usable review front door** - shipped to GitHub and crates.io on
  2026-07-11. Users can start from an empty directory with `shiplog intake`,
  reuse authenticated GitHub CLI credentials, inspect objective-scoped setup,
  maintain evidence with `add` and `update`, and install verified prebuilt
  binaries without Rust.
- **v0.8.0 Evidence Repair Loop** - shipped to GitHub and crates.io. Users can
  run intake, inspect repair items, add local manual evidence from a repair ID,
  rerun, and compare repair movement.
- **Single supported public crate** — `shiplog` is the forward public package.
  Historical implementation crates remain historical artifacts; internal seams
  now live as modules.

## Next Candidates

These are future lanes, not 0.10 release promises.

- **Guided provider readiness** — improve token/setup guidance without OAuth or
  live provider probing by default.
- **Public-share happy path proof** — dogfood public share with redaction setup
  and strict verification once release priorities justify it.
- **Source enable/disable ergonomics** — make optional provider setup easier
  without mutating provider records.
- **Recurring workflow support** — use `status --latest --json` as the safe
  primitive for future reminders or scheduled checks.

## Later

- OAuth readiness as another credential backend.
- Team or manager rollups after single-user status remains stable.
- TUI/GUI/dashboard exploration only after the CLI/JSON control plane is
  boring and proven.
- Plugin APIs or new adapters after the core review loop is released and
  observed.

## Non-goals

- Generated performance-review prose.
- Employee scoring.
- Provider record mutation.
- Live provider probing from `doctor` or `status`.
- Automatic repair or automatic share rendering.
- Telemetry, tracking, or account requirements.

## Release Posture

See:

- [`docs/release/0.9.0-release-hold-lifted.md`](docs/release/0.9.0-release-hold-lifted.md)
- [`docs/release/0.9.0-readiness.md`](docs/release/0.9.0-readiness.md)
- [`docs/release/0.9.0-release-decision.md`](docs/release/0.9.0-release-decision.md)
