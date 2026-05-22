# SHIPLOG-ADR-0002: Machine Source Keys Vs Display Labels

Status: accepted
Date: 2026-05-13
Related proposal:
[`SHIPLOG-PROP-0001-user-polish-release`](../proposals/SHIPLOG-PROP-0001-user-polish-release.md)
Related specs:
[`SHIPLOG-SPEC-0002-intake-report-v1`](../specs/SHIPLOG-SPEC-0002-intake-report-v1.md),
[`SHIPLOG-SPEC-0003-source-identity`](../specs/SHIPLOG-SPEC-0003-source-identity.md)
Related issues:
[#223](https://github.com/EffortlessMetrics/shiplog/issues/223),
[#229](https://github.com/EffortlessMetrics/shiplog/issues/229)

## Context

`intake.report.json` previously exposed the same conceptual source under
different naming conventions across report sections:

```text
source_freshness[].source = github / git / json_import
source_decisions[].source = GitHub / Local git / JSON
included/skipped sources  = display labels
```

That made the report harder for tools and agents to join. Tests had to
recreate private source-name normalization rules to compare sections, and the
deferred skipped-source freshness work would have baked in the same drift if it
had shipped before source identity was settled.

## Decision

Machine-readable JSON uses canonical source keys. Human-readable Markdown uses
display labels.

The preferred JSON shape for source-facing entries is:

```json
{
  "source_key": "github",
  "source_label": "GitHub"
}
```

`source_key` is the stable join key. `source_label` is display-only.
Markdown may render friendly labels directly, but Markdown labels must not be
the machine contract.

## Consequences

- Report JSON consumers join source arrays on `source_key`.
- Schema validation should enforce the canonical source-key vocabulary.
- Markdown remains free to show friendly labels such as `GitHub`, `Local git`,
  and `JSON`.
- Legacy `source` fields may remain temporarily only when the schema and docs
  specify whether they contain a key, label, or compatibility alias.
- Skipped-source freshness uses the canonical source identity fields and joins
  to `source_decisions` on `source_key`.
- Tests should not use report-local source normalization helpers for current
  writer output; they should assert `source_key` directly.

## Alternatives Considered

### Display Labels Everywhere

Rejected. This is convenient for Markdown but makes JSON a presentation surface
instead of a receipt. Labels can change for readability and should not be join
keys.

### Current Mixed Naming Plus Test Normalization

Rejected. Duplicating private normalization rules in tests preserves the bug and
makes future agent consumers repeat shiplog internals.

### Schema-Only Cross-Section Consistency

Rejected as the primary strategy. Schema checks can validate vocabulary and
shape, but they should not compensate for mixing display labels and machine
keys in one report contract.

## Affected Specs, Plans, Tests, And Schemas

- [`SHIPLOG-SPEC-0003-source-identity`](../specs/SHIPLOG-SPEC-0003-source-identity.md)
  owns the canonical source-key vocabulary and source-label contract.
- [`SHIPLOG-SPEC-0002-intake-report-v1`](../specs/SHIPLOG-SPEC-0002-intake-report-v1.md)
  owns the report JSON and Markdown boundary.
- [`contracts/schemas/intake-report.v1.schema.json`](../../contracts/schemas/intake-report.v1.schema.json)
  must enforce the source identity contract when the behavior changes.
- [`apps/shiplog/src/main.rs`](../../apps/shiplog/src/main.rs) currently
  contains `normalized_source_key` and `display_source_label`.
- [`apps/shiplog/tests/intake_cold_start.rs`](../../apps/shiplog/tests/intake_cold_start.rs)
  and
  [`apps/shiplog/tests/cli_integration.rs`](../../apps/shiplog/tests/cli_integration.rs)
  should prove direct joins on `source_key` after implementation.
