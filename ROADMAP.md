# Roadmap

This roadmap is organized by product posture, not calendar date.

## Shipped

- **v0.9.0 Review Loop Cockpit** - shipped to GitHub and crates.io. Users can
  set up safely, inspect status, collect receipts, repair gaps, rerun, compare,
  and explain share posture.
- **Single supported public crate** - `shiplog` is the forward public package.
  Historical implementation crates remain historical artifacts; internal seams
  now live as modules.
- **v0.11.0 Low-Friction Review Readiness** - the one-command front door,
  objective setup, GitHub credential reuse, home/next/update/add/open workflow,
  packet-first output, safe share preflight, installers, Homebrew, and Scoop
  are merged and ready for release.

Current shipped release: `v0.11.0`.

## Previous shipped release

`v0.10.0` shipped the source configuration ergonomics and LLM endpoint
security slices. The public release, crates.io package, checksums, and install
smoke are complete.

Release scope on `main`:

- **Source configuration ergonomics** - `sources list`, `sources enable`, and
  `sources disable` expose local source state and toggle only the `enabled`
  assignment while preserving comments, provider records, and tokens.
- **LLM endpoint security** - opt-in OpenAI-compatible clustering rejects
  malformed and non-HTTPS endpoints before any request or authorization header.
- **Release contract maintenance** - package metadata, changelog, readiness,
  decision, handoff, and post-release proof describe `0.10.0`.

## Next Candidates

These are future lanes, not `0.11.0` release promises.

- **Guided first use** - add an explicit confirmed `shiplog start` command.
- **Provider connection** - add task-level GitLab, Jira, and Linear connection
  flows that preserve non-secret configuration.
- **Share setup ergonomics** - provide safe local/OS-backed redaction-key
  setup without weakening fail-closed rendering.

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

- [`docs/release/0.11.0-readiness.md`](docs/release/0.11.0-readiness.md)
- [`docs/release/0.11.0-release-decision.md`](docs/release/0.11.0-release-decision.md)
- [`RELEASE_HANDOFF_0.11.0.md`](RELEASE_HANDOFF_0.11.0.md)
