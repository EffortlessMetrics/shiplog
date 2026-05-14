# SHIPLOG-PROP-0003: Crate-Surface Contraction

Status: proposed
Owner: product/release
Created: 2026-05-13
Target release: 0.7.0

## Summary

Shiplog 0.7.0 should be the crate-surface contraction release. The 0.6.0
user-polish release made first-run behavior trustworthy, but it also published
22 implementation crates. That broad crates.io surface conflicts with the
intended architecture: single-responsibility structure should usually be repo
modules, not public microcrates.

The 0.7.0 goal is to reduce shiplog's public crates.io surface to intentional
contracts while preserving the first-run behavior shipped in 0.6.0.

Target public surface:

```text
shiplog
```

ADR-0005 decides not to keep `shiplog-schema` as a 0.7 public crate. JSON
schemas under `contracts/schemas/` are the public machine contract unless a
future ADR promotes a typed Rust API.

Everything else should become an internal module, dev-only support, or
historical 0.6.x surface that is not supported as a forward public import
contract.

## Problem

The current workspace split is useful internally, but 0.6.0 made too much of
that internal structure public. The 0.6.0 publish script publishes 22 crates,
including adapters, renderers, cache, bundle, workstreams, LLM/team/merge
support, and the CLI.

That creates avoidable product and maintenance costs:

- release prep must manage a long dependency-ordered publish sequence;
- crates.io becomes a stand-in for internal modularity;
- users can reasonably infer semver promises from implementation crates;
- maintainers and agents have to ask which crate owns behavior that should be
  inside the product package;
- future refactors become constrained by public crate boundaries that were not
  intended as user-facing contracts.

The 0.6.0 publish-order gap is a concrete example. Publishing paused because
`shiplog-team` was listed before `shiplog-merge`. That was recoverable, and the
release is shipped, but the better fix is to avoid a broad publish graph where
possible.

## Why Now

The next product lane, Evidence Repair Loop, will touch the exact seams that
crate contraction affects:

- report builder;
- repair sources;
- source freshness;
- journal add;
- CLI command dispatch;
- open/latest behavior;
- rendered output;
- schemas.

Building the repair loop first would create churn when those modules move
afterward. The cleaner order is:

```text
0.7.0 crate-surface contraction
Evidence Repair Loop
0.8.0 or 0.7.x repair release
```

## Target Users

Primary users:

- users who install shiplog through `cargo install shiplog` and should not need
  to understand the internal crate graph;
- maintainers who need faster, lower-risk release preparation;
- future Codex, Droid, Claude, or human contributors who need clear ownership
  boundaries without accidental public semver commitments.

Secondary users:

- potential Rust library consumers who need to know which crates are actually
  supported as import surfaces;
- downstream packagers who need an explicit public package contract.

## Product End State

0.7.0 preserves the 0.6.0 first-run product path:

```bash
shiplog intake --last-6-months --explain
shiplog open intake-report --latest
shiplog open packet --latest
```

The user-visible behavior should remain unchanged unless a separate spec-backed
PR explicitly changes it.

The package surface changes:

- `shiplog` remains public and installable;
- `shiplog-schema` is not a 0.7 public crate; JSON schemas are the public
  machine contract unless a later ADR promotes a typed Rust API;
- implementation crates stop being published as forward-supported crates;
- `shiplog-testkit` and `xtask` remain dev-only and unpublished;
- 0.6.x implementation crates are historical/transitional and are not yanked
  without a security or severe-correctness reason.

## Public Crate Test

A crate stays public only if it passes this test:

1. External users import it directly.
2. It has an independently meaningful API.
3. It can be versioned and supported separately.
4. Its docs explain its contract.
5. Removing it would break a real user, not just shiplog's internal layout.

No crate is published merely because it is a useful internal boundary.

## Proposed Support Tiers

The follow-up spec should define these tiers:

```text
public-supported
public-transitional
internal-module
dev-only
historical-0.6
```

Every current workspace crate needs an explicit planned disposition before
code movement starts. Future public crates should require an ADR.

## Initial Disposition Hypothesis

This table is a starting point for the crate audit, not the final support-tier
spec.

| Current crate | 0.7 direction |
| --- | --- |
| `shiplog` | keep public |
| `shiplog-schema` | internal module; JSON schemas are public |
| `shiplog-ids` | inline unless externally justified |
| `shiplog-ports` | internal; no plugin API yet |
| `shiplog-engine` | internal module |
| `shiplog-coverage` | internal module |
| `shiplog-workstreams` | internal module |
| `shiplog-redact` | internal module |
| `shiplog-bundle` | internal module |
| `shiplog-cache` | internal module unless cache API is public |
| `shiplog-render-md` | internal module |
| `shiplog-render-json` | internal module |
| `shiplog-ingest-github` | internal adapter module |
| `shiplog-ingest-git` | internal adapter module |
| `shiplog-ingest-json` | internal adapter module |
| `shiplog-ingest-manual` | internal adapter module |
| `shiplog-ingest-gitlab` | internal adapter module |
| `shiplog-ingest-jira` | internal adapter module |
| `shiplog-ingest-linear` | internal adapter module |
| `shiplog-cluster-llm` | internal optional-feature module |
| `shiplog-team` | internal optional-feature module |
| `shiplog-merge` | internal module |
| `shiplog-testkit` | dev-only, `publish = false` |
| `xtask` | dev-only, `publish = false` |

Adapters should not stay public until shiplog intentionally supports a plugin
or adapter API.

## Success Criteria

This lane succeeds when the following are true:

- the supported public crate surface is explicitly documented;
- every current workspace crate has a support-tier disposition;
- retained public crates have a written support reason;
- internal implementation surfaces are modules or unpublished workspace
  support, not crates.io packages;
- release tooling publishes only the explicit allowlist;
- `cargo install shiplog` still works;
- the 0.6.0 first-run behavior is unchanged and tested;
- report JSON and JSON schemas remain the machine contract unless an ADR keeps
  a Rust schema crate public;
- 0.6.x implementation crates are documented as historical/transitional rather
  than yanked as routine cleanup;
- Evidence Repair Loop work remains paused until the report/CLI/module
  boundaries are stable.

## Non-Goals

This proposal does not:

- yank 0.6.0 implementation crates as routine cleanup;
- change first-run report behavior, source freshness, share posture, or CLI
  output;
- implement Evidence Repair Loop behavior;
- create a plugin or adapter API;
- promise that `shiplog-schema` remains public;
- collapse the whole workspace in one large PR;
- use crates.io as an internal module boundary.

## Proposed PR Ladder

1. This proposal:
   `docs/proposals/SHIPLOG-PROP-0003-crate-surface-contraction.md`.
2. Public crate support-tier spec:
   `docs/specs/SHIPLOG-SPEC-0004-public-crate-support-tiers.md`.
3. ADR:
   `docs/adr/SHIPLOG-ADR-0004-srp-modules-over-public-microcrates.md`.
4. Crate audit table:
   `docs/release/0.7-crate-surface.md` or an equivalent durable surface.
5. Rebase and land safe SRP refactor PRs:
   [#239](https://github.com/EffortlessMetrics/shiplog/pull/239),
   [#241](https://github.com/EffortlessMetrics/shiplog/pull/241),
   [#242](https://github.com/EffortlessMetrics/shiplog/pull/242), then
   [#240](https://github.com/EffortlessMetrics/shiplog/pull/240) only after
   full proof.
6. Inline render crates into `shiplog` modules.
7. Inline ingest adapter crates into `shiplog` source modules.
8. Inline product/trust support crates in small slices.
9. Decide whether `shiplog-schema` remains public. Landed as
   [`SHIPLOG-ADR-0005`](../adr/SHIPLOG-ADR-0005-json-schemas-over-public-rust-schema-crate.md).
10. Enforce the 0.7 publish allowlist in release tooling.
11. Document 0.6 implementation crates as historical.
12. Prepare the 0.7.0 crate-surface release.

Plans own PR sequencing and rollback. This proposal records why the lane
exists and what release outcome should be achieved.

## Validation Strategy

Documentation and planning PRs should run:

```bash
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Refactor PRs must prove output behavior is unchanged with targeted tests, at
minimum:

```bash
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test front_door_first_pack_smoke
cargo test -p shiplog --test cli_integration -- intake
cargo test -p shiplog --test cli_integration -- report
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Release-prep proof should additionally show:

- the publish allowlist includes only supported public crates;
- `cargo publish --dry-run` succeeds for allowed crates;
- no 0.6 implementation crate is required by the 0.7 `shiplog` package unless
  intentionally retained;
- release-install smoke passes against the release artifact.

## Yank Policy

Do not yank 0.6.0 implementation crates as a normal cleanup move.

Yanking is reserved for security or severe correctness problems. The issue
here is support-surface sprawl, not an unsafe or broken published artifact.
The safer posture is:

```text
0.6.x crates remain historical.
0.7.0 stops relying on unsupported implementation crates.
Docs define what is supported going forward.
```

## Alternatives Considered

### Leave the 0.6 public crate graph alone

Rejected. It preserves accidental semver promises and keeps release tooling
more complex than the product requires.

### Yank 0.6.0 implementation crates

Rejected. Yanking would create user pain to clean up an architecture decision.
Historical crates can remain available while 0.7.0 clarifies support.

### Keep all crates public but mark them unstable

Rejected. crates.io publication still communicates a public import surface.
Unstable labels are weaker than not publishing implementation crates.

### Build Evidence Repair Loop first

Rejected for sequencing. Repair plan, journal repair, and repair diff will sit
on report, source, and CLI modules that this lane needs to stabilize first.

### Keep adapters public before a plugin API exists

Rejected. Adapter crates are useful SRP boundaries, but shiplog has not
committed to a public adapter API.

## Relationship To Evidence Repair Loop

[`SHIPLOG-PROP-0002-evidence-repair-loop`](SHIPLOG-PROP-0002-evidence-repair-loop.md)
remains the next product direction after crate-surface contraction. Its target
release should move behind 0.7.0 unless the project deliberately splits repair
work into a later 0.7.x release.

The repair loop should start after the main report, CLI, journal, and source
modules are stable enough that repair IDs, source freshness, journal add, and
repair diff do not move across crate boundaries mid-lane.

## Exit Criteria

The lane can close when:

- proposal, spec, ADR, crate audit, implementation plan, and active goal
  manifest have landed;
- the supported public crate surface is enforced by release tooling;
- non-public implementation crates have been inlined or made unpublished in
  small, verified PRs;
- `shiplog-schema` has an ADR-backed internal/public decision;
- 0.6.x implementation crates are documented as historical/transitional;
- the 0.7.0 release is prepared with install, intake, open, report, schema, and
  publish-surface proof.

North star: shiplog should be modular in the repo, not accidentally public on
crates.io.
