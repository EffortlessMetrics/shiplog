# SHIPLOG-SPEC-0005: Evidence Repair Loop

Status: proposed
Owner: product/schema
Created: 2026-05-14
Related proposal:
[`SHIPLOG-PROP-0002-evidence-repair-loop`](../proposals/SHIPLOG-PROP-0002-evidence-repair-loop.md)
Related report spec:
[`SHIPLOG-SPEC-0002-intake-report-v1`](SHIPLOG-SPEC-0002-intake-report-v1.md)
Related source identity spec:
[`SHIPLOG-SPEC-0003-source-identity`](SHIPLOG-SPEC-0003-source-identity.md)

## Purpose

This spec defines the repair-loop contract for turning a trustworthy first-run
report into specific next actions. The repair loop should let a user move from
`Needs evidence` to a better packet through bounded, receipt-derived steps:

```bash
shiplog intake --last-6-months --explain
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
```

The repair loop is a product-quality lane. It is not a new source classifier,
provider mutator, or performance narrative generator.

## Scope

This spec owns:

- repair item semantics and compatibility;
- allowed report receipt inputs for repair planning;
- `shiplog repair plan --latest` behavior;
- `shiplog journal add --from-repair <repair_id>` behavior for manual evidence
  repairs;
- repair diff behavior across recent runs;
- safety rules for commands, source details, and share profiles;
- proof expectations for schema, CLI, and product tests.

Out of scope:

- live provider mutation, issue creation, ticket edits, or API-side repair;
- a second source-health classifier outside the intake report;
- scraping `intake.report.md` for machine control flow;
- broad report schema redesign unrelated to repair items;
- changing source identity vocabulary beyond
  [`SHIPLOG-SPEC-0003`](SHIPLOG-SPEC-0003-source-identity.md);
- changing existing first-run behavior except through the repair-loop surfaces
  named here.

## User Contract

After a run creates an intake report, a user can ask for a repair plan:

```bash
shiplog repair plan --latest
```

The command reads the latest `intake.report.json`. It must not query live
providers or infer source health independently. It prints a short, copyable
queue of repair items ordered by the impact defined in the report.

Each visible repair item must answer:

- what is missing or stale;
- which source owns the gap, when source-owned;
- why Shiplog believes the gap exists;
- what safe action the user can take;
- what should clear the item on a later run.

If no latest report exists, `repair plan --latest` must print the intake
command that creates one:

```bash
shiplog intake --last-6-months --explain
```

If the latest report is older and does not contain repair items, the command
must treat the report as compatible but incomplete. It should ask the user to
rerun intake with a Shiplog version that emits repair items instead of treating
the report as corrupt.

## Machine Contract

Future implementation may extend `intake.report.json` v1 with an optional
top-level `repair_items` array. Adding this optional field remains
backward-compatible for v1 readers.

Absence behavior:

- older reports without `repair_items` remain valid v1 reports;
- repair commands may render a rerun instruction from older reports;
- consumers must not assume `repair_items` exists unless the schema or report
  says it does.

When present, each repair item has this intended shape:

```json
{
  "repair_id": "repair_001_manual_empty",
  "repair_key": "manual:manual_evidence_missing",
  "source_key": "manual",
  "source_label": "Manual",
  "kind": "manual_evidence_missing",
  "reason": "No manual evidence events were found.",
  "action": {
    "kind": "journal_add",
    "command": "shiplog journal add --from-repair repair_001_manual_empty"
  },
  "clears_when": "manual source contributes at least one evidence event",
  "receipt_refs": [
    {
      "field": "repair_sources",
      "source_key": "manual"
    }
  ]
}
```

Required fields:

- `repair_id`: user-facing ID stable within the report;
- `repair_key`: machine comparison key stable across reruns for the same
  underlying repair gap;
- `kind`: bounded repair kind;
- `reason`: user-facing explanation derived from report receipts;
- `action`: safe action object;
- `clears_when`: user-facing condition for clearing the item;
- `receipt_refs`: report fields that justify the item.

Optional fields:

- `source_key`: canonical source key for source-owned repairs;
- `source_label`: display label paired with `source_key`;
- future display-only labels, if they do not become join keys.

`repair_id` is for current-report commands and display. `repair_key` is for
diffing across runs. A repair item must not use a raw provider object ID,
secret value, local private path, or unstable prose as its key.

## Receipt Inputs

Repair planning may read only existing intake report receipts:

- `source_decisions`;
- `source_freshness`;
- `repair_sources`;
- `evidence_debt`;
- `top_fixups`;
- `journal_suggestions`;
- `next_commands`;
- `actions`;
- `artifacts`.

Repair planning must not independently decide whether GitHub, GitLab, Jira,
Linear, local git, JSON, manual evidence, cache, redaction, or sharing is
healthy. Source and report layers produce receipts; the repair planner turns
receipts into actions.

## Repair Kinds

Initial repair kinds should be source- and receipt-derived. The exact enum may
expand by spec update, but the first implementation should cover:

- `manual_evidence_missing`;
- `source_skipped_configuration`;
- `source_freshness_stale`;
- `source_cached_only`;
- `evidence_debt_open`;
- `share_redaction_required`;
- `artifact_missing_or_unopened`.

Adding a kind is compatible when:

- old consumers can treat it as an opaque string;
- the item still carries `reason`, `action`, `clears_when`, and
  `receipt_refs`;
- the schema docs describe the absence and unknown-kind behavior.

## Repair Actions

`action.kind` describes the safe action shape. Initial action kinds:

- `journal_add`: Shiplog-local manual evidence repair;
- `configure_source`: user config or environment setup guidance;
- `rerun_intake`: rerun guidance after setup or journal repair;
- `open_artifact`: inspect an existing Shiplog artifact;
- `no_safe_action`: explain the gap when Shiplog cannot suggest a safe command.

When `action.command` is present, it must be safe to display and copy. It must
not contain secret values, token placeholders that look like real credentials,
private provider IDs, or destructive provider mutations.

Provider setup guidance may say which variable or config key is missing. It
must not print a token value or attempt to set it.

## Journal Repair Contract

`shiplog journal add --from-repair <repair_id>` resolves `<repair_id>` against
the latest report by default.

It may act only on repair items whose `action.kind` is `journal_add`.

Failure cases must be useful:

- no latest report: print the intake command that creates one;
- unknown repair ID: list valid repair IDs from the latest report;
- non-journal repair ID: say which action kind owns the item and show the safe
  command or guidance;
- stale report without `repair_items`: ask the user to rerun intake.

The command must not overwrite existing user journal content destructively. If
it writes a draft, appends an entry, or asks for interactive content, that
behavior belongs in the implementation plan and tests.

## Repair Diff Contract

`shiplog repair diff --latest` or the final chosen diff command compares the
latest two report runs.

The diff groups items by `repair_key`, not by display order. It must show:

- cleared repair items;
- new repair items;
- still-open repair items;
- changed repair items where reason, action, or clear condition changed.

If fewer than two compatible reports exist, the command should say what is
missing and show the next command that would create the needed run.

## Safety Contract

Repair items and repair commands must inherit the report safety posture:

- no secret-bearing field names in JSON objects unless a spec records an
  exemption;
- no secret values in commands or reasons;
- no raw provider opaque IDs across non-internal profiles;
- no provider mutations in this lane;
- no Markdown scraping for machine behavior;
- no report shape changes without schema docs and compatibility notes.

The report JSON remains the machine boundary. Markdown may render repair items
for humans, but it must not be the only source of a repair contract.

## Producers And Consumers

Producers:

- the intake report writer;
- future repair item builder logic that consumes report receipts;
- journal repair helpers for local manual evidence actions.

Consumers:

- `shiplog repair plan --latest`;
- `shiplog journal add --from-repair <repair_id>`;
- `shiplog repair diff --latest` or the final diff command;
- local UI, TUI, editor, and agent surfaces that read `intake.report.json`;
- schema validators and documentation checks.

Producers and consumers must agree on absence behavior before implementation:
old reports without `repair_items` are compatible, but repair commands may ask
the user to rerun intake to get repair-loop support.

## Acceptance Criteria

The repair-loop contract is implemented when:

- `intake.report.json` exposes optional `repair_items` that validate against
  the schema and docs;
- repair items have deterministic `repair_id` values within a report;
- repair items have stable `repair_key` values for diffing across reruns;
- repair items link to `source_key` and `source_label` when source-owned;
- repair items carry `kind`, `reason`, `action`, `clears_when`, and
  `receipt_refs`;
- `shiplog repair plan --latest` renders repair items from the latest report
  and handles missing or old reports;
- `shiplog journal add --from-repair <repair_id>` handles manual evidence
  repairs without provider mutation or destructive overwrite;
- skipped provider sources produce setup and rerun guidance without printing
  secret values;
- stale and cached guidance follows `source_freshness` and cache receipts;
- repair diff uses `repair_key` to show cleared, new, still-open, and changed
  repair items;
- end-to-end proof shows a cold first run, repair plan, manual repair, rerun,
  and improved packet.

## Compatibility Notes

This spec does not itself change the schema or CLI. It defines the contract
future implementation PRs must satisfy.

The expected schema change is backward-compatible:

- add optional `repair_items` to `intake.report.json` v1;
- document absence behavior for older reports;
- keep old v1 readers valid when they ignore unknown optional fields.

Any future required field, type change, enum meaning change, or removal from
existing report fields requires a compatibility note and may require a schema
version change under
[`SHIPLOG-SPEC-0002`](SHIPLOG-SPEC-0002-intake-report-v1.md).

## Proof Mapping

Expected proof surfaces:

- [`contracts/schemas/intake-report.v1.schema.json`](../../contracts/schemas/intake-report.v1.schema.json)
  for optional `repair_items` shape and secret-vocabulary inheritance.
- [`docs/schemas/intake-report-v1.md`](../schemas/intake-report-v1.md)
  for reader compatibility and absence behavior.
- [`apps/shiplog/tests/cli_integration.rs`](../../apps/shiplog/tests/cli_integration.rs)
  for repair CLI, missing report, old report, invalid ID, and diff behavior.
- [`apps/shiplog/tests/intake_cold_start.rs`](../../apps/shiplog/tests/intake_cold_start.rs)
  for cold first-run repair item production.
- [`apps/shiplog/tests/front_door_first_pack_smoke.rs`](../../apps/shiplog/tests/front_door_first_pack_smoke.rs)
  for the end-to-end first packet improvement proof.
- [`docs/proposals/SHIPLOG-PROP-0002-evidence-repair-loop.md`](../proposals/SHIPLOG-PROP-0002-evidence-repair-loop.md)
  for product intent and non-goals.
- Future ADR:
  `docs/adr/SHIPLOG-ADR-0006-repair-actions-are-receipt-derived.md`.
- Future implementation plan:
  `plans/evidence-repair/implementation-plan.md`.

Useful validation commands for docs-only PRs:

```bash
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
cargo xtask check-executable-files --mode blocking-allowlist
git diff --check
```
