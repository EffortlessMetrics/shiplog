# Review-ready loop dogfood transcript

> **Status:** post-0.8 soak receipt for the paused 0.9 candidate.
> **Release posture:** this transcript does not approve tagging, publishing, or
> GitHub release execution.
> **Date:** 2026-05-17.

This records one real local-history run against the `shiplog` repository. It is
proof, not a pitch: the loop improved the packet, kept caveats visible, and did
not turn the packet into generated review prose.

## Setup

The run used a temporary workspace under
`target/codex-review-ready-transcript-397/` with:

- `sources.git.enabled = true`, pointed at `H:/Code/Rust/shiplog`;
- `sources.manual.enabled = true`, pointed at the temporary manual journal;
- no provider tokens;
- no redaction key.

The transcript used a local debug build and an explicit output directory:

```bash
shiplog init --source git --source manual --force
shiplog intake --config target/codex-review-ready-transcript-397/workspace/shiplog.toml --out target/codex-review-ready-transcript-397/out --last-6-months --explain --no-open
```

## Transcript

### 1. First Intake

Run: `merge_1779008650359479900`.

Observed:

- `Local git`: success, 369 events.
- `Manual`: success, 0 events.
- `Intake status`: `Needs curation`.
- `Packet readiness`: `Ready with caveats`.
- `Next` started with `shiplog repair plan --out ... --latest`.
- Direct write-producing evidence-debt commands remained contextual, not the
  top-level first action.

### 2. Repair Plan

```bash
shiplog repair plan --out target/codex-review-ready-transcript-397/out --latest
```

Observed:

- Repair queue had five items.
- The first repair was safe and copyable:
  `repair_001_manual_manual_evidence_missing_fixup_manual_cont`.
- Advisory items stayed explicit as `no safe copyable command`.
- Manager/public sharing stayed blocked on a redaction key.

### 3. Journal Repair

```bash
shiplog journal add --from-repair repair_001_manual_manual_evidence_missing_fixup_manual_cont --out target/codex-review-ready-transcript-397/out --latest
```

Observed:

- Wrote one manual event to the temporary manual journal.
- The command named the repair ID, report path, manual journal path, and clear
  condition.
- The next printed command was a rerun intake with the same config and output
  directory.

### 4. Rerun

```bash
shiplog intake --config target/codex-review-ready-transcript-397/workspace/shiplog.toml --out target/codex-review-ready-transcript-397/out --last-6-months --explain --no-open
```

Run: `merge_1779008667101652700`.

Observed:

- `Local git`: success, 369 events.
- `Manual`: success, 1 event.
- `Intake status`: `Needs curation`.
- `Packet readiness`: `Ready with caveats`.
- `Next` started with `shiplog repair diff --out ... --latest`, then
  `repair plan`, then read-only `share explain manager`.

### 5. Repair Diff

```bash
shiplog repair diff --out target/codex-review-ready-transcript-397/out --latest
```

Observed:

- Cleared: 1.
- The cleared item was the manual evidence repair key:
  `manual:manual_evidence_missing:fixup_manual_context_shiplog`.
- Still open: broad workstream, too many selected receipts, and share redaction
  required.
- The manual-context evidence debt changed from missing outcome note to a
  reminder that manual evidence is user-provided and should stay current.

### 6. Runs Diff

```bash
shiplog runs diff --out target/codex-review-ready-transcript-397/out --latest
```

Observed:

- Improved evidence events: 369 -> 370.
- Improved manual evidence count: 0 -> 1.
- Reported the manual repair key as cleared.
- Still weak:
  - broad workstream repair still open;
  - too many selected receipts still open;
  - share redaction still open;
  - packet readiness still `Ready with caveats`;
  - packet evidence still `partial`.

### 7. Packet

```bash
shiplog open packet --out target/codex-review-ready-transcript-397/out --latest --print-path
```

Observed:

- Packet path:
  `target/codex-review-ready-transcript-397/out/merge_1779008667101652700/packet.md`.
- The packet opened with `# Packet Readiness`.
- `# Claim Candidates` followed before coverage details.
- Report JSON had:
  - `packet_readiness = ready_with_caveats`;
  - `evidence_strength = partial`;
  - one claim candidate;
  - three missing-context prompts.

### 8. Share Explain

```bash
shiplog share explain manager --out target/codex-review-ready-transcript-397/out --latest
```

Observed:

- Status: `blocked`.
- Redaction key: missing.
- Included packet readiness, claim candidates, 370 events, and source-backed
  counts.
- Removed raw redaction key values, opaque provider identifiers, private source
  identifiers where policy applies, and full internal-only packet density.
- Needs review included packet readiness and all three evidence-debt caveats.
- Profile packet and share manifest were not written.
- Follow-up commands were labeled `Render when ready`.

### 9. Share Verify

```bash
shiplog share verify manager --out target/codex-review-ready-transcript-397/out --latest
```

Observed:

- Failed closed with:
  `manager share requires --redact-key or SHIPLOG_REDACT_KEY`.
- No manager profile packet was written.

## What Looked Right

- The first top-level handoff was read-first: `repair plan` before writes.
- Journal repair used the report-derived repair ID instead of asking the user to
  guess a YAML edit.
- Rerun handoff moved to `repair diff` before returning to planning.
- Repair diff cleared only the manual repair whose matching evidence appeared.
- Runs diff showed improvement without claiming the packet was fully strong.
- Packet readiness and claim candidates were visible at the top of the packet.
- `share explain manager` was read-only and kept readiness/evidence debt visible.
- `share verify manager` failed closed without a redaction key.

## Remaining Caveats

- The packet still needs curation: one broad workstream and too many selected
  receipts remain open.
- Manual evidence is present, but it is user-provided and needs real outcome
  context before sharing.
- Manager rendering remains blocked until a redaction key is supplied.
- No provider-token paths were exercised in this transcript.

## Intentionally Not Fixed

- Did not split the broad `shiplog` workstream.
- Did not trim selected receipts.
- Did not render manager/public packets.
- Did not add provider tokens or source adapters.
- Did not generate performance-review prose.

## Release Decision Signal

This transcript supports keeping the 0.9 candidate in soak with stronger release
confidence. It shows the core loop is calm and receipt-derived for local git plus
manual repair, but it does not by itself approve release execution. A release
decision still needs current matrix review, green main CI, current hold receipts,
and explicit owner approval.

## Cleanup

The temporary dogfood workspace under `target/codex-review-ready-transcript-397/`
was removed after this transcript was recorded.
