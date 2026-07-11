# Roadmap

This roadmap is organized by product posture, not calendar date.

## Shipped

- **v0.9.0 Review Loop Cockpit** - shipped to GitHub and crates.io. Users can
  set up safely, inspect status, collect receipts, repair gaps, rerun, compare,
  and explain share posture.
- **Single supported public crate** - `shiplog` is the forward public package.
  Historical implementation crates remain historical artifacts; internal seams
  now live as modules.

## Release Candidate

`v0.10.0` is prepared for final merged-main preflight. Do not tag, publish,
create a GitHub release, dispatch the release workflow, or run release-install
smoke until the release-prep PR is merged and the current final preflight
passes from `main`.

Release scope on `main`:

- **Source configuration ergonomics** - `sources list`, `sources enable`, and
  `sources disable` expose local source state and toggle only the `enabled`
  assignment while preserving comments, provider records, and tokens.
- **LLM endpoint security** - opt-in OpenAI-compatible clustering rejects
  malformed and non-HTTPS endpoints before any request or authorization header.
- **Release contract maintenance** - package metadata, changelog, readiness,
  decision, handoff, and final preflight instructions describe `0.10.0`.

## Next Candidates

These are future lanes, not 0.10 release promises.

- **Guided provider readiness** - improve token/setup guidance without OAuth or
  live provider probing by default.
- **Public-share happy path proof** - dogfood public share with redaction setup
  and strict verification once release priorities justify it.
- **Recurring workflow support** - use `status --latest --json` as the safe
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

- [`docs/release/0.10.0-readiness.md`](docs/release/0.10.0-readiness.md)
- [`docs/release/0.10.0-release-decision.md`](docs/release/0.10.0-release-decision.md)
- [`RELEASE_HANDOFF_0.10.0.md`](RELEASE_HANDOFF_0.10.0.md)
