# Release Record: v0.4.0

Date: 2026-05-09 UTC
Release: <https://github.com/EffortlessMetrics/shiplog/releases/tag/v0.4.0>
Release workflow: <https://github.com/EffortlessMetrics/shiplog/actions/runs/25595483715>
Source commit: `ac9993470ba6234380910b67a02c9815d60db94c`

## Status

`shiplog v0.4.0` has been published to crates.io, tagged on GitHub, built for
all release targets, and smoke-tested from both crates.io and the downloaded
Windows release asset.

This file was originally the v0.4.0 handoff. It now records the shipped release
state so future work can start from verified public evidence instead of the
pre-release runbook.

## Release Summary

This release turns the post-v0.3.0 operator-UX work into a Review Rescue product
line:

- `shiplog intake --last-6-months --explain` is the default first-packet path
  for a review-deadline user.
- Intake writes durable `intake.report.md` and `intake.report.json` artifacts
  with readiness, skipped sources, repair hints, evidence debt, fixups, journal
  suggestions, share commands, and artifact paths.
- Intake reruns preserve curated `workstreams.yaml` and manual evidence while
  refreshing suggested workstreams and reporting prior curation.
- `shiplog review`, `review weekly`, `review --strict`, and `review fixups`
  inspect packet quality without productivity scoring or generated claims.
- `shiplog journal add/list/edit` captures and corrects factual manual evidence
  without requiring hand-edited YAML.
- `shiplog share manager|public` and `shiplog share verify manager|public`
  provide fail-closed share paths.
- `shiplog share verify public --strict` adds a read-only guardrail scan for
  obvious raw URLs and original names in public packets.
- Named config periods make review windows repeatable across `intake`,
  `collect multi`, and `review`.
- The release includes recorded provider payload coverage, all-source fixture
  packets, intake golden fixtures, cache replay coverage, provider-edge smoke
  fixtures, and documented coverage/mutation baselines.

## Published Crates

Published to crates.io in dependency order:

1. `shiplog-ids`
2. `shiplog-schema`
3. `shiplog-ports`
4. `shiplog-coverage`
5. `shiplog-cache`
6. `shiplog-redact`
7. `shiplog-bundle`
8. `shiplog-workstreams`
9. `shiplog-merge`
10. `shiplog-render-md`
11. `shiplog-render-json`
12. `shiplog-ingest-json`
13. `shiplog-ingest-manual`
14. `shiplog-ingest-git`
15. `shiplog-ingest-github`
16. `shiplog-ingest-gitlab`
17. `shiplog-ingest-jira`
18. `shiplog-ingest-linear`
19. `shiplog-cluster-llm`
20. `shiplog-team`
21. `shiplog-engine`
22. `shiplog`

Non-publishable:

- `shiplog-testkit`
- `shiplog-fuzz`

## Release Validation

Pre-publish:

```bash
scripts/package-proof.sh
```

Publication:

```bash
cargo publish -p <package> --dry-run
cargo publish -p <package>
```

After all crates were published:

```bash
scripts/publish-dry-run.sh
```

The final dry-run completed successfully after every upstream `0.4.0` crate was
visible in the crates.io index.

## GitHub Release Assets

The GitHub release workflow completed successfully and the public release page
returned HTTP 200. Uploaded assets:

- `shiplog-x86_64-unknown-linux-gnu`
- `shiplog-x86_64-apple-darwin`
- `shiplog-aarch64-apple-darwin`
- `shiplog-x86_64-pc-windows-msvc.exe`
- `SHA256SUMS.txt`

The assets were downloaded from the public release URLs and verified against
`SHA256SUMS.txt`.

## Smoke Tests

The crates.io install smoke was run under an isolated install root:

```bash
cargo install shiplog --version 0.4.0 --locked
shiplog --version
shiplog init --dry-run
shiplog intake --help
shiplog review --help
shiplog share verify manager --help
shiplog share verify public --help
```

The downloaded Windows release binary was also smoke-tested with:

```bash
shiplog-x86_64-pc-windows-msvc.exe --version
shiplog-x86_64-pc-windows-msvc.exe --help
shiplog-x86_64-pc-windows-msvc.exe init --dry-run
shiplog-x86_64-pc-windows-msvc.exe collect --help
shiplog-x86_64-pc-windows-msvc.exe collect multi --help
shiplog-x86_64-pc-windows-msvc.exe render --help
```

Both paths reported `shiplog 0.4.0`.

Future releases can repeat the public verification path with:

```bash
scripts/verify-release.sh v0.4.0
```

The helper checks the GitHub release state, expected binary assets,
`SHA256SUMS.txt`, crates.io install, installed-binary smoke, and the
current-platform downloaded release asset. During this release, the GitHub API
became rate-limited after the tag workflow completed, so the asset verification
was repeated directly through public release download URLs.

## Known Release Notes

- This is an operator-UX release, not a package-boundary reshaping release.
- Review findings are packet-quality guidance, not productivity scoring.
- LLM clustering remains optional and off by default.
- Manager/public packets and bundles still require explicit redaction keys.
- Strict public verification is a guardrail for obvious raw URL/name leaks, not
  a guarantee of perfect privacy.
