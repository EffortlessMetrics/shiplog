# SHIPLOG-ADR-0006: Repair Actions Are Receipt-Derived

Status: accepted
Date: 2026-05-14
Related proposal:
[`SHIPLOG-PROP-0002-evidence-repair-loop`](../proposals/SHIPLOG-PROP-0002-evidence-repair-loop.md)
Related spec:
[`SHIPLOG-SPEC-0005-evidence-repair-loop`](../specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md)
Related report spec:
[`SHIPLOG-SPEC-0002-intake-report-v1`](../specs/SHIPLOG-SPEC-0002-intake-report-v1.md)

## Context

Shiplog's 0.6.0 first-run work made intake reports trustworthy enough to show
what happened: source decisions, freshness, skipped sources, repair sources,
evidence debt, journal suggestions, next commands, and artifacts. The 0.7.0
crate-surface cut made the package boundary honest enough to resume product
work.

The Evidence Repair Loop asks Shiplog to do the next product job: turn
`Needs evidence` gaps into specific, safe repair actions.

That creates an architecture risk. Repair commands could accidentally become a
second source-state classifier by re-querying GitHub, GitLab, Jira, Linear,
local git, cache state, or JSON/manual evidence outside the intake report. If
that happens, users and agents would get two competing truths:

- the intake report says why a source was included, skipped, stale, cached, or
  unavailable;
- the repair command separately decides what is missing and what to do next.

Shiplog already has ADRs that reject this drift:

- [`SHIPLOG-ADR-0001`](SHIPLOG-ADR-0001-ingest-output-is-receipt-boundary.md)
  puts adapter evidence and freshness at the receipt boundary;
- [`SHIPLOG-ADR-0002`](SHIPLOG-ADR-0002-machine-source-keys-vs-display-labels.md)
  separates source keys from labels;
- [`SHIPLOG-ADR-0003`](SHIPLOG-ADR-0003-stale-requires-cachelookup.md)
  requires proven cache lookup evidence before emitting stale.

Repair actions need the same boundary.

## Decision

Repair actions are derived from intake report receipts.

The repair planner may read the latest `intake.report.json` and map existing
receipts to safe actions. It may not perform live source discovery, inspect
provider state, infer cache freshness, or classify source health independently.

Allowed receipt inputs are the fields named by
[`SHIPLOG-SPEC-0005`](../specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md):

- `source_decisions`;
- `source_freshness`;
- `repair_sources`;
- `evidence_debt`;
- `top_fixups`;
- `journal_suggestions`;
- `next_commands`;
- `actions`;
- `artifacts`.

The intake and report layers own the facts. Repair code owns only the mapping
from those facts to safe user actions.

The durable machine surface for repair actions is report JSON, not Markdown.
Markdown may render repair items for humans, but no command, agent, UI, or TUI
should scrape Markdown to recover repair state.

## Consequences

- `shiplog repair plan --latest` reads the latest report and renders repair
  items from report JSON.
- `shiplog journal add --from-repair <repair_id>` resolves the repair ID from
  the latest report before taking any local journal action.
- Provider setup guidance can name a missing configuration key or environment
  variable, but it cannot print secret values or mutate the provider.
- Source freshness guidance must follow `source_freshness` and cache receipts;
  it cannot be guessed from wall-clock age, global cache files, or absence.
- Repair diff compares report repair items across runs; it does not re-check
  sources to decide whether an item cleared.
- If a report lacks repair items, the repair command should ask for a rerun
  with a version that emits repair items instead of silently inventing them.
- New repair kinds require schema docs, compatibility notes, and tests tied to
  their receipt inputs.

## Alternatives Considered

### Re-query Providers During Repair Planning

Rejected. This would make repair planning another ingest path with different
auth, cache, failure, redaction, and freshness semantics. It would also make
repair output non-reproducible from the report artifact.

### Infer Repair Actions From Markdown

Rejected. Markdown is for human scanning. JSON is the machine boundary. Parsing
Markdown would make wording and section order an accidental API.

### Let Each Source Adapter Emit User Commands Directly

Rejected as the default. Adapters own source receipts. User-facing repair
commands need cross-source ordering, safety policy, share posture, and report
compatibility. Those belong above adapters.

Adapters may expose structured receipts that make repair actions possible, but
they should not own the final user command prose.

### Generate A Better Narrative Instead

Rejected. Better prose alone does not give a user or agent a bounded action
queue, stable IDs, clear conditions, or diffable improvement proof.

### Mutate Providers To Repair Gaps

Rejected for this lane. Repair actions may guide setup and local journal work.
Creating or editing GitHub, GitLab, Jira, or Linear data requires a separate
spec and safety model.

## Affected Specs, Plans, Tests, And Schemas

- [`SHIPLOG-SPEC-0005-evidence-repair-loop`](../specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md)
  defines repair item, repair plan, journal repair, and repair diff behavior.
- [`SHIPLOG-SPEC-0002-intake-report-v1`](../specs/SHIPLOG-SPEC-0002-intake-report-v1.md)
  defines report JSON as the machine-readable receipt.
- [`contracts/schemas/intake-report.v1.schema.json`](../../contracts/schemas/intake-report.v1.schema.json)
  should carry the optional `repair_items` schema when implementation lands.
- [`docs/schemas/intake-report-v1.md`](../schemas/intake-report-v1.md)
  should document repair item compatibility and absence behavior.
- [`apps/shiplog/tests/cli_integration.rs`](../../apps/shiplog/tests/cli_integration.rs)
  should cover repair commands, invalid IDs, old reports, and safety cases.
- [`apps/shiplog/tests/intake_cold_start.rs`](../../apps/shiplog/tests/intake_cold_start.rs)
  should cover cold-run repair item production.
- [`apps/shiplog/tests/front_door_first_pack_smoke.rs`](../../apps/shiplog/tests/front_door_first_pack_smoke.rs)
  should prove that a first packet can improve through the repair loop.
- The future Evidence Repair implementation plan should sequence schema,
  report, CLI, journal, diff, and guide work without violating this boundary.
