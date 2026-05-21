# Roadmap

This roadmap is organized by product posture, not calendar date.

## Shipped

- **v0.8.0 Evidence Repair Loop** — shipped to GitHub and crates.io. Users can
  run intake, inspect repair items, add local manual evidence from a repair ID,
  rerun, and compare repair movement.
- **Single supported public crate** — `shiplog` is the forward public package.
  Historical implementation crates remain historical artifacts; internal seams
  now live as modules.

## Release Candidate

`v0.9.0` is approved for release resume. Do not tag, publish, create a GitHub
release, dispatch release workflow, or run release-install smoke until the
release-resume PR is merged and current final preflight passes from `main`.

Release scope on `main`:

- **Review-ready packet quality** — Packet Readiness, evidence strength,
  receipt-backed claim candidates, missing-context prompts, share posture, and
  packet-quality movement.
- **Guided Setup / Doctor** — `init --guided`, `doctor --setup`,
  `sources status`, and `doctor --setup --json` make setup readiness explicit
  before intake, repair, or share commands.
- **Review-loop Status** — `status --latest` and `status --latest --json` join
  setup, latest run, packet readiness, source state, repair, diff, share
  blockers, next actions, and receipt refs into a read-only cockpit.
- **GitHub activity harvest** — `github activity plan`, `scout`, profile
  `run --resume`, `status`, `report`, and `merge` make long-history GitHub
  evidence collection budgeted, resumable, and receipt-backed.
- **Release-facing docs** — changelog, README, crate README, readiness, handoff,
  guides, schemas, matrices, and transcripts now tell one loop story:
  setup, status, intake, repair, rerun, diff, share explain.

## Next Candidates

These are future lanes, not 0.9 release promises.

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
