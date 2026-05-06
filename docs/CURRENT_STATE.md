# shiplog Current State

This document records the release-readiness baseline after the workspace
boundary cleanup.

## Baseline

- `main` was reset to `origin/main` at `c3395daef7ea280158defa14ed1915f9bf3fb172`.
- The baseline worktree was clean and there were no open pull requests.
- The root workspace now matches the intended public surface: CLI, stable
  contracts, product/trust surfaces, source adapters, optional feature surfaces,
  and dev-only testkit.

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

Optional feature surfaces:

- `shiplog-cluster-llm`

Dev-only tooling:

- `shiplog-testkit` is `publish = false`.
- `fuzz/` is a fuzz harness workspace, not a crates.io target.

## Held Surface

GitLab, Jira, and Linear adapter crates remain in the workspace, but they are
held out of the v0.2.1 crates.io release set until their CLI story, auth model,
examples, and release-grade tests are documented. Their manifests use
`publish = false` until a later PR deliberately promotes them.

`shiplog-team` and `shiplog-template` remain workspace crates, but they are held
out of the v0.2.1 crates.io release set until team aggregation and packet
template syntax have stronger user-facing examples, versioning, and compatibility
tests. Their manifests use `publish = false` until promotion. The published CLI
does not expose a `team` feature for v0.2.1.

## Release Posture

The v0.2.1 readiness branch should prove the current surface without expanding
the product. It may update documentation, release matrix decisions, package
proof scripts, release workflow validation, and changelog handoff notes.

It should not add new product commands, packet redesigns, mutation thresholds,
or additional package-boundary migrations unless package proof exposes a real
blocker.
