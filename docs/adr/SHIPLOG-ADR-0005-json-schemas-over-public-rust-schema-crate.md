# SHIPLOG-ADR-0005: JSON Schemas Over Public Rust Schema Crate

Status: accepted
Date: 2026-05-14
Related proposal:
[`SHIPLOG-PROP-0003-crate-surface-contraction`](../proposals/SHIPLOG-PROP-0003-crate-surface-contraction.md)
Related spec:
[`SHIPLOG-SPEC-0004-public-crate-support-tiers`](../specs/SHIPLOG-SPEC-0004-public-crate-support-tiers.md)
Supersedes pending decision in:
[`SHIPLOG-ADR-0004-srp-modules-over-public-microcrates`](./SHIPLOG-ADR-0004-srp-modules-over-public-microcrates.md)

## Context

The 0.7 crate-surface contraction lane left one explicit decision open:
whether `shiplog-schema` should remain a supported public Rust crate.

Shiplog already has a stronger public machine contract surface:

- JSON schemas under `contracts/schemas/`;
- `shiplog report validate` for report validation;
- compatibility tests around report and schema shape;
- release docs that describe the JSON report contract.

The Rust types in `shiplog-schema` are useful internal implementation
structure. They are used across ingestion, report, bundle, redaction,
workstream, and test support code. That internal usefulness does not by itself
justify a separate crates.io support promise.

No known external Rust consumer has been recorded as depending on
`shiplog-schema` as an independently supported API.

## Decision

Do not retain `shiplog-schema` as a 0.7 public-supported crate.

For 0.7, the public machine contract is the checked-in JSON schemas under
`contracts/schemas/`, not the Rust types in `shiplog-schema`.

`shiplog-schema` should be classified as `internal-module` and remain
`publish = false` while it exists as a workspace package. It may later be
collapsed into owner modules or retained as unpublished workspace support
during the contraction.

A future release may promote a Rust schema crate only through a new ADR that
names the external consumer, supported API, semver commitment, docs contract,
and release proof.

## Consequences

- The 0.7 publish allowlist remains `shiplog` only.
- `shiplog-schema` must not enter `publish.default_order` without a later ADR.
- JSON schema compatibility remains the public machine-contract proof surface.
- Internal code can continue using Rust schema types while crate-collapse work
  proceeds.
- Downstream Rust users should not infer support from the 0.6 publication.
  They can pin 0.6 or open an issue describing the needed public contract.

## Alternatives Considered

### Keep `shiplog-schema` Public

Rejected. There is no recorded external Rust consumer or independently
supported typed API contract. Keeping it public would preserve a semver promise
that the project has not intentionally designed.

### Keep `shiplog-schema` Transitional Through 0.7

Rejected. The lane now has enough information to decide. Leaving the crate
transitional would keep release tooling and future agents uncertain.

### Remove The Rust Types Immediately

Rejected. This ADR decides public support, not internal code movement. The
types can remain as internal workspace support until a small, verified collapse
PR moves them.

### Replace JSON Schemas With Rust Types

Rejected. JSON schemas are language-neutral, validate shipped artifacts, and
match the report contract consumed by agents and other tools.

## Affected Specs, Tests, And Release Surfaces

- [`contracts/schemas/intake-report.v1.schema.json`](../../contracts/schemas/intake-report.v1.schema.json)
  remains the report JSON contract.
- [`contracts/schemas/agent-pack.v1.schema.json`](../../contracts/schemas/agent-pack.v1.schema.json)
  remains the agent-pack JSON contract.
- [`SHIPLOG-SPEC-0004-public-crate-support-tiers`](../specs/SHIPLOG-SPEC-0004-public-crate-support-tiers.md)
  should classify `shiplog-schema` as internal for 0.7.
- [`policy/publish-allowlist.toml`](../../policy/publish-allowlist.toml)
  must keep `shiplog-schema` out of the publish order.
- Release-prep proof should validate JSON schemas and the publish allowlist;
  it should not publish `shiplog-schema` unless this ADR is superseded.
