# SHIPLOG-SPEC-0008: Review Loop Status

Status: proposed
Owner: product/schema
Created: 2026-05-18
Related proposal:
[`SHIPLOG-PROP-0006-review-loop-status`](../proposals/SHIPLOG-PROP-0006-review-loop-status.md)
Related setup spec:
[`SHIPLOG-SPEC-0007-setup-readiness`](SHIPLOG-SPEC-0007-setup-readiness.md)
Related report spec:
[`SHIPLOG-SPEC-0002-intake-report-v1`](SHIPLOG-SPEC-0002-intake-report-v1.md)
Related repair spec:
[`SHIPLOG-SPEC-0005-evidence-repair-loop`](SHIPLOG-SPEC-0005-evidence-repair-loop.md)
Related packet-quality spec:
[`SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates`](SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates.md)

## Purpose

This spec defines the review-loop status contract for `shiplog status --latest`
and `shiplog status --latest --json`.

Status is a read-only projection over existing setup, evidence, repair, diff,
and share receipts. It should tell humans and agents where the review loop is,
what is blocking progress, and which command is safe next.

The intended recurring path is:

```bash
shiplog status --latest
shiplog intake --last-6-months --explain
shiplog status --latest
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog runs diff --latest
shiplog share explain manager --latest
shiplog status --latest
```

Status should act like a front panel, not a dashboard. It must not replace the
underlying command-specific receipts.

## Scope

This spec owns:

- the review-loop status vocabulary;
- the machine-readable review-loop status model;
- summary sections for setup, latest run, packet readiness, source readiness,
  repair, diff, and share posture;
- blocking reasons that explain why the next stage is not safe;
- deterministic next actions with read-only/write labels;
- receipt references back to durable local machine sources;
- compatibility behavior for missing, old, or malformed receipts;
- proof expectations for future implementation PRs.

Out of scope:

- OAuth implementation;
- live provider probing;
- implicit intake reruns;
- automatic repair;
- automatic share rendering;
- dashboards, GUI, TUI, team rollups, manager rollups, or plugin APIs;
- LLM summaries or generated performance-review prose;
- new source adapters;
- public crate changes;
- release execution.

## Signal Boundary

Review-loop status must keep the existing product signals separate:

```text
setup readiness != evidence quality
evidence quality != repair readiness
repair readiness != share readiness
share explain != share render
status != packet prose
```

| Signal | Owner | Answers | Status may summarize | Must not imply |
| --- | --- | --- | --- | --- |
| Setup readiness | doctor/setup | Are prerequisites usable before a run? | Yes, from the setup model. | Evidence was collected. |
| Source freshness | intake report | What did intake collect, cache, skip, or find unavailable? | Yes, from report receipts. | Provider setup is globally ready. |
| Packet readiness | packet quality | Can current evidence support review work? | Yes, from `packet_quality`. | Review prose is written. |
| Repair readiness | repair plan | What evidence repair actions are safe? | Yes, from repair items and plan receipts. | Setup blockers are fixed. |
| Repair movement | repair diff | What changed after rerun? | Yes, from repair diff receipts when present. | A missing old item was cleared without evidence. |
| Run movement | runs diff | What packet-quality movement happened across runs? | Yes, from runs diff receipts when present. | The packet is ready to share. |
| Share posture | share explain/verify/render | Can an existing profile be explained, verified, or rendered safely? | Yes, from share readiness receipts. | Rendering already happened. |

Boundary:

```text
doctor explains setup readiness.
intake produces evidence receipts.
repair consumes intake receipts.
diff compares durable run receipts.
share explain consumes report and share receipts.
status summarizes those receipts and chooses a safe next action.
```

Status must not scrape `packet.md`, inspect Markdown headings as machine data,
infer evidence quality from token presence, or infer share safety from prose.

## User Contract

`shiplog status --latest` prints a compact review-loop status summary.

Example:

```text
Review loop status: Needs repair

Setup:
  ready with caveats

Latest run:
  run_id: 2026-05-18T170000Z
  packet readiness: needs_evidence
  included sources: manual 0, git 4
  unavailable sources: GitHub token missing

Repair:
  open items: 2
  safe writes: 1
  setup-blocked writes: 0

Diff:
  no prior comparable run

Share:
  manager: blocked - SHIPLOG_REDACT_KEY missing
  public: blocked - public review not yet possible

Next:
1. shiplog repair plan --latest [read-only] - inspect repair actions
2. shiplog journal add --from-repair <repair_id> [writes] - add local evidence
3. shiplog intake --last-6-months --explain [writes] - rerun after repair

Receipts:
- setup: doctor model
- latest_run: out/2026-05-18T170000Z/intake.report.json
```

The human surface should:

- name the overall review-loop state;
- show which latest run or receipt was read;
- show setup blockers before evidence repair actions;
- show packet readiness from report JSON, not packet Markdown;
- summarize source availability separately from evidence quality;
- summarize open repair work without replacing `repair plan`;
- summarize diff availability without replacing `repair diff` or `runs diff`;
- summarize share blockers without replacing `share explain`;
- label read-only and write-producing next actions;
- avoid offering write commands that cannot succeed in the current state.

## Machine Contract

The internal model is `review_loop_status`. Future JSON output and schemas
should use this shape unless a later spec replaces it.

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

Required fields for `next_actions[]`:

```text
key
label
command
writes
reason
preconditions[]
priority
receipt_refs[]
```

Required fields for `blocking_reasons[]`:

```text
key
label
status
reason
scope
receipt_refs[]
```

Required fields for `receipt_refs[]`:

```text
field
kind
path
key
```

Field rules:

- `overall_status` uses the vocabulary defined below.
- Section summaries may be empty or `unknown` when the receipt is missing.
- `key` is a stable machine key, not a display label.
- `label` is user-facing and may change for clarity.
- `command` is copyable when present.
- `writes` is true only when running the command can create, modify, or delete
  local files, share artifacts, provider state, or release artifacts.
- `preconditions[]` names machine-checkable requirements that should already
  be true before the command is run.
- `priority` sorts next actions from safest first move to later optional work.
- `receipt_refs[]` point to local config, setup model, report JSON, repair
  receipts, diff receipts, share manifests, or share readiness receipts.
- Receipt refs must not contain secret values, provider token values, private
  provider IDs, or generated review prose.

The JSON form:

```bash
shiplog status --latest --json
```

must preserve this contract with stable keys, snake_case statuses,
deterministic arrays, no secrets, no provider calls, no writes, and no Markdown
scraping.

## Status Vocabulary

The overall status vocabulary is finite:

| Status | Triggering receipt condition | First safe action |
| --- | --- | --- |
| `unknown` | Required receipts are absent, unreadable, or too malformed to classify. | Run `shiplog doctor --setup` or inspect the named receipt. |
| `needs_setup` | Setup model reports blocked, malformed, missing, or stale required setup. | Run `shiplog doctor --setup` or `shiplog sources status`. |
| `ready_to_collect` | Setup is usable and no latest intake run exists. | Run `shiplog intake --last-6-months --explain`. |
| `needs_evidence` | Latest report exists but packet readiness or evidence strength indicates insufficient evidence before share. | Run `shiplog repair plan --latest`. |
| `needs_repair` | Latest report has open repair items with at least one safe repair action. | Run `shiplog repair plan --latest`. |
| `repair_in_progress` | A repair was applied or newer manual evidence exists, but no comparable rerun/diff receipt proves movement yet. | Rerun intake, then run `shiplog repair diff --latest`. |
| `ready_with_caveats` | Packet is usable with explicit caveats such as unavailable optional sources, manual-only evidence, or missing context prompts. | Run `shiplog share explain manager --latest` before rendering. |
| `ready_to_explain_share` | Packet is sufficiently ready for share posture explanation, but profile verification/rendering is not yet proven. | Run `shiplog share explain <profile> --latest`. |
| `share_blocked` | Share explain, verify, or profile readiness receipts report a blocker such as missing `SHIPLOG_REDACT_KEY` or failed strict public checks. | Fix share setup or keep using internal packet only. |
| `ready_to_share` | Share explain and verify receipts show the requested profile is renderable under current prerequisites. | Run the explicit share render command if the user chooses to write artifacts. |
| `blocked` | A non-recoverable or ambiguous local state prevents safe next action selection. | Inspect the named blocking receipt. |

Status selection should be deterministic. If multiple conditions apply, choose
the earliest unsafe stage in this order:

```text
blocked
needs_setup
ready_to_collect
unknown
needs_evidence
needs_repair
repair_in_progress
share_blocked
ready_to_explain_share
ready_with_caveats
ready_to_share
```

The follow-up implementation may refine precedence in the spec only if tests
pin the new order.

## Section Contracts

### `setup_summary`

Summarizes the setup readiness model from `SHIPLOG-SPEC-0007`.

Required behavior:

- use the same setup model as `doctor --setup`;
- keep source setup separate from evidence quality;
- report setup blockers before repair or share next actions;
- never query providers;
- never mutate config.

### `latest_run`

Summarizes latest run discovery.

Required behavior:

- respect explicit `--out` and `--run` once those flags exist;
- make `--latest` deterministic;
- name the run ID and report path when present;
- classify no output directory or empty output directory as missing run state,
  not as packet failure;
- treat malformed JSON as `unknown` or `blocked` with the path named.

### `packet_readiness`

Summarizes `intake.report.json` packet quality fields.

Required behavior:

- read `packet_quality.packet_readiness` when present;
- gracefully degrade old reports without `packet_quality`;
- prompt rerun for richer signals instead of inventing readiness;
- never scrape `packet.md`.

### `source_summary`

Summarizes included, skipped, unavailable, and disabled source state.

Required behavior:

- distinguish setup unavailable from weak evidence;
- keep provider token gaps as setup/source availability facts;
- never print secret values;
- preserve canonical source keys.

### `repair_summary`

Summarizes repair items and safe repair actions.

Required behavior:

- count open repair items from report receipts;
- count write-producing repair actions only when setup prerequisites are valid;
- suppress dead-end `journal add --from-repair` commands while manual setup is
  malformed or blocked;
- prefer read-first `repair plan` when repair details need inspection.

### `diff_summary`

Summarizes repair and run-diff availability.

Required behavior:

- report no prior comparable run clearly;
- read repair diff receipts when present;
- read runs diff receipts when present;
- avoid claiming repair clearance without matching evidence in the newer run.

### `share_summary`

Summarizes share explain, verify, render, and manifest posture.

Required behavior:

- prefer read-only `share explain` before render commands;
- fail closed when redaction key, profile, or strict public checks are missing;
- never render share artifacts;
- never offer manager/public render commands while share is blocked;
- read existing share manifests only when already present.

## Compatibility

Status must degrade cleanly for existing users:

- no config: `needs_setup` with `init --guided` or `doctor --setup` guidance;
- old config: use setup-readiness compatibility rules;
- no output directory: `ready_to_collect` when setup allows intake;
- empty output directory: `ready_to_collect`;
- old report without `packet_quality`: report is valid, but packet readiness is
  unknown and rerun guidance should be visible;
- old report without `repair_items`: repair summary is unknown or zero with
  rerun guidance, not a fake "no repair needed" claim;
- old report without share posture: share summary should prompt rerun or
  `share explain`, not claim ready;
- malformed JSON report: status names the malformed receipt and avoids repair
  or share write commands.

## Safety Rules

- No provider probing.
- No config mutation.
- No implicit intake rerun.
- No automatic repair.
- No share rendering.
- No packet Markdown scraping.
- No LLM calls.
- No generated performance-review prose.
- No secret values in text or JSON output.
- No release execution.

## Acceptance Criteria

Future implementation PRs must prove:

- `shiplog status --latest` is read-only;
- `shiplog status --latest --json` is read-only;
- JSON uses stable keys and snake_case statuses;
- arrays are deterministic;
- setup blockers match `doctor --setup --json`;
- source blockers match `sources status`;
- packet readiness matches `intake.report.json`;
- repair counts and repair action write labels match `repair plan`;
- repair clearance matches `repair diff`;
- packet-quality movement matches `runs diff`;
- share blockers match `share explain` and fail-closed share commands;
- missing setup and missing runs produce safe next actions;
- setup-blocked states never offer evidence-repair write commands;
- share-blocked states never offer profile render commands;
- old reports degrade with rerun/setup guidance;
- no status path reads `packet.md` as machine truth.

## Proof Map

Existing proof surfaces:

- [`SHIPLOG-PROP-0006-review-loop-status`](../proposals/SHIPLOG-PROP-0006-review-loop-status.md):
  product lane and non-goal framing.
- [`SHIPLOG-SPEC-0007-setup-readiness`](SHIPLOG-SPEC-0007-setup-readiness.md):
  setup model that status must summarize rather than replace.
- [`docs/schemas/setup-readiness-v1.md`](../schemas/setup-readiness-v1.md):
  machine-readable setup contract.
- [`SHIPLOG-SPEC-0002-intake-report-v1`](SHIPLOG-SPEC-0002-intake-report-v1.md):
  report receipt contract for source, repair, packet-quality, and share data.
- [`SHIPLOG-SPEC-0005-evidence-repair-loop`](SHIPLOG-SPEC-0005-evidence-repair-loop.md):
  repair item and repair action contract.
- [`SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates`](SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates.md):
  packet readiness, evidence strength, claim candidate, and share posture
  contract.
- [`docs/product/guided-setup-transcript.md`](../product/guided-setup-transcript.md):
  setup front-door dogfood proof.
- [`docs/product/review-ready-loop-transcript.md`](../product/review-ready-loop-transcript.md):
  intake, repair, diff, and share explain dogfood proof.

Required future proof surfaces:

- model tests for missing setup, no run, setup-blocked run, repairable run, and
  share-blocked run;
- CLI no-write tests for human and JSON status;
- schema and examples for `review-loop-status.v1`;
- consistency tests against doctor, sources status, intake report, repair plan,
  repair diff, runs diff, and share explain;
- dogfood transcript showing status before intake, after intake, after repair,
  after rerun/diff, and before share rendering.

## Release Posture

This spec does not approve a release. `v0.9.0` remains prepared but held until
owner approval and current release preflight exist. Status work may become part
of an unreleased candidate only after implementation and proof PRs land; this
spec alone does not authorize tag, crates.io publish, GitHub release creation,
release workflow dispatch, or release-install smoke.
