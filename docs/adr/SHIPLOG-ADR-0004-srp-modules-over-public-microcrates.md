# SHIPLOG-ADR-0004: SRP Modules Over Public Microcrates

Status: accepted
Date: 2026-05-13
Related proposal:
[`SHIPLOG-PROP-0003-crate-surface-contraction`](../proposals/SHIPLOG-PROP-0003-crate-surface-contraction.md)
Related specs:
[`SHIPLOG-SPEC-0004-public-crate-support-tiers`](../specs/SHIPLOG-SPEC-0004-public-crate-support-tiers.md)

## Context

Shiplog 0.6.0 made the first-run product trustworthy, but it also published a
broad workspace graph. The 0.6 publish script includes adapters, renderers,
cache, bundle, workstream, merge, team, LLM, schema, and CLI crates.

That graph reflects useful internal single-responsibility boundaries. It does
not mean every boundary should be a crates.io support promise.

Public crates create external contracts:

- users can import them directly;
- docs and semver expectations attach to them;
- release prep must publish them in dependency order;
- future refactors must preserve their package identities or provide migration
  paths.

For shiplog, most current non-CLI crates are product internals. The package
architecture should make the product easy to install and support, while the
repository architecture remains free to use clear modules for implementation
structure.

## Decision

Use crates for true external contracts. Use Rust modules or unpublished
workspace support for internal single-responsibility structure.

The default public surface for 0.7.0 is:

```text
shiplog
```

`shiplog-schema` remains a pending explicit decision. It may stay public only
if a later ADR records a typed Rust contract reason or known external Rust
consumer. Otherwise, JSON schemas under `contracts/schemas/` remain the public
machine contract and Rust schema types are internal implementation support.

Internal adapters, renderers, cache, bundle, coverage, workstream, engine,
merge, team, and LLM support should not remain public crates merely because
they are useful seams. They should become modules inside `shiplog` or
unpublished workspace support as the crate-surface contraction proceeds.

Workspace membership is not publish eligibility. Publish eligibility comes from
the support tier defined in
[`SHIPLOG-SPEC-0004`](../specs/SHIPLOG-SPEC-0004-public-crate-support-tiers.md).

## Consequences

- Release tooling must publish only an explicit allowlist of supported public
  crates.
- Future public crates require an ADR before they are added to the publish
  allowlist.
- Refactor PRs may preserve SRP by creating folders/modules, but should not
  create public crates for internal structure.
- Collapsing a crate must preserve current user-visible CLI/report behavior
  unless a separate spec-backed behavior PR says otherwise.
- 0.6.x implementation crates are treated as historical/transitional surfaces,
  not yanked as routine cleanup.
- Adapters are internal until shiplog intentionally defines a public plugin or
  adapter API.
- The Evidence Repair Loop should wait until report, CLI, journal, and source
  module boundaries are stable enough for repair IDs and actions to avoid
  crate-graph churn.

## Alternatives Considered

### Publish Every Workspace Crate

Rejected. This treats crates.io as internal modularity and turns implementation
seams into external semver promises.

### Keep The 0.6 Public Graph Indefinitely

Rejected. It preserves accidental support surface, longer publish order,
release fragility, and contributor confusion about ownership.

### Mark Implementation Crates Unstable But Keep Publishing Them

Rejected. Continued crates.io publication still communicates an importable
package surface. Unstable wording is weaker than not publishing implementation
crates.

### Yank 0.6 Implementation Crates

Rejected as routine cleanup. The issue is support-surface sprawl, not a
security or severe-correctness defect. Historical crates can remain available
while 0.7.0 stops relying on them.

### Flatten Everything Into One Large File Or Module

Rejected. The decision is not anti-modularity. The intended structure is
foldered SRP modules inside the product package, not a loss of internal
ownership boundaries.

### Keep Adapters Public Before A Plugin API Exists

Rejected. Adapter crates can be useful internal seams, but shiplog has not
committed to a public adapter API. A plugin or adapter API needs its own spec
and ADR.

## Affected Specs, Plans, Tests, And Release Surfaces

- [`SHIPLOG-PROP-0003-crate-surface-contraction`](../proposals/SHIPLOG-PROP-0003-crate-surface-contraction.md)
  explains why 0.7.0 is the crate-surface contraction release.
- [`SHIPLOG-SPEC-0004-public-crate-support-tiers`](../specs/SHIPLOG-SPEC-0004-public-crate-support-tiers.md)
  defines the support tiers and publish eligibility rules.
- [`Cargo.toml`](../../Cargo.toml) currently lists the broad workspace graph
  that implementation PRs will contract.
- [`scripts/publish-v0.6.0.sh`](../../scripts/publish-v0.6.0.sh) records the
  broad 0.6 publish order that the 0.7 release tooling must not repeat by
  default.
- Future crate-collapse PRs must run targeted first-run behavior proof, such as
  intake cold start, front-door smoke, CLI intake/report, policy gates, and
  diff checks named by the crate-surface proposal.
- Future release-prep PRs must prove the publish allowlist and publish dry-run
  for supported public crates only.
