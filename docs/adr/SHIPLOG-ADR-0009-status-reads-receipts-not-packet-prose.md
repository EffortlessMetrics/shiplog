# SHIPLOG-ADR-0009: Status Reads Receipts, Not Packet Prose

Status: accepted
Date: 2026-05-18
Related proposal:
[`SHIPLOG-PROP-0006-review-loop-status`](../proposals/SHIPLOG-PROP-0006-review-loop-status.md)
Related spec:
[`SHIPLOG-SPEC-0008-review-loop-status`](../specs/SHIPLOG-SPEC-0008-review-loop-status.md)
Related setup ADR:
[`SHIPLOG-ADR-0008-doctor-is-setup-readiness-not-intake`](SHIPLOG-ADR-0008-doctor-is-setup-readiness-not-intake.md)
Related repair ADR:
[`SHIPLOG-ADR-0006-repair-actions-are-receipt-derived`](SHIPLOG-ADR-0006-repair-actions-are-receipt-derived.md)

## Context

Guided Setup / Doctor made Shiplog safer before intake:

```text
doctor tells users and agents whether setup is ready.
```

The review-ready repair loop made Shiplog useful after intake:

```text
intake records evidence receipts, repair improves the packet, diff proves
movement, and share explain describes posture before rendering.
```

The next review-loop status lane needs to join those surfaces into one
read-only front panel. That creates a familiar architecture risk: status could
become "smart" by scraping `packet.md`, rerunning intake, probing providers,
rendering share artifacts, or summarizing review quality from human prose.

That would undermine the receipt boundaries Shiplog has already established:

- [`SHIPLOG-ADR-0001`](SHIPLOG-ADR-0001-ingest-output-is-receipt-boundary.md)
  puts adapter output at the intake receipt boundary;
- [`SHIPLOG-ADR-0006`](SHIPLOG-ADR-0006-repair-actions-are-receipt-derived.md)
  requires repair actions to come from report receipts;
- [`SHIPLOG-ADR-0008`](SHIPLOG-ADR-0008-doctor-is-setup-readiness-not-intake.md)
  keeps setup readiness separate from evidence collection;
- [`SHIPLOG-SPEC-0008`](../specs/SHIPLOG-SPEC-0008-review-loop-status.md)
  defines review-loop status as a read-only projection over setup, evidence,
  repair, diff, and share receipts.

Status needs to be useful without becoming another evidence plane.

## Decision

`shiplog status` reads typed models and durable machine receipts only.

Accepted status inputs are:

- the setup readiness model used by `doctor --setup` and
  `doctor --setup --json`;
- `intake.report.json`;
- source failure or source decision receipts when present;
- repair items and repair-plan data derived from report receipts;
- repair diff receipts when present;
- runs diff receipts when present;
- share explain, share verify, and share readiness receipts;
- bundle and share manifests when present.

Status may summarize those inputs, choose a safe next action, and explain which
blocking reason matters first. It must preserve the existing boundaries:

```text
setup readiness != evidence quality
evidence quality != repair readiness
repair readiness != share readiness
share explain != share render
status != packet prose
```

Status must not:

- scrape `packet.md`, `intake.report.md`, or other Markdown renderings as
  machine input;
- query GitHub, GitLab, Jira, Linear, or other provider APIs;
- rerun intake implicitly;
- mutate config, manual journals, provider state, or output artifacts;
- run repair commands automatically;
- render manager or public share artifacts;
- call an LLM or generate performance-review prose;
- infer evidence quality from token presence;
- infer share safety from human-facing prose;
- disclose secret values;
- imply that the paused `v0.9.0` release is approved.

Human status output and JSON status output must be two renderings of the same
typed status model. Human text may be clearer or more compact, but it must not
classify setup, evidence, repair, diff, or share state differently from the
machine model.

## Consequences

- `shiplog status --latest` can be a compact front panel without replacing
  doctor, intake, repair, diff, or share commands.
- `shiplog status --latest --json` can become an agent control surface without
  forcing agents to scrape terminal prose or packet Markdown.
- Missing or old receipts become explicit `unknown`, `blocked`, or rerun/setup
  guidance instead of panics or invented readiness.
- Status can label next actions as read-only or write-producing, but it cannot
  run those commands.
- Share rendering remains behind `share verify` and profile render commands;
  status may report share blockers but must not produce profile artifacts.
- Future implementation PRs must prove status agrees with the underlying
  receipt surfaces instead of creating an independent classifier.
- Release posture remains separate: this ADR documents architecture for an
  unreleased candidate lane and does not authorize tagging, publishing, GitHub
  release creation, workflow dispatch, or release-install smoke.

## Alternatives Considered

### Scrape Packet Markdown

Rejected. Packet Markdown is the human rendering. Treating it as a machine
source would make prose wording, heading names, and section order accidental
APIs. Status must use report JSON, setup models, and explicit receipts.

### Rerun Intake Before Printing Status

Rejected. Status is a read-only control-plane command. Intake is the evidence
collection command. Running intake implicitly would surprise users, create new
artifacts, and blur read/write posture.

### Probe Providers For Fresh Status

Rejected. Provider probing introduces network, auth, latency, rate-limit, and
secret-handling failure modes. Setup readiness can report credential presence;
intake owns provider evidence receipts.

### Render Share Artifacts To Determine Share State

Rejected. Share explain and verify describe readiness before rendering. Status
may summarize that readiness, but rendering profile artifacts is a separate
write-producing action.

### Use An LLM Summary

Rejected. Status should be deterministic, receipt-derived, and safe for agents.
An LLM summary would create unverifiable claims and could drift into generated
performance-review prose.

### Build A Dashboard Or TUI

Rejected for this lane. The needed product shape is one read-only command and a
JSON contract, not a new interface layer.

## Affected Specs, Plans, Tests, And Schemas

- [`SHIPLOG-PROP-0006-review-loop-status`](../proposals/SHIPLOG-PROP-0006-review-loop-status.md)
  proposes the review-loop front panel and non-goals.
- [`SHIPLOG-SPEC-0008-review-loop-status`](../specs/SHIPLOG-SPEC-0008-review-loop-status.md)
  defines the status vocabulary, model sections, next actions, receipt refs,
  and safety rules.
- Future status model and CLI tests must prove no writes, no provider calls, no
  Markdown scraping, deterministic ordering, old-receipt compatibility, and
  safe next-action behavior.
- Future schema docs must define the JSON contract from the typed status model,
  including stable keys, secret hygiene, receipt references, and additive
  compatibility.
- Future consistency tests must prove status agrees with doctor setup JSON,
  sources status, intake report JSON, repair plan, repair diff, runs diff, and
  share explain/verify receipts.
