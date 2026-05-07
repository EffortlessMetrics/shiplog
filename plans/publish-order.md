# crates.io Publish Order

This document intentionally no longer lists every package directory as a public
publish target. The current boundary doctrine is in [`API_SURFACE.md`](../API_SURFACE.md):
public crates are stable contracts, trust surfaces, real adapters, or heavy
optional boundaries. Internal SRP seams should be modules inside an owner crate.

## Non-publishable crates

| Crate | Reason |
|---|---|
| `shiplog-testkit` | Test-only shared fixtures (`publish = false`) |
| `shiplog-fuzz` (in `fuzz/`) | Fuzz harnesses, not a workspace member (`publish = false`) |

There is no deferred production-package category. Production code is either a
publishable public crate or an owner module. `publish = false` is reserved for
dev-only packages.

## Target public tiers

These tiers are the intended public surface after weak implementation seams were
folded into owner modules.

### Tier 1 - Stable contracts

| Crate | Role |
|---|---|
| `shiplog-ids` | Deterministic identifier contract |
| `shiplog-schema` | Canonical event, coverage, workstream, and bundle schema |
| `shiplog-ports` | Extension traits for ingest, render, redact, and cluster |

### Tier 2 - Trust and product surfaces

| Crate | Role |
|---|---|
| `shiplog-coverage` | Completeness and coverage honesty |
| `shiplog-redact` | Privacy/security redaction behavior |
| `shiplog-bundle` | Shareable bundle manifests and checksums |
| `shiplog-workstreams` | Curation/workstream domain |
| `shiplog-cache` | Supported API-cache facade |
| `shiplog-render-md` | Primary user artifact renderer |
| `shiplog-render-json` | Supported machine-readable renderer |
| `shiplog-engine` | Pipeline orchestration API |

### Tier 3 - Real adapters

| Crate | Role |
|---|---|
| `shiplog-ingest-github` | GitHub ingest |
| `shiplog-ingest-git` | Local git ingest |
| `shiplog-ingest-json` | JSONL import |
| `shiplog-ingest-manual` | Manual YAML import |
| `shiplog-ingest-gitlab` | GitLab ingest library adapter |
| `shiplog-ingest-jira` | Jira ingest library adapter |
| `shiplog-ingest-linear` | Linear ingest library adapter |

### Tier 4 - Optional feature boundaries

| Crate | Role |
|---|---|
| `shiplog-cluster-llm` | Optional external LLM/privacy boundary |
| `shiplog-team` | Optional team aggregation surface |
| `shiplog-merge` | Public only if multi-source merge is a stable external API |

### Tier 5 - CLI product

| Crate | Role |
|---|---|
| `shiplog` | CLI product and composition root |

## Release proof

Before a release, regenerate a dependency-topological dry-run for the final
publishable set and prove no public crate depends on an unpublished internal
production package. For v0.3.0, use
[`plans/release-matrix-v0.3.0.md`](release-matrix-v0.3.0.md) as the current
publish decision record. The v0.2.1 matrix remains as historical context.
Current package-boundary enforcement lives in
`scripts/package-boundary-audit.sh`; current version-alignment enforcement lives
in `scripts/package-version-audit.sh`.
