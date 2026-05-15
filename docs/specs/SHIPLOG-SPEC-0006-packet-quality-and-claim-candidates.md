# SHIPLOG-SPEC-0006: Packet Quality And Claim Candidates

Status: proposed
Owner: product/schema
Created: 2026-05-15
Related proposal:
[`SHIPLOG-PROP-0004-review-ready-packet-quality`](../proposals/SHIPLOG-PROP-0004-review-ready-packet-quality.md)
Related report spec:
[`SHIPLOG-SPEC-0002-intake-report-v1`](SHIPLOG-SPEC-0002-intake-report-v1.md)
Related repair spec:
[`SHIPLOG-SPEC-0005-evidence-repair-loop`](SHIPLOG-SPEC-0005-evidence-repair-loop.md)

## Purpose

This spec defines the contract for turning a repaired Shiplog packet into a
review-ready evidence scaffold. The 0.8.0 Evidence Repair Loop lets a user move
from a rough first packet to a better packet:

```bash
shiplog intake --last-6-months --explain
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
```

This spec defines what must be true before Shiplog can help the user interpret
that better packet:

- packet readiness;
- evidence strength;
- receipt-backed claim candidates;
- missing-context prompts;
- share posture;
- packet quality diffs across runs.

Shiplog may produce evidence scaffolds. It must not write the final
self-review, invent accomplishments, infer impact, rate the person, or require
an LLM for the core path.

## Scope

This spec owns:

- the packet-quality fields expected in future `intake.report.json` v1 reports;
- the evidence-strength vocabulary and receipt requirements;
- the claim-candidate shape, safety rules, and missing-context prompts;
- the user-facing Packet Readiness and Claim Candidates packet sections;
- `shiplog share explain <profile> --latest` behavior;
- run quality diff behavior across compatible reports;
- compatibility and proof expectations for schema, packet, CLI, and product
  tests.

Out of scope:

- generated performance-review paragraphs;
- employee scoring, rankings, or performance ratings;
- LLM-required claim generation;
- live provider mutation, issue creation, ticket edits, or API-side repair;
- OAuth, team dashboards, manager rollups, plugin APIs, GUI, or TUI work;
- changing the crate surface;
- broad report schema redesign unrelated to packet quality;
- changing repair-loop behavior except through the surfaces named here.

## User Contract

After a user repairs and reruns a packet, the primary path is:

```bash
shiplog intake --last-6-months --explain
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog open packet --latest
shiplog share explain manager --latest
```

The packet should answer:

- what evidence is strong;
- what evidence is partial, manual-only, skipped, stale, or unavailable;
- which repair items cleared;
- which claims the evidence can support;
- what context the user still needs to add;
- whether the packet is safe for manager or public sharing.

The packet must include a Packet Readiness section near the top once the
packet-quality implementation lands:

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

When claim candidates exist, the packet must include a Claim Candidates section:

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

Claim candidates are starting points for the user. They are not final review
text, not impact claims, and not performance ratings.

## Machine Contract

Future implementation may extend `intake.report.json` v1 with an optional
top-level `packet_quality` object. Adding this optional field is
backward-compatible for v1 readers.

Absence behavior:

- older reports without `packet_quality` remain valid v1 reports;
- packet-quality commands may render a rerun or upgrade instruction from older
  reports;
- consumers must not assume `packet_quality` exists unless the schema or report
  says it does;
- packet rendering must tolerate absence by omitting Packet Readiness and Claim
  Candidates sections or by rendering a clear "not available" note.

When present, the intended shape is:

```json
{
  "packet_quality": {
    "packet_readiness": {
      "status": "ready_with_caveats",
      "summary": "Ready with caveats.",
      "reasons": [
        {
          "kind": "repair_cleared",
          "summary": "Manual evidence repair cleared.",
          "receipt_refs": [
            {
              "field": "repair_items",
              "repair_key": "manual:manual_evidence_missing"
            }
          ]
        }
      ],
      "next_actions": [
        "shiplog repair plan --latest"
      ]
    },
    "evidence_strength": [
      {
        "scope": "packet",
        "status": "partial",
        "reason": "Manual evidence is present, but GitHub and Jira are unavailable.",
        "receipt_refs": [
          {
            "field": "source_freshness",
            "source_key": "github"
          }
        ]
      }
    ],
    "claim_candidates": [
      {
        "claim_id": "claim_release_reliability",
        "title": "Release reliability",
        "supporting_repair_keys": [
          "manual:manual_evidence_missing"
        ],
        "supporting_sources": [
          "manual",
          "github"
        ],
        "evidence_strength": "partial",
        "supporting_receipt_refs": [
          {
            "field": "good"
          }
        ],
        "missing_context_prompts": [
          "What failure mode did this prevent?",
          "Who benefited?"
        ],
        "safe_profiles": [
          "manager"
        ]
      }
    ],
    "share_posture": [
      {
        "profile": "manager",
        "status": "ready_with_caveats",
        "included": [
          "workstream summaries"
        ],
        "removed": [
          "private source identifiers"
        ],
        "blocked": [],
        "next_actions": [
          "shiplog share manager --latest"
        ],
        "receipt_refs": [
          {
            "field": "share_commands"
          }
        ]
      }
    ]
  }
}
```

Required `packet_quality` children:

- `packet_readiness`;
- `evidence_strength`;
- `claim_candidates`;
- `share_posture`.

`claim_candidates` may be an empty array. `share_posture` may be empty only when
the report cannot derive any share profile posture from existing receipts.

## Receipt Inputs

Packet quality may read only existing Shiplog receipts:

- `included_sources`;
- `skipped_sources`;
- `source_decisions`;
- `source_freshness`;
- `repair_sources`;
- `repair_items`;
- `good`;
- `needs_attention`;
- `evidence_debt`;
- `top_fixups`;
- `journal_suggestions`;
- `share_commands`;
- `next_commands`;
- `actions`;
- `artifacts`;
- packet and profile artifacts already written by the run;
- share manifests already written by the run.

Run quality diff may compare two compatible reports and their already-written
artifacts. It must not re-query providers, scrape `intake.report.md`, scrape
`packet.md`, or create a second source classifier.

If a useful quality signal is not present in JSON or a durable artifact, the
implementation must either add a receipt in a schema-backed PR or mark the
signal as unavailable. It must not infer from Markdown wording.

## Packet Readiness Contract

`packet_readiness.status` describes packet usability. It is not a score for the
person whose work is being reviewed.

Initial statuses:

- `ready`: the packet has defensible claim candidates and no known blocking
  share or evidence issue;
- `ready_with_caveats`: the packet has useful claim candidates but still has
  skipped, stale, manual-only, or context-limited evidence;
- `needs_evidence`: the packet lacks enough evidence to support a claim
  candidate;
- `needs_context`: the packet has receipts but needs human explanation before
  a claim is useful;
- `blocked`: a redaction, artifact, validation, or compatibility blocker makes
  the packet unsafe or unavailable for the requested use.

Each readiness result must include:

- `summary`: short user-facing text;
- `reasons`: receipt-backed reasons;
- `next_actions`: safe user actions or an empty array.

Readiness must be deterministic from receipts. It must not use broad heuristics
such as "many commits means strong" without a receipt-backed rule in the spec or
schema docs.

## Evidence Strength Contract

`evidence_strength` is a list of scoped assessments. It may assess the packet,
a source, a workstream, a claim candidate, or a repair key.

Initial statuses:

- `strong`: source-backed receipts support the scope and no open repair item
  directly undermines it;
- `partial`: some receipts support the scope, but important context, source
  coverage, or repair evidence is incomplete;
- `manual_only`: support comes only from Shiplog-local manual evidence or local
  journal evidence;
- `source_skipped`: a configured or expected source is skipped, unavailable, or
  missing setup;
- `needs_context`: the receipts show activity, but the user must add why it
  mattered, who benefited, or what changed.

Each strength item must include:

- `scope`;
- `status`;
- `reason`;
- `receipt_refs`.

Evidence strength must not collapse to a numeric confidence score. It must not
be presented as productivity, performance, or employee quality.

## Claim Candidate Contract

A claim candidate is a receipt-backed scaffold for a possible review claim. It
must not be a complete review paragraph.

Required fields:

- `claim_id`: deterministic ID stable across reruns when the supporting receipt
  cluster is materially the same;
- `title`: short noun phrase or neutral label;
- `evidence_strength`: one of the evidence-strength statuses;
- `supporting_receipt_refs`: one or more report or artifact receipts;
- `missing_context_prompts`: zero or more user prompts;
- `safe_profiles`: share profiles where this candidate may be shown.

Optional fields:

- `supporting_repair_keys`;
- `supporting_sources`;
- `workstream_id` or workstream label, if a stable receipt exists;
- `caveats`;
- `related_artifacts`;
- `suppressed_profiles`, with reasons.

Claim candidate rules:

- no claim candidate without at least one supporting receipt;
- no invented impact, beneficiary, business outcome, or performance conclusion;
- no first-person review prose such as "I improved" unless the source receipt
  already contains that exact user-authored wording;
- no private provider opaque IDs outside internal-only contexts;
- no unsafe profile in `safe_profiles`;
- if a claim depends only on manual evidence, set `evidence_strength` to
  `manual_only` or explain why a stronger status is receipt-backed.

`missing_context_prompts` should ask for human context, not supply it. Good
prompts ask:

- what failure mode did this prevent?
- who benefited?
- what changed after this work?
- what tradeoff did the user make?
- what evidence would strengthen this claim?

Prompts must not imply an accomplishment that the receipts do not support.

## Share Posture Contract

`share_posture` explains whether a profile is safe to share and why.

Initial profiles:

- `manager`;
- `public`.

Initial statuses:

- `ready`: the profile can be prepared from current receipts without known
  blockers;
- `ready_with_caveats`: the profile can be prepared, but skipped sources,
  strict-scan caveats, manual-only evidence, or missing context should be
  reviewed;
- `blocked`: a redaction key, validation failure, missing artifact, or policy
  issue prevents safe sharing;
- `not_generated`: no share artifact exists yet, but Shiplog can explain the
  preflight posture from receipts.

Each posture item must include:

- `profile`;
- `status`;
- `included`;
- `removed`;
- `blocked`;
- `next_actions`;
- `receipt_refs`.

`shiplog share explain manager --latest` and
`shiplog share explain public --latest` read the latest compatible report and
any existing share manifest for the selected profile. They must not write share
artifacts by default.

The command output should show:

```text
Manager profile:
Included:
- workstream summaries
- evidence counts
- approved links

Removed:
- opaque provider IDs
- private source identifiers

Blocked:
- missing SHIPLOG_REDACT_KEY
```

If a redaction key is missing, the command may name `SHIPLOG_REDACT_KEY` or the
configured key environment variable. It must not print key values.

The public profile must be at least as strict as manager. If public sharing
needs a strict scan or manifest verification before it can be called ready, the
posture must say so.

## Run Quality Diff Contract

`shiplog runs diff --latest` or the final chosen command compares the latest two
compatible reports and shows how packet quality changed.

The diff must include:

- packet readiness changes;
- evidence-strength status changes;
- claim candidates added, removed, or materially changed;
- missing-context prompts added or cleared;
- share posture changes;
- repair items cleared, new, still open, or changed.

Join keys:

- use `run_id` for report identity;
- use `repair_key` for repair item movement;
- use `claim_id` for claim candidate movement;
- use `profile` for share posture movement;
- do not use display order or Markdown headings as join keys.

If fewer than two compatible reports exist, the command should explain what is
missing and show the next command that would create the needed run.

## Packet Rendering Contract

Packet rendering may display packet quality in Markdown, but Markdown must not
be the machine source of truth.

The Packet Readiness section should appear before detailed workstream or
receipt sections. It should be short enough for a time-pressured reviewer to
scan first.

The Claim Candidates section should:

- group evidence under each candidate;
- show missing-context prompts;
- show caveats when evidence is partial, manual-only, skipped, stale, or
  unavailable;
- avoid final review prose;
- preserve existing packet density controls where possible.

Internal packets may show more receipt detail than manager or public packets.
Manager and public profile packets must inherit share posture and redaction
rules from the existing share pipeline.

## Safety Contract

Packet quality inherits the report safety posture:

- no secret-bearing field names in JSON objects unless a spec records an
  exemption;
- no secret values in readiness, claim candidates, prompts, share posture, or
  commands;
- no raw provider opaque IDs outside internal-only contexts;
- no provider mutation;
- no Markdown scraping for machine behavior;
- no LLM-required path;
- no generated performance-review prose;
- no hidden report/schema changes without schema docs and compatibility tests.

The report JSON remains the machine boundary. Packet Markdown may help humans,
but it must not be the only source of a quality contract.

## Producers And Consumers

Producers:

- the intake report writer;
- future packet-quality builder logic that consumes report receipts;
- packet renderer logic for Packet Readiness and Claim Candidates sections;
- share explain logic that consumes report receipts and share manifests;
- run quality diff logic that compares compatible reports.

Consumers:

- `shiplog open packet --latest`;
- `shiplog share explain manager --latest`;
- `shiplog share explain public --latest`;
- `shiplog runs diff --latest` or the final quality diff command;
- `shiplog report validate` and schema validators;
- `shiplog report export-agent-pack`, if it later exposes a compact quality
  view;
- local UI, TUI, editor, and agent surfaces that read `intake.report.json`.

Producers and consumers must agree on absence behavior before implementation:
old reports without `packet_quality` are compatible, but quality commands may
ask the user to rerun intake to get packet-quality support.

## Acceptance Criteria

The packet-quality contract is implemented when:

- `intake.report.json` exposes optional `packet_quality` that validates against
  the schema and docs;
- packet quality derives from existing receipts and never from Markdown
  scraping or live provider queries;
- `packet_quality.packet_readiness` uses the defined status vocabulary and
  includes receipt-backed reasons;
- `packet_quality.evidence_strength` uses the defined status vocabulary and
  includes receipt-backed reasons;
- `packet_quality.claim_candidates` are deterministic, receipt-backed, and
  conservative;
- claim candidates include missing-context prompts instead of generated review
  prose;
- `packet_quality.share_posture` explains manager and public profile included,
  removed, blocked, and next-action states;
- `packet.md` renders Packet Readiness near the top;
- `packet.md` renders Claim Candidates without inventing accomplishments or
  impact;
- `shiplog share explain manager --latest` and
  `shiplog share explain public --latest` explain posture without writing
  share artifacts by default;
- the quality diff command shows readiness, evidence strength, claim candidate,
  missing-context, share posture, and repair movement between runs;
- old v1 reports without `packet_quality` remain valid and produce useful
  absence behavior;
- end-to-end product proof shows cold intake, repair plan, journal repair,
  rerun, repair diff, packet readiness improvement, claim candidate appearance,
  and share posture explanation.

## Compatibility Notes

This spec does not itself change the schema or CLI. It defines the contract
future implementation PRs must satisfy.

The expected schema change is backward-compatible:

- add optional `packet_quality` to `intake.report.json` v1;
- document absence behavior for older reports;
- keep old v1 readers valid when they ignore unknown optional fields;
- keep required existing top-level report fields unchanged.

Adding `packet_quality` requires synchronized updates to:

- `contracts/schemas/intake-report.v1.schema.json`;
- `docs/schemas/intake-report-v1.md`;
- report validation tests;
- packet rendering tests;
- CLI tests for share explain and quality diff once those commands exist.

Any future required field, type change, enum meaning change, or removal from
existing report fields requires a compatibility note and may require a schema
version change under
[`SHIPLOG-SPEC-0002`](SHIPLOG-SPEC-0002-intake-report-v1.md).

## Proof Mapping

Expected proof surfaces:

- [`contracts/schemas/intake-report.v1.schema.json`](../../contracts/schemas/intake-report.v1.schema.json)
  for optional `packet_quality` shape and secret-vocabulary inheritance.
- [`docs/schemas/intake-report-v1.md`](../schemas/intake-report-v1.md)
  for reader compatibility and absence behavior.
- [`apps/shiplog/tests/cli_integration.rs`](../../apps/shiplog/tests/cli_integration.rs)
  for report validation, share explain, quality diff, and absence behavior.
- [`apps/shiplog/tests/intake_cold_start.rs`](../../apps/shiplog/tests/intake_cold_start.rs)
  for cold first-run packet-quality signals.
- [`apps/shiplog/tests/front_door_first_pack_smoke.rs`](../../apps/shiplog/tests/front_door_first_pack_smoke.rs)
  for the end-to-end repaired-packet-to-review-ready proof.
- [`docs/guides/evidence-repair-loop.md`](../guides/evidence-repair-loop.md)
  for the repair loop this spec builds on.
- [`docs/proposals/SHIPLOG-PROP-0004-review-ready-packet-quality.md`](../proposals/SHIPLOG-PROP-0004-review-ready-packet-quality.md)
  for product intent, non-goals, and alternatives.
- Future ADR:
  `docs/adr/SHIPLOG-ADR-0007-claim-candidates-are-evidence-scaffolds.md`.
- Future implementation plan:
  `plans/packet-quality/implementation-plan.md`.

Useful validation commands for docs-only PRs:

```bash
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
cargo xtask check-executable-files --mode blocking-allowlist
git diff --check
```
