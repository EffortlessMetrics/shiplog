# shiplog Current State

This document records the shipped v0.4.0 Review Rescue baseline after the
operator-UX lane and release verification.

## Baseline

- `main` is the released v0.4.0 product baseline.
- `v0.4.0` is published on crates.io and tagged on GitHub.
- The GitHub release has Linux x86_64, macOS x86_64, macOS arm64, Windows
  x86_64, and `SHA256SUMS.txt` assets.
- The crates.io install smoke and downloaded Windows binary smoke both reported
  `shiplog 0.4.0`.
- `docs/install.md` documents release binaries, checksum verification,
  `cargo install`, and package-manager tracking notes.
- The workspace follows the package invariant: production workspace crates are
  publishable public surfaces, and `publish = false` is reserved for dev-only
  packages.
- All workspace packages are version-aligned for the v0.4.0 release train.

## Product Contract

shiplog's product loop is:

```text
collect -> curate -> render
```

The artifact contract is:

```text
packet + ledger + coverage + bundles
```

The CLI should let a review-cycle user run a fast best-effort intake, initialize
local config, collect from configured sources, inspect coverage and gaps, curate
workstreams without hand-editing YAML, render a packet with prompts and
receipts, and produce manager/public share profiles only with an explicit
redaction key. Intake also leaves `intake.report.md` and `intake.report.json`
beside the run so readiness, skipped sources, repair hints, fixups, and share
commands are not terminal-only. The JSON report is documented as v1 in
`docs/schemas/intake-report-v1.md` with the schema at
`contracts/schemas/intake-report.v1.schema.json`; current writers include
stable repair kinds, top-fixup IDs/kinds, and a deduplicated machine-readable
`actions` array for future local UI or agent surfaces. `shiplog report
export-agent-pack` derives a compact validated `agent-pack.json` from that
report for downstream UI, agent, and support/debug tooling; its schema is
documented at `docs/schemas/agent-pack-v1.md` and
`contracts/schemas/agent-pack.v1.schema.json`.

## CLI Product Surface

First-run and setup:

- `shiplog init`
- `shiplog doctor`
- `shiplog intake`
- `shiplog config validate`
- `shiplog config explain`
- `shiplog config migrate`
- `shiplog sources status`
- `shiplog sources status --json`
- `shiplog journal add`
- `shiplog journal list`
- `shiplog journal edit`

Collection and source coverage:

- `shiplog collect github`
- `shiplog collect gitlab`
- `shiplog collect jira`
- `shiplog collect linear`
- `shiplog collect git`
- `shiplog collect json`
- `shiplog collect manual`
- `shiplog collect multi`

Collection supports relative date presets (`--last-6-months`, `--last-quarter`,
`--year`), explicit date windows, and named config periods for `intake`,
`collect multi`, and `review` via `--period <name>`. GitHub and GitLab support
`--me` identity inference when a token is available.

Curation and inspection:

- `shiplog runs list`
- `shiplog runs show`
- `shiplog runs compare --from <run> --to <run>`
- `shiplog runs compare --from-period <name> --to-period <name>`
- `shiplog runs diff --latest`
- `shiplog periods list`
- `shiplog periods explain <name>`
- `shiplog review`
- `shiplog review weekly`
- `shiplog review fixups`
- `shiplog review fixups --commands-only`
- `shiplog review fixups --journal-template`
- `shiplog review --strict`
- `shiplog open packet`
- `shiplog open workstreams`
- `shiplog open intake-report`
- `shiplog open out`
- `shiplog workstreams list`
- `shiplog workstreams validate`
- `shiplog workstreams create`
- `shiplog workstreams rename`
- `shiplog workstreams move`
- `shiplog workstreams split`
- `shiplog workstreams receipts`
- `shiplog workstreams receipt add/remove`
- `shiplog workstreams delete`
- `shiplog cache stats`
- `shiplog cache inspect`
- `shiplog cache clean`

Rendering and sharing:

- `shiplog render --mode packet`
- `shiplog render --mode scaffold`
- `shiplog render --mode receipts`
- `shiplog render --receipt-limit <N>`
- `shiplog render --appendix full|summary|none`
- `shiplog render --bundle-profile manager|public`
- `shiplog share manager|public`
- `shiplog share explain manager|public`
- `shiplog share verify manager|public`
- `shiplog share verify public --strict`
- `shiplog share verify manifest --profile manager|public`

Default CLI packets open with `Coverage and Limits`, then summary,
workstreams, receipts, and file artifacts. Internal and manager selected-profile
packet defaults use a receipt-summary appendix, with manager rendering showing
fewer main receipts. Public selected-profile rendering defaults to lower density
and omits the receipt appendix unless `--appendix` is provided. Receipts mode
keeps the full audit appendix by default.

Manager and public profiles fail closed unless `--redact-key` or the configured
redaction-key environment variable is provided.
`shiplog share explain manager|public` reports what a profile includes, removes,
and blocks without writing profile artifacts or requiring a redaction key.
`shiplog share verify manager|public` performs the same redaction-key preflight
and inspects coverage/skipped-source metadata without writing share artifacts.
`shiplog share verify public --strict` also scans the existing or in-memory
public packet for obvious raw URLs and original names; it is a guardrail, not a
perfect-privacy claim. Successful `share manager|public` writes
`profiles/<profile>/share.manifest.json` as a postflight receipt with the
profile, redaction-key source, coverage status, skipped-source count, public
strict-scan result, output paths, and SHA-256 checksums. `share verify manifest`
re-checks that receipt against the run packet and optional zip without
rendering or requiring the redaction key again.

Review evidence debt is packet-quality focused. It surfaces skipped sources,
partial coverage, missing or over-selected receipts, broad/misc/thin
workstreams, one-source-type workstreams, manual context gaps, and validation
errors with concrete next commands; it is not a productivity score. Generated
intake reports include stable `top_fixups[].id` and `top_fixups[].kind` fields
plus optional `actions` entries with id/kind/label/command/writes/risk metadata,
while validation remains compatible with older v1 reports that only recorded
fixup title/detail/command.

## Public Crate Surface

The 0.7 contraction lane makes `shiplog` the supported public package surface.
Implementation seams are moving into owner modules unless a spec and ADR
promote them to external Rust APIs. JSON schemas under `contracts/schemas/`
remain the public machine contract.

Inlined product and trust modules include:

- `shiplog::engine`
- `shiplog::coverage`
- `shiplog::cache`
- `shiplog::bundle`
- `shiplog::redact`
- `shiplog::render::md`
- `shiplog::engine::artifact_json`
- `shiplog::ingest::*`

Remaining internal support packages are tracked by
[`docs/release/0.7-crate-surface.md`](release/0.7-crate-surface.md).

Dev-only tooling:

- `shiplog-testkit` is `publish = false`.
- `fuzz/` is a fuzz harness package, not a crates.io target.

## Package Boundary

There is no durable held-production-crate category. A workspace package is
either a publishable public surface or dev-only tooling. Implementation seams
that are not public promises live as owner modules.

The release proof scripts enforce both sides of the boundary:

- `scripts/package-boundary-audit.sh` verifies published/dev-only
  classification.
- `scripts/package-version-audit.sh` verifies workspace version alignment and
  normal workspace dependency requirements.
- `scripts/package-proof.sh` runs both audits before package listing.
- `scripts/verify-release.sh <version>` verifies the public GitHub release,
  checksums, crates.io install, and current-platform release binary smoke after
  publication.

## Release Posture

v0.3.0 is shipped. The historical release record is
[`RELEASE_HANDOFF_0.3.0.md`](../RELEASE_HANDOFF_0.3.0.md), including the
published crate order, release workflow, asset set, checksum verification, and
install smoke evidence.

v0.4.0 is shipped. The release record is
[`RELEASE_HANDOFF_0.4.0.md`](../RELEASE_HANDOFF_0.4.0.md), including the
published crate order, release workflow, asset set, checksum verification, and
install smoke evidence.

The next post-v0.4 work should deepen real-world usage rather than add broad new
surface area: provider edge cases that affect first-run intake, stricter public
packet safety checks where redaction rules make them testable, and follow-on
release/distribution polish. Documented mutation baselines now include
`shiplog::coverage`, `shiplog::ids`, `shiplog::schema`, `shiplog::redact`,
`shiplog::bundle`, and a `shiplog::ports` no-target scan, recorded in
[`docs/ci/mutation.md`](ci/mutation.md).
