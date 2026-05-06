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

## Target public tiers

These tiers are the intended public surface after transitional implementation
carriers are either folded into owner modules or deliberately promoted.

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
| `shiplog-ingest-gitlab` | Conditional public adapter |
| `shiplog-ingest-jira` | Conditional public adapter |
| `shiplog-ingest-linear` | Conditional public adapter |

### Tier 4 - Optional feature boundaries

| Crate | Role |
|---|---|
| `shiplog-cluster-llm` | Optional external LLM/privacy boundary |
| `shiplog-team` | Optional team aggregation surface |
| `shiplog-merge` | Public only if multi-source merge is a stable external API |
| `shiplog-template` | Public only if packet templates become a stable user contract |

### Tier 5 - CLI product

| Crate | Role |
|---|---|
| `shiplog` | CLI product and composition root |

## Transitional carriers

Some implementation carriers still exist as packages to preserve behavior during
this cleanup. They should not be treated as crates.io product surface unless
`API_SURFACE.md` deliberately promotes them:

`shiplog-alias`, `shiplog-cache-expiry`, `shiplog-cache-key`,
`shiplog-cache-sqlite`, `shiplog-cache-stats`, `shiplog-cluster-llm-parse`,
`shiplog-cluster-llm-prompt`, `shiplog-date-windows`, `shiplog-manual-events`,
`shiplog-output-layout`, `shiplog-receipt`, `shiplog-redaction-policy`,
`shiplog-redaction-profile`, `shiplog-redaction-projector`,
`shiplog-redaction-repo`, `shiplog-team-aggregate`, `shiplog-team-core`,
`shiplog-team-render`, `shiplog-workstream-cluster`,
`shiplog-workstream-layout`, and `shiplog-workstream-receipt-policy`.

Before a release, regenerate a dependency-topological dry-run for the final
publishable set and prove no public crate depends on an unpublished internal
carrier.
