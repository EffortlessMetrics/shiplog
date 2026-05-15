# SHIPLOG-ADR-0007: Claim Candidates Are Evidence Scaffolds

Status: accepted
Date: 2026-05-15
Related proposal:
[`SHIPLOG-PROP-0004-review-ready-packet-quality`](../proposals/SHIPLOG-PROP-0004-review-ready-packet-quality.md)
Related spec:
[`SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates`](../specs/SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates.md)
Related repair spec:
[`SHIPLOG-SPEC-0005-evidence-repair-loop`](../specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md)
Related report spec:
[`SHIPLOG-SPEC-0002-intake-report-v1`](../specs/SHIPLOG-SPEC-0002-intake-report-v1.md)

## Context

The 0.8.0 Evidence Repair Loop made Shiplog useful after a rough first run:
intake can identify repair items, repair planning can guide local fixes,
journal repair can add local evidence, and repair diff can prove whether a
later run improved.

The next product step is not more repair plumbing. A repaired packet should
help the user understand what the evidence can support:

- what evidence is strong;
- what still needs human context;
- which claim candidates are defensible from receipts;
- which share profiles are safe or blocked;
- what changed since the last run.

That creates a product safety boundary. If Shiplog writes final self-review
paragraphs, invents impact, or converts activity into performance ratings, it
will cross from evidence tooling into unsupported evaluation. The strongest
product is narrower: make the defensible path easy without pretending to know
the user's full context.

Shiplog already has receipt-boundary decisions that apply here:

- [`SHIPLOG-ADR-0001`](SHIPLOG-ADR-0001-ingest-output-is-receipt-boundary.md)
  puts adapter evidence at the receipt boundary;
- [`SHIPLOG-ADR-0006`](SHIPLOG-ADR-0006-repair-actions-are-receipt-derived.md)
  requires repair actions to be derived from report receipts;
- [`SHIPLOG-SPEC-0006`](../specs/SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates.md)
  defines packet quality as readiness, evidence strength, claim candidates,
  missing-context prompts, share posture, and quality diffs.

Claim candidates need the same boundary.

## Decision

Shiplog may produce claim candidates, evidence links, missing-context prompts,
packet readiness, evidence strength, share posture, and quality diffs.

Shiplog must not generate final performance-review prose, invent
accomplishments, infer impact, rate employee performance, or present confidence
as a performance score.

A claim candidate is an evidence scaffold. It may say:

- a neutral claim title or theme;
- which receipts support it;
- which repair keys, sources, workstreams, or artifacts are involved;
- how strong the evidence is;
- what human context is still missing;
- which share profiles may show it.

A claim candidate must not say:

- a final first-person review paragraph;
- who benefited unless a receipt says so;
- business or user impact unless a receipt says so;
- a performance judgment;
- a claim without supporting receipts.

The machine source of truth for claim candidates is report JSON and durable
artifacts, not packet Markdown. Packet Markdown may render claim candidates for
humans, but commands, agents, UI, TUI, and share flows must not scrape Markdown
to recover claim state.

The core claim-candidate path must be deterministic and not require an LLM. A
future optional LLM feature may only operate on receipt-backed scaffolds and
must preserve this ADR's safety boundary.

## Consequences

- `packet_quality.claim_candidates` must require supporting receipts and stable
  claim IDs before implementation lands.
- Packet rendering may add a Claim Candidates section, but the section must
  render scaffolds and missing-context prompts instead of review prose.
- Packet readiness and evidence strength may explain evidence quality, but they
  must not become productivity scores, ratings, rankings, or employee-quality
  signals.
- `missing_context_prompts` ask the user to add context. They do not supply the
  missing impact, beneficiary, tradeoff, or outcome.
- `safe_profiles` and share posture decide where a scaffold may appear. A claim
  candidate that is not safe for a profile must be suppressed or clearly
  explained as blocked for that profile.
- Claim generation must be reproducible from `intake.report.json` and durable
  artifacts. It must not re-query providers, inspect private provider state, or
  infer facts from Markdown wording.
- Manual-only evidence can support a candidate only with a manual-only or
  weaker evidence-strength status unless another receipt supports a stronger
  status.
- Schema, docs, packet tests, CLI tests, and product proof must preserve old
  report compatibility when `packet_quality` is absent.

## Alternatives Considered

### Generate Final Review Paragraphs

Rejected. A paragraph that sounds polished can still invent impact, flatten
caveats, hide missing context, or imply a performance judgment that receipts do
not support. Shiplog should help the user write a better review, not write it
for them.

### Present Confidence Scores

Rejected. Numeric confidence scores are easy to misread as productivity,
employee quality, or performance ratings. The evidence-strength vocabulary is
intentionally categorical and receipt-backed.

### Emit Claims Without Receipts

Rejected. Unsupported claims would make the packet look more complete while
reducing trust. Every claim candidate needs at least one receipt reference and
clear caveats when evidence is partial, manual-only, skipped, or context-limited.

### Let An LLM Decide The Claims

Rejected for the core path. The product contract must work offline from
Shiplog receipts. Future optional LLM assistance can be considered only after
the deterministic scaffold and safety checks exist.

### Scrape Packet Markdown For Claim State

Rejected. Markdown is a human rendering. Report JSON and durable artifacts are
the machine boundary. Scraping Markdown would make headings, wording, and
section order accidental APIs.

### Build Manager Dashboards First

Rejected for this lane. The next unlock is making one user's repaired packet
review-ready. Team dashboards, rollups, and manager workflows need separate
consent, privacy, and policy design.

## Affected Specs, Plans, Tests, And Schemas

- [`SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates`](../specs/SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates.md)
  defines packet readiness, evidence strength, claim candidates,
  missing-context prompts, share posture, and quality diff behavior.
- [`SHIPLOG-SPEC-0002-intake-report-v1`](../specs/SHIPLOG-SPEC-0002-intake-report-v1.md)
  remains the report JSON compatibility boundary.
- [`SHIPLOG-SPEC-0005-evidence-repair-loop`](../specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md)
  defines the repair receipts that packet quality can build on.
- [`contracts/schemas/intake-report.v1.schema.json`](../../contracts/schemas/intake-report.v1.schema.json)
  should carry the optional `packet_quality` schema when implementation lands.
- [`docs/schemas/intake-report-v1.md`](../schemas/intake-report-v1.md)
  should document `packet_quality` absence behavior, receipt references, and
  safety vocabulary.
- [`apps/shiplog/tests/cli_integration.rs`](../../apps/shiplog/tests/cli_integration.rs)
  should cover claim-candidate absence behavior, share explain, quality diff,
  and validation cases.
- [`apps/shiplog/tests/intake_cold_start.rs`](../../apps/shiplog/tests/intake_cold_start.rs)
  should cover first-run evidence-strength and readiness signals when
  implementation lands.
- [`apps/shiplog/tests/front_door_first_pack_smoke.rs`](../../apps/shiplog/tests/front_door_first_pack_smoke.rs)
  should prove that repair can improve a packet and unlock a review-ready
  scaffold.
- The future packet-quality implementation plan should sequence schema,
  report, packet rendering, share explain, quality diff, product proof, and
  guide work without generating review prose or violating receipt boundaries.
