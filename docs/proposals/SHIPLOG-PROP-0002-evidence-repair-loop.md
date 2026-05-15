# SHIPLOG-PROP-0002: Evidence Repair Loop

Status: proposed
Owner: product/docs
Created: 2026-05-13
Target release: after 0.7.0 crate-surface contraction
Superseding note: `SHIPLOG-PROP-0003-crate-surface-contraction` moves the next
release lane to public crate-surface correction. Evidence Repair Loop remains
the next product lane after report, CLI, journal, and source modules are stable.

## Summary

Shiplog's next product lane should turn first-run gaps into specific,
safe, receipt-derived repair actions. The 0.6.0 user-polish release made
the first run trustworthy: a user can run intake, open the latest report or
packet, and understand source state, freshness, skipped sources, share posture,
and next commands. The next lane should make that trustworthy first run
actionable.

The product goal is not to generate a better performance narrative. It is to
help a user climb from a rough first packet toward a useful packet in small,
verifiable steps:

```bash
shiplog intake --last-6-months --explain
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog open packet --latest
```

The architecture goal is equally important: repair actions must be derived from
existing intake report receipts. They should not become a second source-state
classifier.

## Problem

0.6.0 answers "what happened?" well enough for the first-run path:

```bash
shiplog intake --last-6-months --explain
shiplog open intake-report --latest
shiplog open packet --latest
```

That is necessary but incomplete. A report with `Needs evidence` can still
leave the user doing translation work:

- what exactly is missing;
- why it is missing;
- which source owns the gap;
- what command or file repairs it;
- whether the repair is safe;
- whether rerunning intake improved the packet.

Shiplog already emits the primitives needed to answer those questions without
guessing:

- `source_key` and `source_label`;
- `source_decisions`;
- `source_freshness`;
- `repair_sources`;
- `needs_attention`;
- `evidence_debt`;
- `journal_suggestions`;
- `next_commands`;
- `actions`;
- `artifacts`.

The missing piece is a stable repair loop that turns those receipts into a
bounded queue of repair items and user-directed commands.

## Target Users

Primary users:

- a self-reviewer who has an incomplete first packet and needs the next
  evidence action instead of another diagnosis;
- a manager-prep user who must make a packet reviewable under time pressure;
- a deadline-pressure user who needs to improve the highest-impact gap first;
- an agent consumer that needs a machine-readable repair queue instead of
  scraping Markdown or terminal output.

Secondary users:

- maintainers and reviewers checking that repair guidance stays tied to report
  receipts;
- future local UI, TUI, or editor integrations that can render repair items
  from JSON.

## Product End State

The lane is done when a user can run:

```bash
shiplog intake --last-6-months --explain
shiplog repair plan --latest
```

and see a copyable repair plan such as:

```text
Needs evidence

1. repair_001_manual_empty
   Source: Manual
   Problem: No manual evidence events were found.
   Action:
     shiplog journal add --from-repair repair_001_manual_empty

2. repair_002_github_skipped
   Source: GitHub
   Problem: GitHub was skipped because GITHUB_TOKEN is not set.
   Action:
     set GITHUB_TOKEN, then rerun:
     shiplog intake --last-6-months --explain
```

Then the user can run:

```bash
shiplog journal add --from-repair repair_001_manual_empty
shiplog intake --last-6-months --explain
shiplog repair diff --latest
```

and see whether the latest rerun improved the packet, cleared repair items,
changed source state, or left gaps open.

## Machine End State

The intake report should expose stable repair items for local UI, agents, and
future TUI surfaces. The exact schema belongs in the spec, but the intended
shape is:

```json
{
  "repair_id": "repair_001_manual_empty",
  "source_key": "manual",
  "source_label": "Manual",
  "kind": "manual_evidence_missing",
  "reason": "No manual evidence events were found.",
  "action": {
    "kind": "journal_add",
    "command": "shiplog journal add --from-repair repair_001_manual_empty"
  },
  "clears_when": "manual source contributes at least one event"
}
```

Required machine outcomes:

- repair items have deterministic IDs within a report;
- repair items link to `source_key` and `source_label` when source-owned;
- repair items carry a kind, reason, action, and clear condition;
- repair actions do not include secret values, private opaque IDs, or unsafe
  provider mutations;
- report JSON remains the machine-readable boundary for repair commands;
- older report compatibility is preserved or explicitly documented.

## Architecture Rule

Repair actions are generated from intake report receipts.

Repair planning may read:

- `source_decisions`;
- `source_freshness`;
- `repair_sources`;
- `needs_attention`;
- `evidence_debt`;
- `journal_suggestions`;
- `next_commands`;
- `actions`;
- `artifacts`.

Repair planning must not independently decide whether GitHub, GitLab, Jira,
Linear, local git, JSON, manual evidence, cache, redaction, or sharing is
healthy. The source and report layers produce receipts; the repair planner
turns receipts into actions.

## Success Criteria

This lane succeeds when the following are true:

- `shiplog repair plan --latest` reads the latest `intake.report.json` and
  prints a stable, copyable repair queue;
- missing latest reports produce the exact intake command that creates one;
- repair items are derived from existing report receipts, not from live source
  discovery;
- repair items are machine-readable in report JSON with deterministic
  `repair_id` values;
- manual-evidence gaps can be repaired through
  `shiplog journal add --from-repair <repair_id>`;
- skipped provider sources produce setup/rerun guidance without mutating the
  provider;
- stale or cached evidence guidance follows `source_freshness` and cache
  receipts instead of guessing;
- invalid repair IDs fail with a useful error;
- repair actions are safe to show and do not leak secret values or source
  opaque IDs across non-internal profiles;
- `shiplog repair diff --latest` or the chosen diff command can compare the
  latest two runs and show cleared, changed, new, and still-open repair items;
- an end-to-end product test proves that a cold first run can produce a repair
  plan, add manual evidence from a repair item, rerun intake, and improve the
  packet.

## Non-Goals

This proposal does not include:

- generated performance narratives;
- team rollups or manager dashboards;
- live-provider mutation, issue creation, ticket edits, or API-side repair;
- a second source-state classifier outside the intake report;
- scraping `intake.report.md` for machine control flow;
- broad report schema redesign unrelated to repair items;
- reopening protected-fields or `disallowed_fields`;
- changing 0.6.0 first-run release behavior as part of the proposal PR.

## Proposed Artifact Stack

Land the lane in small semantic PRs:

1. This proposal:
   `docs/proposals/SHIPLOG-PROP-0002-evidence-repair-loop.md`.
2. Evidence repair loop spec:
   `docs/specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md`.
3. Repair-action ADR:
   `docs/adr/SHIPLOG-ADR-0006-repair-actions-are-receipt-derived.md`.
4. Implementation plan and active goal:
   `plans/evidence-repair/implementation-plan.md` and
   `.shiplog/goals/active.toml`.
5. Report repair item model.
6. `shiplog repair plan --latest`.
7. `shiplog journal add --from-repair <repair_id>`.
8. Repair diff between latest runs.
9. End-to-end product proof.
10. Worked guide.
11. Release prep for the evidence repair loop release.

The proposal explains why the lane exists. The spec defines the behavior
contract. The ADR records the durable architecture decision. The plan sequences
PRs, proof commands, rollback, and follow-up.

## Proof Map

Existing proof surfaces to link from future specs and plans:

- [`docs/schemas/intake-report-v1.md`](../schemas/intake-report-v1.md):
  current report receipt surface.
- [`contracts/schemas/intake-report.v1.schema.json`](../../contracts/schemas/intake-report.v1.schema.json):
  report schema and secret-vocabulary firewall.
- [`docs/specs/SHIPLOG-SPEC-0001-rapid-first-intake.md`](../specs/SHIPLOG-SPEC-0001-rapid-first-intake.md):
  first-run intake contract.
- [`docs/specs/SHIPLOG-SPEC-0002-intake-report-v1.md`](../specs/SHIPLOG-SPEC-0002-intake-report-v1.md):
  intake report JSON and Markdown contract.
- [`docs/specs/SHIPLOG-SPEC-0003-source-identity.md`](../specs/SHIPLOG-SPEC-0003-source-identity.md):
  canonical source identity contract.
- [`docs/adr/SHIPLOG-ADR-0001-ingest-output-is-receipt-boundary.md`](../adr/SHIPLOG-ADR-0001-ingest-output-is-receipt-boundary.md):
  adapter receipt boundary.
- [`docs/adr/SHIPLOG-ADR-0002-machine-source-keys-vs-display-labels.md`](../adr/SHIPLOG-ADR-0002-machine-source-keys-vs-display-labels.md):
  machine source keys and display labels.
- [`docs/adr/SHIPLOG-ADR-0003-stale-requires-cachelookup.md`](../adr/SHIPLOG-ADR-0003-stale-requires-cachelookup.md):
  stale freshness proof boundary.
- [`apps/shiplog/tests/intake_cold_start.rs`](../../apps/shiplog/tests/intake_cold_start.rs):
  cold first-run report contract.
- [`apps/shiplog/tests/front_door_first_pack_smoke.rs`](../../apps/shiplog/tests/front_door_first_pack_smoke.rs):
  first-pack smoke path.
- [`apps/shiplog/tests/cli_integration.rs`](../../apps/shiplog/tests/cli_integration.rs):
  CLI report, intake, open, and future repair command coverage.

Docs-only proposal validation:

```bash
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Behavior PRs should add targeted tests from the owning spec and update schemas,
schema docs, and guides only when the user-visible or machine-readable contract
changes.

## Alternatives Considered

### Keep repair guidance as prose only

Rejected. Prose in `intake.report.md` is useful for humans, but local UI,
agents, and future TUI surfaces need stable JSON instead of Markdown scraping.

### Re-run source discovery inside repair commands

Rejected. Repair commands should consume report JSON and guide the user. They
should not become a competing source-state engine.

### Hard-code CLI advice separately from report JSON

Rejected. Separate advice would drift from `source_decisions`,
`source_freshness`, `repair_sources`, `evidence_debt`, and `actions`. The
repair plan should render receipt-derived actions.

### Make repair commands mutate providers

Rejected. Provider setup can be explained, but shiplog should not create or
edit GitHub, GitLab, Jira, or Linear data as part of this lane.

### Skip repair diff

Deferred only if needed for PR size. The diff is high value because it proves
that repair work improved the packet instead of merely changing files.

## Linked Work

Prior lane closure:

- [PR #247](https://github.com/EffortlessMetrics/shiplog/pull/247):
  archived the user-polish 0.6.0 active goal.
- [PR #248](https://github.com/EffortlessMetrics/shiplog/pull/248):
  recorded the 0.6.0 publish-order receipt and release closure.

Related source-of-truth work:

- [PR #230](https://github.com/EffortlessMetrics/shiplog/pull/230):
  source-of-truth scaffold.
- [PR #231](https://github.com/EffortlessMetrics/shiplog/pull/231):
  user-polish proposal.
- [PR #232](https://github.com/EffortlessMetrics/shiplog/pull/232):
  rapid first-intake spec.
- [PR #233](https://github.com/EffortlessMetrics/shiplog/pull/233):
  intake-report and source-identity specs.
- [PR #234](https://github.com/EffortlessMetrics/shiplog/pull/234):
  receipt, source identity, and stale ADRs.
- [PR #235](https://github.com/EffortlessMetrics/shiplog/pull/235):
  user-polish implementation plan and active goal.

## Exit Criteria

The lane can close when:

- the proposal, spec, ADR, implementation plan, and active goal manifest have
  landed and link to each other;
- `intake.report.json` exposes stable repair items;
- `shiplog repair plan --latest` renders repair items from the latest report;
- `shiplog journal add --from-repair <repair_id>` handles manual evidence gaps
  without destructive overwrite;
- repair diff shows cleared, changed, new, and still-open repair items between
  runs;
- end-to-end product proof shows that a cold run with `Needs evidence` can be
  repaired and rerun into a more useful packet;
- a short guide teaches the repair loop without restating the spec;
- release notes explain that shiplog now turns first-run gaps into guided
  repairs.

North star: shiplog's product receipts and development receipts should keep
the same shape: claim, source, freshness, proof, gaps, and next action.
