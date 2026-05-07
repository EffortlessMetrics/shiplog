# shiplog Current State

This document records the v0.3.0 release-readiness baseline after the product
completion pass.

## Baseline

- `main` is a release-ready product baseline for v0.3.0 once the release
  validation scripts and hosted checks pass.
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

## Release Posture

The v0.3.0 readiness branch should not add new product features. It should align
versions, changelog entries, release matrix decisions, package proof scripts,
release workflow smoke tests, and the release handoff for the already-merged
product surface.
