# Release Handoff: v0.4.0

Date prepared: 2026-05-09 UTC
Release theme: Review Rescue
Release status: prepared, not published
Source commit: use the merge commit for the v0.4.0 release-prep PR

## Status

`shiplog v0.4.0` is version-aligned and ready for final proof, publication,
tagging, release-asset generation, and install smoke. Do not describe v0.4.0 as
shipped until crates.io, GitHub release assets, checksums, and install smoke
have all been verified.

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

Publish to crates.io in dependency order:

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

Before publication:

```bash
git switch main
git pull --ff-only origin main
scripts/package-proof.sh
```

Publication is manual and ordered. For each package:

```bash
cargo publish -p <package> --dry-run
cargo publish -p <package>
```

Downstream dry-runs can fail until upstream `0.4.0` crates are visible in the
crates.io index. Resume dry-runs with:

```bash
scripts/publish-dry-run.sh --from <package>
```

After all crates are published:

```bash
scripts/publish-dry-run.sh
```

Then tag:

```bash
git tag -a v0.4.0 -m "Release v0.4.0"
git push origin v0.4.0
```

## Expected GitHub Release Assets

The release workflow should produce:

- `shiplog-x86_64-unknown-linux-gnu`
- `shiplog-x86_64-apple-darwin`
- `shiplog-aarch64-apple-darwin`
- `shiplog-x86_64-pc-windows-msvc.exe`
- `SHA256SUMS.txt`

## Post-Release Verification

After the tag workflow completes:

```bash
scripts/verify-release.sh v0.4.0
```

Also smoke the installed CLI path explicitly:

```bash
cargo install shiplog --version 0.4.0 --locked
shiplog --version
shiplog init --dry-run
shiplog intake --help
shiplog review --help
shiplog share verify manager --help
shiplog share verify public --help
```

## Known Release Notes

- This is an operator-UX release, not a package-boundary reshaping release.
- Review findings are packet-quality guidance, not productivity scoring.
- LLM clustering remains optional and off by default.
- Manager/public packets and bundles still require explicit redaction keys.
- Strict public verification is a guardrail for obvious raw URL/name leaks, not
  a guarantee of perfect privacy.
