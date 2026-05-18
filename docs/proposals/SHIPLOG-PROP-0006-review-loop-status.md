# SHIPLOG-PROP-0006: Review Loop Status

Status: proposed
Owner: product/docs
Created: 2026-05-18
Target release: after the paused 0.9.0 review-ready and guided setup decision

## Summary

Shiplog's next product lane should join the existing setup, evidence, repair,
diff, and share receipts into one read-only review-loop status surface.

Guided Setup / Doctor made the front door honest:

```text
Am I configured enough to start?
```

The review-ready repair loop made packet improvement visible:

```text
What evidence did I collect, what is missing, and did repair help?
```

The next missing surface is the recurring operating view:

```text
What state is my review loop in, and what is the next safe action?
```

The target user path still exists as separate commands:

```bash
shiplog init --guided
shiplog doctor --setup
shiplog sources status
shiplog doctor --setup --json
shiplog intake --last-6-months --explain
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog runs diff --latest
shiplog share explain manager --latest
```

This lane proposes a compact status front panel over those receipts:

```bash
shiplog status --latest
shiplog status --latest --json
```

Status should not become a dashboard, TUI, dry-run intake, or report renderer.
It should be a read-only control-plane surface that tells humans and agents what
is ready, blocked, caveated, safe to run next, and unsafe to render or write.

## Problem

The setup-readiness lane made preflight state clear, but users and agents still
have to mentally join too many artifacts after the first intake:

- `doctor --setup` explains setup readiness;
- `sources status` explains source setup only;
- `doctor --setup --json` exposes setup state for agents;
- `intake.report.json` records evidence, packet quality, repair items, and
  share posture;
- `repair plan` explains which repair actions are safe;
- `repair diff` explains repair movement after rerun;
- `runs diff` explains packet-quality movement across runs;
- `share explain` explains profile posture before rendering.

Those surfaces are correct, but the recurring review user still has to assemble
the cockpit manually. That is too much work for a weekly check-in, deadline
packet prep, or agent loop that needs to choose the next bounded command.

The product gap is not lack of receipts. The gap is that no single read-only
surface answers:

- am I set up?
- did I collect enough evidence?
- what is still missing?
- what changed since the last run?
- what can I safely repair?
- what can I safely share?
- what should I do next?

Shiplog should answer from receipts, not vibes.

## Target Users

Primary users:

- a recurring review user who wants to check progress weekly or monthly;
- a deadline user who needs the fastest safe next action;
- a local-only user who expects provider gaps to remain caveats, not blockers;
- a token-backed user who needs setup gaps separated from evidence weakness;
- a manager-share user who needs share safety before rendering.

Secondary users:

- agents that need a contract-backed status surface after setup readiness;
- maintainers checking that status remains receipt-derived and read-only;
- future release reviewers deciding whether the review loop is coherent enough
  to ship.

## Product End State

The lane is done when a user can run:

```bash
shiplog status --latest
```

and get a compact, human-readable status view like:

```text
Review loop status: Needs evidence repair

Setup:
- ready with caveats

Latest run:
- packet readiness: needs_evidence
- included sources: manual 0, git 4
- unavailable sources: GitHub token missing

Repair:
- 2 open repair items
- 1 safe write command available
- 0 setup-blocked write commands

Diff:
- no prior comparable run

Share:
- manager blocked: SHIPLOG_REDACT_KEY missing
- public blocked: strict review not yet possible

Next:
1. shiplog repair plan --latest [read-only]
2. shiplog journal add --from-repair <repair_id> [writes]
3. shiplog intake --last-6-months --explain [writes]
```

The status view should make the safe sequence obvious without hiding the
underlying command-specific surfaces. It should say which durable receipt was
read, which blocker matters first, which actions write files, and which actions
are read-only.

## Machine End State

The exact JSON shape belongs in the follow-up spec, but this proposal expects a
contract-backed review-loop status model:

```text
review_loop_status:
  overall_status
  setup_summary
  latest_run
  packet_readiness
  source_summary
  repair_summary
  diff_summary
  share_summary
  next_actions[]
  blocking_reasons[]
  receipt_refs[]
```

Agents should be able to consume:

```bash
shiplog status --latest --json
```

and make bounded decisions:

- setup blocked -> do not run evidence repair;
- source unavailable -> ask for credentials or stay local-only;
- packet weak -> run `repair plan` before share rendering;
- repair cleared -> rerun intake and compare with `repair diff`;
- share blocked -> do not render manager or public packets;
- status ready -> explain, verify, or share only through the allowed profile
  path.

Required machine outcomes:

- stable keys and deterministic ordering;
- no secret values;
- no provider network calls;
- no writes;
- no Markdown scraping;
- no generated performance-review prose;
- next actions carry read/write posture;
- receipt references point back to durable machine sources.

## Status Vocabulary

The follow-up spec should define final enums. Suggested starting statuses:

```text
ready_to_collect
needs_setup
needs_evidence
needs_repair
repair_in_progress
ready_with_caveats
ready_to_explain_share
share_blocked
blocked
unknown
```

The status contract must keep these distinctions explicit:

```text
setup readiness != evidence quality
repair readiness != share readiness
share explain != share render
status != packet prose
```

## Receipt Sources

Status may read durable local receipts and typed models:

- setup readiness model used by `doctor --setup`;
- `intake.report.json`;
- repair-plan data derived from `repair_items`;
- repair-diff report data when available;
- runs-diff report data when available;
- share explain / share readiness data;
- share manifests when already present.

Status must not:

- query providers;
- mutate config;
- render share artifacts;
- scrape `packet.md` as a machine source;
- infer evidence health from token presence;
- call an LLM;
- generate final review prose.

## Success Criteria

This lane succeeds when the following are true:

- `shiplog status --latest` prints a compact setup/evidence/repair/diff/share
  summary from receipts;
- `shiplog status --latest --json` exposes the same state for agents;
- status output names the newest run or clearly says no comparable run exists;
- setup blockers match `doctor --setup --json`;
- source blockers match `sources status`;
- packet readiness matches `intake.report.json`;
- repair counts and safe write commands match `repair plan`;
- diff state matches `repair diff` / `runs diff` when those receipts exist;
- share blockers match `share explain` and fail-closed share commands;
- next actions are deterministic and label read-only versus write-producing
  commands;
- status never offers share rendering while share is blocked;
- status never offers evidence-repair writes while setup is blocked;
- old reports and missing runs degrade into clear rerun/setup guidance without
  panic or fake readiness;
- docs teach status as a recurring review-loop front panel, not a dashboard.

## Non-Goals

This proposal does not include:

- OAuth implementation;
- live provider probing;
- automatic repair;
- automatic share rendering;
- dashboard, TUI, GUI, or manager rollup work;
- LLM summaries or generated review prose;
- new source adapters;
- public crate changes;
- release execution;
- replacing `doctor`, `intake`, `repair plan`, `repair diff`, `runs diff`, or
  `share explain`.

## Alternatives Considered

### Keep separate commands only

Rejected. The separate commands remain important, but recurring users and
agents need one control-plane read before choosing which command should run
next.

### Extend doctor to cover the whole loop

Rejected. Doctor is setup readiness, not evidence state. Extending doctor would
blur the boundary that the setup lane deliberately created.

### Parse packet Markdown

Rejected. Packet Markdown is for humans. Status must read machine receipts so
agents do not scrape rendered prose.

### Build a dashboard or TUI

Rejected. The immediate product need is a durable front panel that works in the
CLI and JSON. A UI layer would add surface area without improving the receipt
contract.

### Auto-run the next action

Rejected. Status should explain the safe next action, not mutate setup, rerun
intake, repair evidence, or render share packets on the user's behalf.

## Proposed Artifact Stack

Land the lane in small semantic PRs:

1. This proposal:
   `docs/proposals/SHIPLOG-PROP-0006-review-loop-status.md`.
2. Review-loop status contract spec.
3. ADR: status reads receipts, not packet prose.
4. Internal review-loop status model.
5. `shiplog status --latest` human CLI surface.
6. `shiplog status --latest --json` agent surface.
7. Review-loop status JSON schema and examples.
8. Consistency proof against doctor, sources status, intake report, repair plan,
   repair diff, runs diff, and share explain.
9. Dogfood transcript for the status front panel.
10. Recurring review-loop guide.
11. Release posture update that keeps 0.9 held unless explicitly approved.

The proposal explains why the lane exists. The spec defines the machine and
user-facing status contract. The ADR records the receipt boundary. Implementation
PRs should preserve read-only, no-network, no-render behavior unless a later
spec deliberately scopes otherwise.

## Proof Map

Existing proof surfaces to link from future specs and plans:

- [`docs/proposals/SHIPLOG-PROP-0005-guided-setup-doctor.md`](SHIPLOG-PROP-0005-guided-setup-doctor.md):
  setup readiness made the front door explicit before intake.
- [`docs/specs/SHIPLOG-SPEC-0007-setup-readiness.md`](../specs/SHIPLOG-SPEC-0007-setup-readiness.md):
  the setup readiness contract status must preserve.
- [`docs/adr/SHIPLOG-ADR-0008-doctor-is-setup-readiness-not-intake.md`](../adr/SHIPLOG-ADR-0008-doctor-is-setup-readiness-not-intake.md):
  the boundary that doctor is setup readiness, not evidence state.
- [`docs/schemas/setup-readiness-v1.md`](../schemas/setup-readiness-v1.md):
  the setup JSON contract that status should summarize rather than duplicate.
- [`docs/product/guided-setup-transcript.md`](../product/guided-setup-transcript.md):
  the guided setup transcript that proves the setup front door.
- [`docs/product/review-ready-loop-transcript.md`](../product/review-ready-loop-transcript.md):
  the review-ready transcript that proves intake, repair, diff, and share
  explain already have receipts.
- [`docs/product/review-ready-dogfood-matrix.md`](../product/review-ready-dogfood-matrix.md):
  the dogfood matrix for review-ready evidence and share posture.
- [`docs/release/0.9.0-release-decision.md`](../release/0.9.0-release-decision.md):
  the current decision to keep the 0.9 hold active after setup-readiness
  operationalization.
