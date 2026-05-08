# shiplog Current State

This document records the shipped v0.3.0 baseline after the product completion
pass and release verification.

## Baseline

- `main` is the released v0.3.0 product baseline.
- `v0.3.0` is published on crates.io and tagged on GitHub.
- The GitHub release has Linux x86_64, macOS x86_64, macOS arm64, Windows
  x86_64, and `SHA256SUMS.txt` assets.
- The crates.io install smoke and downloaded Windows binary smoke both reported
  `shiplog 0.3.0`.
- The workspace follows the package invariant: production workspace crates are
  publishable public surfaces, and `publish = false` is reserved for dev-only
  packages.
- All workspace packages are version-aligned for the v0.3.0 release train.

## Product Contract

shiplog's product loop is:

```text
collect -> curate -> render
```

The artifact contract is:

```text
packet + ledger + coverage + bundles
```

The CLI should let a review-cycle user initialize local config, collect from
configured sources, inspect coverage and gaps, curate workstreams without
hand-editing YAML, render a packet with prompts and receipts, and produce
manager/public share profiles only with an explicit redaction key.

## CLI Product Surface

First-run and setup:

- `shiplog init`
- `shiplog doctor`
- `shiplog config validate`
- `shiplog config explain`
- `shiplog config migrate`
- `shiplog journal add`

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
`--year`) and explicit date windows. GitHub and GitLab support `--me` identity
inference when a token is available.

Curation and inspection:

- `shiplog runs list`
- `shiplog runs show`
- `shiplog review`
- `shiplog review --strict`
- `shiplog open packet`
- `shiplog open workstreams`
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

Default CLI packets open with `Coverage and Limits`, then summary,
workstreams, receipts, and file artifacts. Internal and manager selected-profile
packet defaults use a receipt-summary appendix, with manager rendering showing
fewer main receipts. Public selected-profile rendering defaults to lower density
and omits the receipt appendix unless `--appendix` is provided. Receipts mode
keeps the full audit appendix by default.

Manager and public profiles fail closed unless `--redact-key` or the configured
redaction-key environment variable is provided.

## Public Crate Surface

Stable contracts:

- `shiplog-ids`
- `shiplog-schema`
- `shiplog-ports`

Product and trust surfaces:

- `shiplog-engine`
- `shiplog-coverage`
- `shiplog-workstreams`
- `shiplog-redact`
- `shiplog-bundle`
- `shiplog-cache`
- `shiplog-render-md`
- `shiplog-render-json`
- `shiplog-merge`

Source adapters:

- `shiplog-ingest-github`
- `shiplog-ingest-git`
- `shiplog-ingest-json`
- `shiplog-ingest-manual`
- `shiplog-ingest-gitlab`
- `shiplog-ingest-jira`
- `shiplog-ingest-linear`

Optional feature surfaces:

- `shiplog-cluster-llm`
- `shiplog-team`

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

v0.3.0 is shipped. The release record is
[`RELEASE_HANDOFF_0.3.0.md`](../RELEASE_HANDOFF_0.3.0.md), including the
published crate order, release workflow, asset set, checksum verification, and
install smoke evidence.

The next post-v0.3 work should harden real-world usage rather than add broad new
surface area: source adapter receipts, Jira/Linear identity help, config
reference material, packet readability passes against real packets, and
continued coverage/mutation baseline review. Documented mutation baselines now
include `shiplog-coverage`, `shiplog-ids`, `shiplog-schema`, `shiplog-redact`,
`shiplog-bundle`, and a `shiplog-ports` no-target scan, recorded in
[`docs/ci/mutation.md`](ci/mutation.md).
