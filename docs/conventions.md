# Repo Conventions

This document records cross-cutting conventions that are visible in the shiplog
codebase but were not centralized anywhere. A new contributor or agent should be
able to find these patterns here directly instead of inferring them from prior
PRs and scattered audit comments.

Each pattern below is already established practice on the repo. This doc is
descriptive, not aspirational: it names what is already in place and explains
why, so the patterns survive contributor churn.

## `cpf-NNNN` design-concern identifiers

Each protected-field class in shiplog carries a stable identifier of the form
`cpf-NNNN`, where `NNNN` is a four-digit sequence. The current range is
`cpf-0001` through `cpf-0006`, defined in
[`policy/clippy-protected-fields.toml`](../policy/clippy-protected-fields.toml)
with one `[[class]]` entry per identifier:

```toml
[[class]]
id = "cpf-0001"
slug = "redaction-internals"
title = "Redaction internals"
# ...
```

The identifier is the anchor. Commits cite it (e.g. "policy(clippy): audit
source-opaque-ids — cpf-0004"), audit-history comments in the TOML cite it,
the closing doc section for each class cites it, and the per-entry `id` field
in the policy TOML carries it. The identifier outlives any individual PR — it
names the *concern*, not the implementation. The same `cpf-0005`
(cache-internals) is referenced across PR #192, #194, and #196 and the closing
protected-fields summary in
[`docs/CLIPPY_PROTECTED_FIELDS.md`](CLIPPY_PROTECTED_FIELDS.md); it remained
stable through the inner-struct refactor that fundamentally reshaped the
protection mechanism for that class.

The naming is project-internal. It is analogous to CVE numbers for
vulnerabilities or RFC numbers for protocol decisions, but it names design
concerns inside one codebase rather than anything externally visible. The
prefix `cpf` stands for the policy file (`clippy-protected-fields`); the
four-digit sequence is allocated by the first PR that introduces a new class
to the ledger.

This convention is currently scoped to the protected-fields ledger and should
not be extended to other domains without a deliberate decision. The cited
files —
[`policy/clippy-protected-fields.toml`](../policy/clippy-protected-fields.toml)
and [`docs/CLIPPY_PROTECTED_FIELDS.md`](CLIPPY_PROTECTED_FIELDS.md) — are the
canonical living examples. If a future ledger wants to mint stable concern
IDs of its own, the `<prefix>-NNNN` shape is the model.

## `propertyNames` regex as schema-level vocabulary gate

The intake-report JSON schema at
[`contracts/schemas/intake-report.v1.schema.json`](../contracts/schemas/intake-report.v1.schema.json)
applies a `propertyNames` constraint that rejects any field whose *name*
contains a secret-shaped substring, case-insensitive:

```json
"propertyNames": {
  "$ref": "#/$defs/non_secret_field_name"
},
"$defs": {
  "non_secret_field_name": {
    "type": "string",
    "not": {
      "pattern": "[Tt][Oo][Kk][Ee][Nn]|[Ss][Ee][Cc][Rr][Ee][Tt]|[Pp][Aa][Ss][Ss][Ww][Oo][Rr][Dd]|[Cc][Rr][Ee][Dd][Ee][Nn][Tt][Ii][Aa][Ll]|[Aa][Pp][Ii]_?[Kk][Ee][Yy]|[Kk][Ee][Yy]_?[Vv][Aa][Ll][Uu][Ee]"
    }
  }
}
```

The check fires at the schema layer, before any value-level validation. The
same `non_secret_field_name` is referenced from `$defs/object_base`, which
every nested object in the schema layers in via `allOf`, so the gate applies
recursively to the whole document — not just the top level.

This defense is incident-driven, not theoretical. Secret leaks have been a
real concern in the project and each layer of defense (content scanning,
`.gitignore` patterns, this schema gate) carries the receipt of a specific
past failure mode. The gate addresses one failure mode in particular: a
future contributor adding a field named `github_token` or `api_key` to the
intake report and not noticing.

The defense is **vocabulary**, not contents. Content-level secret detection
(redaction, scanning) is a different layer with its own tools. This gate does
not look at any value; it rejects a *name* that looks like it should never
have been a field in the first place. The point is that the schema author
cannot accidentally introduce the next leaky field name.

When adding a new JSON schema in `contracts/schemas/`, consider whether the
same vocabulary gate applies. Most schemas that describe user-facing output
should adopt it — the cost is one `$defs` entry and one `propertyNames` line
on each object base. Schemas that describe purely internal structures with
known field sets need not.

## Forward-receipt convention (reserved enum variants with epistemic comments)

The `FreshnessStatus` enum in
[`crates/shiplog-schema/src/freshness.rs`](../crates/shiplog-schema/src/freshness.rs)
reserves a `Stale` variant that no adapter emits today. The module doc and
the variant doc state explicitly *why* it is not emitted:

```rust
//! The taxonomy is intentionally narrow in v1: `Fresh`, `Cached`, `Skipped`,
//! and `Unavailable` (see the `FreshnessStatus` enum below). A `Stale` state is reserved
//! but not emitted today because the SQLite cache filters expired rows out
//! of `ApiCache::get` (the `expires_at > now` predicate), so adapters cannot
//! honestly distinguish stale-hit from cache-miss without a new lookup
//! return type. Adding a `CacheLookup::{Fresh, Stale, Miss}` enum to
//! `shiplog-cache` is the follow-up that unlocks stale-fallback reporting.
```

This is a **forward receipt**: the schema commits to a future shape so
consumers cannot be surprised when it lands. The cost is one enum variant and
one schema entry; the benefit is that the future state is named and reserved
*today*, with a written explanation of the epistemic constraint that
currently prevents emitting it. Tracked under issue
[#224](https://github.com/EffortlessMetrics/shiplog/issues/224)
(`CacheLookup::{Fresh, Stale, Miss}`).

Distinct from mechanical forward-compat: `#[non_exhaustive]`, proto3 unknown
fields, and deprecated-but-reserved field numbers all address *not breaking*
when an unknown shape appears. The forward-receipt convention addresses
something different — *announcing a specific known future shape* and the
constraint that prevents emitting it today.

The comment that explains why-not-yet is load-bearing. Without it, a future
contributor reading `FreshnessStatus::Stale` will see an enum arm that no
match expression reaches and either delete it as dead code or fake an
emission. The comment is what makes the reservation honest: it gives the
next reader the reason the variant exists, the dependency that has to land
first, and the name of the issue tracking that dependency. The forward
receipt only works while the comment is intact.

When adding a new enum that has a clear future state behind a known
dependency, prefer the forward-receipt shape with an epistemic comment over
a TODO or a separate "future" enum. A TODO disappears under pressure; a
separate future enum signals "the real enum is wrong"; a reserved variant
with an epistemic comment signals "the real enum already knows about this
shape; the constraint is upstream of this file."

## Inline labelled-parameter test comments

For one-off boundary-case inputs in `#[test]` functions, prefer an inline
labelled comment over a separate constant or a comment block:

```rust
assert_eq!(discounted_total(/* discount_threshold (equality boundary) */ 5), expected);
```

The principle: the *value* and the *reason this value* share an eye fixation.
The reader sees `5` and the label `discount_threshold (equality boundary)`
in the same place at the same time. There is no jump to a constant
definition, no separable comment block above the assertion line, and no
extracted helper that the reader has to scroll to.

Use the pattern only when the literal value is meaningful in a way the literal
does not show on its own — boundary cases (equality, off-by-one, the maximum
representable integer), the exact value at which a rate-limiter trips,
the smallest value that triggers a specific code path. Skip it for inputs
that are obviously what they are — a normal index, a normal count, a
placeholder string.

The pattern is invisible to `grep`. A refactoring search for `discount_threshold`
does not surface this site, and a search for `5` is useless. That invisibility
is a feature, not a bug: the label adds no noise to refactoring searches,
because it is not a code symbol. The label is for the human or agent who
reads the test line.

This is the assertion-layer instance of the same principle the `cpf-NNNN`
audit-history blocks and the `FreshnessStatus::Stale` epistemic comment use
at other granularities: capture non-obvious context at the point of
decision, in the surface the reader already has open. The granularities
differ — a ledger audit block is paragraphs, the `Stale` comment is a
docblock, a labelled parameter is a few words — but the convention is the
same.

The pattern is recommended where the situation arises, not required
repo-wide. The cost of overusing it is noise. The cost of underusing it
is a reader who wonders why `5`.

## Conventions under discussion

Source-name canonicalization across the `intake.report.json` sections is
currently inconsistent. `source_freshness` entries use lowercase internal
keys (`"github"`, `"manual"`); `source_decisions` entries use display labels.
The intended canonical form is being worked out under issue
[#223](https://github.com/EffortlessMetrics/shiplog/issues/223). This doc
will document the canonical form once the issue is resolved.

## See also

- [`docs/CLIPPY_PROTECTED_FIELDS.md`](CLIPPY_PROTECTED_FIELDS.md) — the
  canonical reference for the `cpf-NNNN` identifier scheme and the six
  classes that use it today.
- [`contracts/schemas/intake-report.v1.schema.json`](../contracts/schemas/intake-report.v1.schema.json)
  — the canonical example of the `propertyNames` vocabulary gate
  (`$defs/non_secret_field_name`).
- [`crates/shiplog-schema/src/freshness.rs`](../crates/shiplog-schema/src/freshness.rs)
  — the canonical example of the forward-receipt convention
  (`FreshnessStatus::Stale` variant + module doc).
- [`docs/POLICY_ALLOWLISTS.md`](POLICY_ALLOWLISTS.md) — the broader policy
  ledger common header schema this fits alongside.
