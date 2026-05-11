# Rapid First-Intake Review Pack — Product Definition

> **Audience:** product/engineering decisions about the cold-start path.
> **Companion docs:** [`docs/guides/review-deadline.md`](../guides/review-deadline.md)
> (how-to today), [`docs/guides/review-cycle.md`](../guides/review-cycle.md)
> (how-to for repeat cycles), [`README.md`](../../README.md) (quick start).

This document defines what the **rapid first-intake review pack** should be —
the experience a first-time shiplog user has when they need a defensible review
pack quickly and have not done any prior weekly setup. It is a product
specification, not a how-to guide. It captures:

1. [The target user and the cold-start journey](#1-target-user-and-cold-start-journey).
2. [The one-command happy path](#2-one-command-happy-path).
3. [The defaults the first run uses without asking](#3-defaults).
4. [The review-pack contents the first run produces](#4-review-pack-contents).
5. [The implementation PR ladder](#5-implementation-pr-ladder) — each row with
   an acceptance contract.
6. [Out of scope / non-goals](#6-out-of-scope).

The product promise this doc anchors:

> Useful immediately, auditable later.

`shiplog intake` already exists and already does most of the cold-start work.
This doc closes the gap between "the command works" and "a first-time user in
a hurry experiences shiplog as obviously useful."

---

## 1. Target user and cold-start journey

### Target user

Someone with:

- a review form open (self-review, promo packet, brag doc, or a manager asking
  "what did you ship?");
- scattered evidence across at least one of: GitHub, GitLab, Jira, Linear,
  local git, JSON exports, manual notes;
- **no prior shiplog configuration**: no `shiplog.toml`, no `manual_events.yaml`,
  no `out/` directory;
- **no time** to read documentation before producing a first pack;
- a willingness to come back later and curate / fix gaps once a defensible
  artifact exists.

### Cold-start journey

```text
0. user has an empty directory, a deadline, and at least one provider token
1. user runs one command
2. shiplog scaffolds local files, picks usable sources, fetches evidence
3. shiplog renders a review pack with explicit gaps, receipts, and next steps
4. user opens the pack, reads the executive summary + skipped items,
   decides whether to ship it or spend more time curating
```

The success criterion at step 4 is that the pack reads as **honest and
defensible** even with one source and gaps — not that every source succeeded.

---

## 2. One-command happy path

The cold-start command is:

```bash
shiplog intake --last-6-months --explain
```

`--explain` is recommended on first run so the terminal surfaces every
source-decision and repair hint. The command is single-shot; it does not
require prior `init` / `doctor` / `collect` / `render` steps. After it
returns, the user runs:

```bash
shiplog open intake-report --latest
shiplog open packet --latest
```

These are non-interactive; they open the durable checklist
(`intake.report.md`) and the rendered pack (`packet.md`) in the platform's
default markdown viewer. They are the recommended-first-look paths because
the report carries the readiness summary that the rendered pack alone does
not.

The acceptance test for the happy path is: from an empty directory with one
provider token set, a single invocation of `shiplog intake --last-6-months`
produces a `packet.md` plus an `intake.report.md` and exits with non-zero
status only if **zero** sources succeeded.

---

## 3. Defaults

The first-time user does not have a `shiplog.toml` yet. shiplog therefore
needs an explicit set of defaults the cold-start command applies without
asking. The defaults below are the **product contract** for the cold-start
path; they are not the same as the steady-state defaults a weekly user
relies on.

| Surface              | Default                                                                     | Why                                                                                                          |
|----------------------|------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------|
| Time window          | Six months ending today (`--last-6-months`)                                  | Matches typical review-cycle scope; users override with `--last-N-months` or a named period in `shiplog.toml`. |
| Source selection     | Every source with a usable token / local path; skipped sources recorded     | Avoids forcing the user to know what their config sections mean before running.                              |
| Redaction profile    | `internal` (no redaction)                                                    | First run is for the author. Manager / public profiles are explicit follow-up via `shiplog share`.           |
| Cache behavior       | Fresh fetch; cache populated for subsequent runs                              | First run has no cache; a stale-cache signal (PR 4) makes reuse visible on later runs.                       |
| Output location      | `./out/<run_id>/`                                                            | `<run_id>` is a sortable timestamp + short hash so reruns do not overwrite.                                  |
| Config scaffolding   | Creates a starter `shiplog.toml` and `manual_events.yaml` if missing         | Lets `intake` complete from a literal empty directory.                                                       |
| Workstream curation  | Auto-clustered by repo into `workstreams.suggested.yaml`; not promoted yet   | User-curated `workstreams.yaml` is never overwritten; suggested file is regenerable.                         |
| Exit status          | Non-zero only when zero sources succeeded                                    | A single failed source must not block a useful pack from existing.                                           |

Defaults are **inverted** as soon as `shiplog.toml` exists: enabled sources
from `[sources.*]` and the configured window override these defaults. The
contract above only applies to the first-run, no-config case.

---

## 4. Review-pack contents

The first-run output bundle must contain these six pieces. Each row lists
what the piece is, the current artifact that carries it (today vs target),
and the gap to close.

### 4.1 Executive summary

- **Today:** `packet.md` opens with a header section but does not lead with a
  human-readable executive summary; the user has to scroll to find what was
  shipped at a glance. `intake.report.md` carries a readiness summary that
  is closer to what a reviewer wants first.
- **Target:** `packet.md` opens with a 5–15 line executive summary block —
  one sentence per workstream — driven by the workstream titles and event
  counts, with explicit gaps called out inline. Cited from the workstream
  file and the coverage manifest.
- **Gap:** [PR 3 — review-pack manifest and summary polish](#pr-3--review-pack-manifest-and-summary-polish).

### 4.2 Evidence table

- **Today:** `packet.md` already carries per-workstream evidence sections
  with anchor receipts and claim prompts.
- **Target:** unchanged — the existing evidence sections already meet the
  bar. The polish PR may tighten the receipt-limit / appendix-summary
  defaults so the first run reads less like a dump.
- **Gap:** small follow-up; not blocking the cold-start path.

### 4.3 Source receipts

- **Today:** `ledger.events.jsonl` carries the full canonical event ledger
  with provider IDs and source-system tags; `coverage.manifest.json`
  carries the API query windows and pagination hits.
- **Target:** unchanged — the receipts are already complete. The
  cold-start path needs `intake.report.md` to *reference* them prominently
  so a reviewer knows where to look.
- **Gap:** small wording polish in the intake report's "where to look"
  section; rolled into PR 3.

### 4.4 Redaction summary

- **Today:** redaction is opt-in via `SHIPLOG_REDACT_KEY` plus
  `shiplog share`. Internal-profile first runs do not produce a redaction
  summary because there is no redaction.
- **Target:** `intake.report.md` carries a one-line redaction status block
  on every run (`profile: internal — no aliasing applied`,
  `profile: manager — N entities aliased`, `profile: public — N entities
  aliased`) so the reviewer can answer "is this safe to share?" without
  reading the share command's output.
- **Gap:** rolled into PR 3.

### 4.5 Skipped / omitted / uncertain items

- **Today:** `coverage.manifest.json` records skipped sources and gaps;
  `intake.report.md` carries a `Repair sources` section grouping repair
  commands by source. The framing is correct.
- **Target:** unchanged — this is the strongest piece of the existing first
  run. The polish PR makes sure the executive summary references the
  skipped section by anchor so a reviewer sees the gap before they form
  an opinion of the pack.
- **Gap:** small cross-reference polish; rolled into PR 3.

### 4.6 Reproducibility manifest

- **Today:** `bundle.manifest.json` carries SHA256 checksums for every file
  in the output bundle. The shiplog version, redaction profile, and run
  parameters are recorded but not framed as a "reproducibility receipt."
- **Target:** `bundle.manifest.json` (or a dedicated `reproducibility.json`
  sibling) carries an explicit reproducibility section: shiplog version,
  CLI invocation, source set, time window, cache hits-vs-misses, output
  hashes. A reviewer can re-run the same command on the same data and
  verify the artifact bytes match.
- **Gap:** [PR 4 — stale/cache/freshness receipt](#pr-4--stalecachefreshness-receipt)
  adds the freshness half; the rest is small framing in PR 3.

---

## 5. Implementation PR ladder

Five PRs, each tightly scoped. Each row carries an acceptance contract in
the per-PR shape this repo uses elsewhere (scope / behavior / advisory /
artifacts / validation / rollback / follow-up).

### PR 1 — fixture-backed first-intake command test

**Scope.** A workspace-level integration test that drives `shiplog intake
--last-6-months` against a fixture set in `crates/shiplog-testkit/`, asserts
the produced `intake.report.md` and `packet.md` exist, asserts the readiness
summary frames the run as "intake succeeded" or "no sources succeeded"
appropriately, asserts exit-status semantics from § 3.

**Behavior change.** None to production code. New test only.

**Advisory vs blocking.** Blocking — the test runs in the PR-fast lane.

**New artifacts.** One new test file (`apps/shiplog/tests/intake_cold_start.rs`
or similar) and fixture YAML/JSONL inputs under `crates/shiplog-testkit/`.

**Validation.** `cargo test -p shiplog`, `cargo test -p shiplog-testkit`,
the policy-checker suite, `cargo clippy -- -D warnings`.

**Rollback.** Single revert removes the test; no production change to undo.

**Follow-up.** PR 2 builds on the same fixture set.

### PR 2 — default config / source-selection behavior

**Scope.** Tighten the cold-start defaults documented in § 3 so they match
what the test in PR 1 asserts. Concretely: ensure `shiplog intake` from an
empty directory produces a starter `shiplog.toml` with every source listed
but disabled except those with a usable token / path; ensure the time
window resolves to "six months ending today" without requiring
`--last-6-months`; ensure the exit status is non-zero only when zero
sources succeeded.

**Behavior change.** Small. The exit-status and default-window
behavior may already match; the source-selection scaffold may need a
tweak. Driven by what PR 1's test reveals.

**Advisory vs blocking.** Blocking.

**New artifacts.** None (small in-source change).

**Validation.** `cargo test --workspace`, `cargo clippy -- -D warnings`,
the integration test from PR 1 must still pass.

**Rollback.** Single revert. The previous defaults are the prior commit.

**Follow-up.** PR 3 polishes the rendered output the user reads.

### PR 3 — review-pack manifest and summary polish

**Scope.** The pieces called out in § 4.1, § 4.3, § 4.4, § 4.5 as "rolled
into PR 3":

- Executive summary block at the top of `packet.md` (one sentence per
  workstream, gaps inlined, ≤ 15 lines).
- Redaction status block in `intake.report.md` even on internal-profile
  runs.
- Cross-reference from the executive summary to the skipped-items section
  (so a reviewer sees the gap before forming an opinion).
- "Where to look" section in `intake.report.md` linking to
  `ledger.events.jsonl`, `coverage.manifest.json`, and
  `bundle.manifest.json` with one-line summaries.

**Behavior change.** Rendering only. No new data is collected; only the
shape of the rendered output changes.

**Advisory vs blocking.** Blocking — affects every render output.

**New artifacts.** None.

**Validation.** `cargo test -p shiplog-render-md` (snapshot diffs are
intentional; reviewers verify the new shape is correct);
`cargo clippy -- -D warnings`.

**Rollback.** Single revert. Snapshot baseline reverts with it.

**Follow-up.** PR 4 adds the cache-freshness piece.

### PR 4 — stale/cache/freshness receipt

**Scope.** Capture cache hit/miss counts and source freshness in
`bundle.manifest.json` (or a sibling) so a reviewer can answer "is this
data fresh?" without reading the cache implementation. Surface a one-line
freshness signal in `intake.report.md`'s readiness summary
(e.g. `Cache: 87% hits, oldest entry 2 days old`).

**Behavior change.** Small. The cache layer already tracks the data; this
PR adds the receipt emission and the report-side display.

**Advisory vs blocking.** Blocking.

**New artifacts.** New `cache.receipt.json` or extension of
`bundle.manifest.json`. Schema doc under `docs/schemas/`.

**Validation.** `cargo test --workspace`, snapshot tests under
`shiplog-render-md`, the integration test from PR 1 (now asserting the
freshness line in the report).

**Rollback.** Single revert. The cache layer continues to track the
counts internally; only the emission goes away.

**Follow-up.** PR 5 adds docs examples and verifies installation works.

### PR 5 — docs examples and install smoke

**Scope.** Two parts:

1. Add a worked example to `docs/product/` (alongside this doc) showing
   the cold-start journey end to end against the fixture set from PR 1.
   The example should be runnable from a fresh checkout.
2. Extend the existing release install smoke (referenced from the release
   runbook) to cover `shiplog intake` from a literal empty directory in
   the install-smoke harness.

**Behavior change.** None to production code. Docs + test-harness changes.

**Advisory vs blocking.** Blocking — the install smoke gates release.

**New artifacts.** One new example doc + one extension to the install
smoke shell/PowerShell.

**Validation.** `bash scripts/install-smoke.sh` (or equivalent), the
fixture-driven example doc reproduces the documented output.

**Rollback.** Single revert.

**Follow-up.** None on this lane. The next product lane (warm weekly
collection, review-pack polish, etc.) is a separate ladder.

---

## 6. Out of scope

The following are intentionally **not** part of the rapid first-intake
lane:

- Generated narrative or impact claims. shiplog produces evidence anchors
  and claim prompts; the user writes the narrative.
- Multi-cycle comparison. `shiplog runs compare` already exists for repeat
  cycles; the cold-start path is single-run.
- Cross-user / team aggregation. shiplog is single-user by default; team
  rollups are a separate product lane.
- Provider-specific dashboards or live polling. shiplog is a one-shot
  command that produces a static bundle.
- Any change to the protected-fields ladder (cache-internals, bundle-paths,
  redaction-internals, trust-receipts, source-opaque-ids,
  policy-ledger-metadata). That lane proceeds independently per its own
  ladder in [`docs/CLIPPY_PROTECTED_FIELDS.md`](../CLIPPY_PROTECTED_FIELDS.md).
- Workflow changes, release work, crates.io publish, or new policy
  ledgers. The five-PR ladder above is source/docs/test work only.

---

## See also

- [`README.md`](../../README.md) — top-level quick start and feature list.
- [`docs/guides/review-deadline.md`](../guides/review-deadline.md) — how-to
  for deadline-driven use today.
- [`docs/guides/review-cycle.md`](../guides/review-cycle.md) — how-to for
  repeat review cycles today.
- [`docs/config-reference.md`](../config-reference.md) — full
  `shiplog.toml` field reference.
- [`docs/schemas/intake-report-v1.md`](../schemas/intake-report-v1.md) —
  intake report JSON contract.
- [`docs/schemas/agent-pack-v1.md`](../schemas/agent-pack-v1.md) — agent
  pack export contract.
- [`docs/install.md`](../install.md) — installation paths and checksum
  verification.
