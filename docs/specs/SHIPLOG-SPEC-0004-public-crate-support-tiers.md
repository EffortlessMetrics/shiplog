# SHIPLOG-SPEC-0004: Public Crate Support Tiers

Status: proposed
Owner: product/release
Created: 2026-05-13
Related proposal:
[`SHIPLOG-PROP-0003-crate-surface-contraction`](../proposals/SHIPLOG-PROP-0003-crate-surface-contraction.md)

## Purpose

This spec defines the support-tier contract for shiplog crates. The goal is to
make the 0.7.0 crate-surface contraction executable without treating every
workspace member as a public package.

Shiplog can keep single-responsibility boundaries in the repository. That does
not mean those boundaries are crates.io contracts.

## Scope

This spec owns:

- crate support-tier names and meanings;
- publish eligibility rules;
- the public crate test;
- the planned disposition of every current workspace crate;
- compatibility posture for crates published during the 0.6.0 release.

This spec does not move code, change package manifests, or update release
scripts. Those belong to implementation PRs and the crate-surface
implementation plan.

## Public Crate Test

A crate is public-supported only when all of these are true:

1. External users are expected to import it directly.
2. It has an independently meaningful API.
3. It can be versioned and supported separately from the CLI package.
4. Its docs explain the external contract.
5. Removing it would break a real user, not only shiplog's internal layout.

No crate is published merely because it is a useful internal boundary.

## Support Tiers

### public-supported

`public-supported` crates are intentional external contracts.

Contract:

- may be published by the 0.7 release tooling;
- must be listed in the publish allowlist;
- must have user-facing docs for the supported API;
- must have release proof for publish dry-run and install/import behavior;
- must follow semver as an external import surface.

Future crates cannot become `public-supported` without an ADR.

### public-transitional

`public-transitional` crates are temporary public surfaces that exist for
migration or a pending support decision.

Contract:

- may be published only when a release plan names the migration reason;
- must have a planned exit: keep as `public-supported` by ADR, or retire as an
  unsupported forward surface;
- must not be introduced as a convenience boundary;
- must be documented in release notes when users need to change imports.

ADR-0005 decides that `shiplog-schema` is internal for 0.7. JSON schemas under
`contracts/schemas/` are the public machine contract unless a later ADR promotes
a typed Rust API.

### internal-module

`internal-module` crates are implementation boundaries.

Contract:

- must not be published by 0.7 release tooling;
- must set `publish = false` while it remains a package manifest;
- should become modules inside the `shiplog` package or unpublished workspace
  support;
- may keep SRP structure through Rust modules, feature modules, or private
  package organization;
- must not be documented as a supported external import surface.

Adapters, renderers, cache, bundle, coverage, workstream, engine, merge, team,
and LLM support belong here unless a later ADR creates a public plugin or
library API.

### dev-only

`dev-only` crates support repository development, tests, or release checks.

Contract:

- must not be published;
- should use `publish = false` when they remain package manifests;
- must not be required by normal `cargo install shiplog` users;
- may be used by tests, fixtures, `xtask`, CI, or local validation.

### historical-0.6

`historical-0.6` describes crates that were published during the 0.6.0 release
but are not supported as forward public import surfaces after contraction.

Contract:

- are not yanked as routine cleanup;
- may remain available on crates.io for historical dependency resolution;
- must not be republished in 0.7+ unless another support tier explicitly allows
  it;
- should be documented as historical/transitional in release notes or release
  docs.

Yanking remains reserved for security or severe correctness issues.

## Publish Eligibility

The release publish allowlist must be tier-driven:

```text
public-supported   -> publish eligible
public-transitional -> publish eligible only by named release exception
internal-module    -> not publish eligible
dev-only           -> not publish eligible
historical-0.6     -> not publish eligible for forward releases
```

Workspace membership is not publish eligibility.

The publish tooling should fail closed when a workspace package is not assigned
a support tier or when a non-eligible crate appears in the publish allowlist.

## Current Workspace Disposition

This table records the planned 0.7 disposition for the current workspace. Code
movement and manifest edits belong to later PRs; this table defines the
contract those PRs must satisfy.

| Current crate | 0.7 support tier | Publish eligibility | Planned disposition |
| --- | --- | --- | --- |
| `shiplog` | `public-supported` | yes | Keep as the supported CLI/package surface. |
| `shiplog-schema` | `internal-module` | no | Keep Rust schema types internal; JSON schemas are the public machine contract. |
| `shiplog-ids` | `internal-module` | no | Inline or make unpublished support; 0.6 published versions become `historical-0.6`. |
| `shiplog-ports` | `internal-module` | no | Keep private until a plugin/API ADR exists; 0.6 published versions become `historical-0.6`. |
| `shiplog-engine` | `internal-module` | no | Inline into product internals; 0.6 published versions become `historical-0.6`. |
| `shiplog-coverage` | `internal-module` | no | Inline coverage support; 0.6 published versions become `historical-0.6`. |
| `shiplog-workstreams` | `internal-module` | no | Inline workstream support; 0.6 published versions become `historical-0.6`. |
| `shiplog-redact` | `internal-module` | no | Inline redaction support while preserving fail-closed behavior; 0.6 published versions become `historical-0.6`. |
| `shiplog-bundle` | `internal-module` | no | Inline bundle support; 0.6 published versions become `historical-0.6`. |
| `shiplog-cache` | `internal-module` | no | Inline cache/freshness support unless a future ADR creates a public cache API; 0.6 published versions become `historical-0.6`. |
| `shiplog-render-md` | `historical-0.6` / `internal-module` | no | Collapsed into `shiplog::render::md`; 0.6 published versions become `historical-0.6`. |
| `shiplog-render-json` | `historical-0.6` / `internal-module` | no | Collapsed into the engine artifact writer for this contraction slice; 0.6 published versions become `historical-0.6`. |
| `shiplog-ingest-github` | `historical-0.6` / `internal-module` | no | Collapsed into `shiplog::ingest::github`; 0.6 published versions become `historical-0.6`. |
| `shiplog-ingest-git` | `historical-0.6` / `internal-module` | no | Collapsed into `shiplog::ingest::git`; 0.6 published versions become `historical-0.6`. |
| `shiplog-ingest-json` | `historical-0.6` / `internal-module` | no | Collapsed into `shiplog::ingest::json`; 0.6 published versions become `historical-0.6`. |
| `shiplog-ingest-manual` | `historical-0.6` / `internal-module` | no | Collapsed into `shiplog::ingest::manual`; 0.6 published versions become `historical-0.6`. |
| `shiplog-ingest-gitlab` | `historical-0.6` / `internal-module` | no | Collapsed into `shiplog::ingest::gitlab`; 0.6 published versions become `historical-0.6`. |
| `shiplog-ingest-jira` | `historical-0.6` / `internal-module` | no | Collapsed into `shiplog::ingest::jira`; 0.6 published versions become `historical-0.6`. |
| `shiplog-ingest-linear` | `historical-0.6` / `internal-module` | no | Collapsed into `shiplog::ingest::linear`; 0.6 published versions become `historical-0.6`. |
| `shiplog-cluster-llm` | `internal-module` | no | Inline optional LLM support; 0.6 published versions become `historical-0.6`. |
| `shiplog-team` | `internal-module` | no | Inline team support; 0.6 published versions become `historical-0.6`. |
| `shiplog-merge` | `internal-module` | no | Inline merge support; 0.6 published versions become `historical-0.6`. |
| `shiplog-testkit` | `dev-only` | no | Keep test support unpublished. |
| `xtask` | `dev-only` | no | Keep repository automation unpublished. |

## Compatibility Notes

0.6.0 published implementation crates. That release is not rewritten by this
spec.

The 0.7 posture is:

- historical 0.6 crates remain available unless a security or severe
  correctness issue requires yanking;
- supported 0.7 docs name only the public-supported surface and any explicit
  public-transitional exception;
- `cargo install shiplog` remains the primary user path;
- JSON schemas under `contracts/schemas/` remain the public machine contract.

## Acceptance Criteria

The support-tier model is complete when:

- every current workspace crate has a tier and planned disposition;
- publish eligibility follows the tier table in this spec;
- future public crates require an ADR;
- release tooling can enforce an explicit publish allowlist;
- package manifests cannot remain publishable when policy marks the package
  non-public;
- historical 0.6 crates are documented without routine yanking;
- the `shiplog-schema` public-support decision is recorded before 0.7 release
  prep;
- implementation PRs prove 0.6.0 first-run behavior is unchanged when crates
  are collapsed.

## Proof Mapping

Current proof and control surfaces:

- [`SHIPLOG-PROP-0003-crate-surface-contraction`](../proposals/SHIPLOG-PROP-0003-crate-surface-contraction.md)
  explains why 0.7.0 is a crate-surface contraction release.
- [`Cargo.toml`](../../Cargo.toml) lists the current workspace members.
- [`scripts/publish-v0.6.0.sh`](../../scripts/publish-v0.6.0.sh) records the
  broad 0.6 publish sequence that this spec narrows for 0.7.
- [`RELEASE_HANDOFF_0.6.0.md`](../../RELEASE_HANDOFF_0.6.0.md) records the
  0.6 release handoff context.
- [`docs/release/0.6.0-readiness.md`](../release/0.6.0-readiness.md) records
  the 0.6 readiness proof surface.
- [`docs/release/0.6-crate-surface-historical.md`](../release/0.6-crate-surface-historical.md)
  records the 0.6 crates as historical/transitional forward surfaces.

Expected follow-up proof:

- crate audit table matching this spec;
- ADR for SRP modules over public microcrates;
- ADR-0005 deciding that `shiplog-schema` is internal and JSON schemas are
  public;
- release-tooling allowlist check via
  [`policy/publish-allowlist.toml`](../../policy/publish-allowlist.toml) and
  [`scripts/package-boundary-audit.sh`](../../scripts/package-boundary-audit.sh);
- Cargo manifest `publish = false` checks for policy-disabled workspace
  packages;
- publish dry-run proof for allowed public crates only;
- first-run intake/open/report tests after crate collapse.

Useful validation commands for docs-only PRs:

```bash
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

## Non-Goals

This spec does not:

- collapse any crate;
- edit package publish settings;
- update the release script;
- yank 0.6.0 crates;
- change user-visible first-run behavior;
- change report JSON or schema compatibility;
- implement Evidence Repair Loop behavior;
- create a plugin or adapter API.
