# shiplog Current State

This document records the release-readiness baseline after the workspace
boundary cleanup.

## Baseline

- `main` is the release-safe baseline after v0.2.1 publication and PR queue
  cleanup.
- The root workspace now matches the intended package invariant: production
  workspace crates are publishable public surfaces, and `publish = false` is
  reserved for dev-only packages.

## Product Contract

shiplog's product loop is:

```text
collect -> curate -> render
```

The artifact contract is:

```text
packet + ledger + coverage + bundles
```

The public API doctrine remains module-first: product contracts, trust
surfaces, real adapter boundaries, and heavy optional dependency boundaries may
be crates; implementation seams start as modules inside owner crates.

## Public Surface

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

Adapters for the v0.2.1 release path:

- `shiplog-ingest-github`
- `shiplog-ingest-git`
- `shiplog-ingest-json`
- `shiplog-ingest-manual`

Additional published adapter surfaces:

- `shiplog-ingest-gitlab`
- `shiplog-ingest-jira`
- `shiplog-ingest-linear`

Optional feature surfaces:

- `shiplog-cluster-llm`
- `shiplog-team`

Dev-only tooling:

- `shiplog-testkit` is `publish = false`.
- `fuzz/` is a fuzz harness workspace, not a crates.io target.

## Package Boundary

There is no durable held-production-crate category. A workspace package is
either a publishable public surface or dev-only tooling. Implementation seams
that are not public promises live as owner modules.

`shiplog-template` has been folded into `shiplog-team` as an internal module
because packet templates are not a standalone public contract. The CLI source
list remains narrower than the library adapter surface; GitLab, Jira, Linear,
and team are library surfaces until separate CLI work promotes user-facing
commands.

## Release Posture

The v0.2.1 readiness branch should prove the current surface without expanding
the product. It may update documentation, release matrix decisions, package
proof scripts, release workflow validation, and changelog handoff notes.

It should not add packet redesigns or mutation thresholds unless package proof
exposes a real blocker.
