# SHIPLOG-PROP-0004: Review-Ready Packet Quality

Status: proposed
Owner: product/docs
Created: 2026-05-15
Target release: after 0.8.0 Evidence Repair Loop

## Summary

Shiplog's next product lane should turn repaired evidence into review-ready
packet guidance: claim candidates, supporting receipts, missing-context prompts,
readiness, and share posture. The 0.8.0 Evidence Repair Loop made a rough first
packet improvable. The next step is to help the user understand how good the
repaired packet is and what defensible claims it can support.

This lane should not generate performance-review prose. Shiplog should provide
an evidence scaffold that helps the user write honestly:

```text
Here are the strongest claim candidates this evidence supports.
Here is the evidence behind each candidate.
Here is what remains weak or missing.
Here is what is safe to share.
Here is what still needs human context.
```

The line is deliberate: Shiplog may suggest claim candidates and prompts, but it
must not invent accomplishments, impact, or performance narratives.

## Problem

After 0.8.0, the user can run:

```bash
shiplog intake --last-6-months --explain
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog open packet --latest
```

That loop answers whether a packet improved. It still leaves the user doing the
hardest review translation work:

- which claims the evidence can safely support;
- which evidence belongs under each claim;
- which parts are still weak, skipped, stale, or manual-only;
- what context the user needs to add before a claim is review-ready;
- whether a manager or public share profile is safe enough to use;
- what changed since the last run beyond raw repair-key movement.

Without this layer, Shiplog can still feel like a report generator: it shows
events, gaps, and repairs, but the user has to infer what to do next. A user
should not have to ask, "Okay, now what?"

## Target Users

Primary users:

- a self-reviewer who has repaired a rough packet and needs claim-ready
  guidance without generated prose;
- a deadline-pressure user who needs to know which claims are defensible now and
  which need more context;
- a manager-prep user who needs to understand what is safe to share;
- an agent consumer that needs machine-readable packet-quality signals instead
  of scraping Markdown.

Secondary users:

- maintainers checking that claim guidance stays tied to receipts;
- future local UI, TUI, or editor integrations that need stable readiness and
  claim-candidate surfaces.

## Product End State

The lane is done when a repaired packet can answer:

```text
What did shiplog find?
What did it skip?
What is missing?
What can I safely repair?
Did the repair improve the packet?
What is safe to share?
What claims can this evidence support?
What still needs human context?
```

The target user path is:

```bash
cargo install shiplog

shiplog intake --last-6-months --explain
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog open packet --latest
shiplog share prepare manager --latest
```

The repaired packet should include a visible readiness section near the top:

```markdown
## Packet Readiness

Ready with caveats.

Strong:
- Manual evidence added from repair repair_001_manual_empty.

Still weak:
- GitHub skipped: missing token.
- Jira skipped: missing token.

Next:
- shiplog repair plan --latest
```

It should also include deterministic claim candidates:

```markdown
## Claim Candidates

### Release reliability

Evidence:
- PR #283
- package-boundary audit
- release-prep proof

Missing context:
- What failure mode did this prevent?
- Who benefited?
```

The claim candidate is not the final review text. It is a receipt-backed
starting point plus prompts for the user to add human context.

## Machine End State

The exact schema belongs in the follow-up spec, but this proposal expects the
report to expose these concepts without requiring an LLM:

```text
evidence_strength
claim_candidates
missing_context_prompts
share_posture
packet_readiness
```

Suggested evidence-strength values:

```text
strong
partial
manual_only
source_skipped
needs_context
```

Suggested claim-candidate shape:

```json
{
  "claim_id": "claim_release_reliability",
  "title": "Release reliability",
  "supporting_repair_keys": ["manual:no_events"],
  "supporting_sources": ["manual", "github"],
  "evidence_strength": "partial",
  "missing_context": [
    "What failure mode did this prevent?",
    "Who benefited?"
  ],
  "safe_profiles": ["manager"]
}
```

Required machine outcomes:

- readiness and claim candidates are deterministic from report receipts;
- claim candidates include supporting receipt references;
- missing-context prompts are explicit and conservative;
- share posture explains included, removed, and blocked material by profile;
- old compatible reports remain valid;
- no claim candidate is emitted without supporting evidence.

## Relationship To Evidence Repair Loop

This lane builds on 0.8.0 repair receipts instead of replacing them:

- `repair_items` describe what is missing or repairable;
- `repair_key` gives stable before/after identity;
- `source_key` and `source_label` keep source identity explicit;
- `source_freshness` explains fresh, stale, skipped, cached, or unavailable
  source state;
- `repair plan` tells the user how to improve the packet;
- `journal add --from-repair` adds local manual evidence;
- `repair diff` proves what changed across runs;
- packet output becomes the review-facing surface for readiness and claim
  candidates.

The non-obvious unlock is the before/after signal:

```text
This packet improved because this repair cleared.
This evidence is still weak because this source remains skipped.
This claim candidate is strong because it has source-backed receipts.
This claim candidate needs context because it only has manual evidence.
```

## Safety Rules

- Do not generate final performance-review prose.
- Do not score employees, rank people, or present confidence as a performance
  rating.
- Do not invent impact, accomplishments, beneficiaries, or outcomes.
- Do not emit claim candidates without receipt-backed support.
- Do not require an LLM for the core path.
- Do not mutate GitHub, GitLab, Jira, Linear, or other providers.
- Do not leak tokens, secret values, private opaque IDs, or internal-only source
  details through readiness, claims, prompts, or share posture.
- Do not make manager dashboards, team rollups, OAuth flows, plugin APIs, GUI,
  or TUI work part of this lane.
- Preserve report compatibility or document any compatibility boundary before
  implementation.

## Success Criteria

This lane succeeds when the following are true:

- packets include a visible, deterministic Packet Readiness section;
- report JSON exposes schema-backed evidence-strength and packet-readiness
  data;
- claim candidates are deterministic, receipt-backed, and conservative;
- claim candidates include missing-context prompts instead of invented prose;
- share posture can explain what a manager or public profile includes, removes,
  or blocks;
- a quality diff shows how readiness, claim candidates, and weak areas changed
  across runs;
- the product proof covers cold intake, repair plan, journal repair, rerun,
  repair diff, packet readiness improvement, claim candidate appearance, and
  share posture explanation;
- docs teach the user how to interpret readiness, claim candidates, missing
  context, and share posture;
- the lane can be released without reopening repair-loop plumbing.

## Non-Goals

This proposal does not include:

- generated performance narratives;
- employee scoring or manager ratings;
- LLM-required claim generation;
- private-provider mutation or ticket edits;
- OAuth setup;
- team dashboards or manager rollups;
- plugin APIs;
- GUI or TUI work;
- broad report schema redesign unrelated to packet quality;
- changing repair-loop behavior unless the follow-up spec explicitly scopes it.

## Alternatives Considered

### Generate final review prose

Rejected. That would make Shiplog sound more helpful while increasing the risk
of invented impact, unverifiable claims, and review text that the user cannot
defend. The safer product role is evidence scaffold plus missing-context
prompts.

### Build share posture before packet readiness

Rejected for this lane order. Share posture matters, but the packet first needs
to explain its evidence strength and claim support. Otherwise sharing answers
"can I send this?" before Shiplog has answered "what does this support?"

### Use an LLM as the claim engine

Rejected for the core path. An optional future assistive layer may be useful,
but the first contract must be deterministic, receipt-backed, and testable
without network or model access.

### Jump to manager dashboards

Rejected. Dashboards and rollups need reliable per-packet claim and share
posture first. This lane should make one user's packet review-ready before
aggregating anything.

## Proposed Artifact Stack

Land the lane in small semantic PRs:

1. This proposal:
   `docs/proposals/SHIPLOG-PROP-0004-review-ready-packet-quality.md`.
2. Packet quality and claim-candidate spec.
3. ADR: claim candidates are evidence scaffold, not generated review prose.
4. Evidence-strength model.
5. Packet readiness section.
6. Receipt-backed claim-candidate model.
7. Packet claim-candidate section with missing-context prompts.
8. Share-posture explanation.
9. Run quality diff.
10. End-to-end product proof.
11. Worked guide.
12. Release prep for the review-ready packet release.

The proposal explains why the lane exists. The spec defines the machine and
user-facing contract. The ADR records the safety boundary. The implementation
plan should sequence proof commands, rollback, and stop conditions before
behavior changes start.

## Proof Map

Existing proof surfaces to link from future specs and plans:

- [`docs/guides/evidence-repair-loop.md`](../guides/evidence-repair-loop.md):
  current user loop for repair plan, journal repair, rerun, and diff.
- [`docs/specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md`](../specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md):
  repair item and repair command contract.
- [`docs/adr/SHIPLOG-ADR-0006-repair-actions-are-receipt-derived.md`](../adr/SHIPLOG-ADR-0006-repair-actions-are-receipt-derived.md):
  no-second-classifier repair boundary.
- [`docs/schemas/intake-report-v1.md`](../schemas/intake-report-v1.md):
  intake report JSON surface and compatibility notes.
- [`contracts/schemas/intake-report.v1.schema.json`](../../contracts/schemas/intake-report.v1.schema.json):
  machine-readable report schema.
- [`docs/release/0.8.0-readiness.md`](../release/0.8.0-readiness.md):
  shipped Evidence Repair Loop release receipt.
- [`.shiplog/goals/archive/2026-05-15-evidence-repair-0.8.0.toml`](../../.shiplog/goals/archive/2026-05-15-evidence-repair-0.8.0.toml):
  closed lane archive and release receipts.
