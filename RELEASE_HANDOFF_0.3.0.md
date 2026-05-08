# Release Record: v0.3.0

Date: 2026-05-08 UTC
Release: <https://github.com/EffortlessMetrics/shiplog/releases/tag/v0.3.0>
Release workflow: <https://github.com/EffortlessMetrics/shiplog/actions/runs/25529576221>
Source commit: `5a8214436b867214061a3009ecc1d9dadaa522e3`

## Status

`shiplog v0.3.0` has been published to crates.io, tagged on GitHub, built for
all release targets, and smoke-tested from both crates.io and the downloaded
Windows release asset.

This file was originally the v0.3.0 handoff. It now records the shipped release
state so future work can start from verified public evidence instead of the
pre-release runbook.

## Release Summary

This release turns the post-v0.2.1 product work into a coherent installable
surface:

- GitHub, GitLab, Jira, Linear, local git, JSON, and manual sources are
  CLI-visible.
- `collect multi`, `merge`, runs/open/cache commands, config validation and
  migration, and workstream curation commands are part of the product path.
- Packet rendering includes coverage/gap summaries, evidence anchors, claim
  prompts, render modes, receipt limits, and appendix controls.
- Manager/public share profiles fail closed without a real redaction key.
- Workspace package versions and normal workspace dependency requirements are
  aligned at `0.3.0`.

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

The final dry-run completed successfully after every upstream `0.3.0` crate was
visible in the crates.io index.

## GitHub Release Assets

The GitHub release is live, non-draft, and non-prerelease. Uploaded assets:

- `shiplog-x86_64-unknown-linux-gnu`
- `shiplog-x86_64-apple-darwin`
- `shiplog-aarch64-apple-darwin`
- `shiplog-x86_64-pc-windows-msvc.exe`
- `SHA256SUMS.txt`

The assets were downloaded and verified against `SHA256SUMS.txt`.

## Smoke Tests

The crates.io install smoke was run under an isolated install root:

```bash
cargo install shiplog --version 0.3.0 --locked
shiplog --version
shiplog init --dry-run
shiplog collect --help
shiplog collect multi --help
shiplog render --help
```

The downloaded Windows release binary was also smoke-tested with:

```bash
shiplog-x86_64-pc-windows-msvc.exe --version
shiplog-x86_64-pc-windows-msvc.exe --help
shiplog-x86_64-pc-windows-msvc.exe init --dry-run
shiplog-x86_64-pc-windows-msvc.exe collect multi --help
shiplog-x86_64-pc-windows-msvc.exe render --help
```

Both paths reported `shiplog 0.3.0`.

Future releases can repeat the public verification path with:

```bash
scripts/verify-release.sh v0.3.0
```

The helper checks the GitHub release state, expected binary assets,
`SHA256SUMS.txt`, crates.io install, installed-binary smoke, and the
current-platform downloaded release asset.

## Follow-up Disposition

Issue #61, `ci(windows): investigate windows-latest check failure`, was closed
as stale after current `main` and the v0.3.0 release workflow both showed a
successful Windows check/build.

## Known Release Notes

- This release is a CLI/product surface release, not another package-boundary
  migration.
- LLM clustering remains optional and off by default.
- Manager/public packets and bundles require explicit redaction keys.
