# SHIPLOG-PROP-0005: Guided Setup And Doctor

Status: proposed
Owner: product/docs
Created: 2026-05-17
Target release: after the paused 0.9.0 review-ready packet decision

## Summary

Shiplog's next product lane should make setup state explicit before the user
discovers problems through a failed intake, a blocked repair action, or a
caveated share step. The review-ready packet loop now has enough structure to
tell users what was found, what is missing, what changed after repair, which
claim candidates are defensible, and what is safe to share. The next missing
front-door surface is setup readiness:

```text
Your local setup is ready.
Your manual journal is valid.
Your Git source is usable.
Your provider sources are skipped for clear reasons.
Your share profile is blocked until redaction is configured.
Here is the next safe command.
```

Doctor should make setup state explicit without becoming an intake substitute.

The target user path is:

```bash
shiplog init --guided
shiplog doctor --setup
shiplog sources status
shiplog intake --last-6-months --explain
shiplog repair plan --latest
shiplog share explain manager --latest
```

This lane should make Shiplog's front door honest: the user sees source,
credential, local-file, and share-profile readiness before spending time on a
run that can only produce confusing caveats.

## Problem

The post-0.8 soak showed that many review-ready rough edges are really setup
preconditions surfacing too late:

- a malformed manual journal blocks repair actions after intake has already
  produced output;
- missing provider credentials appear as skipped-source caveats in the packet
  rather than as clear source-readiness facts before intake;
- share redaction setup can remain invisible until `share verify` or rendering
  fails closed;
- old or partial reports need richer rerun guidance instead of making commands
  infer readiness from incomplete packet-quality signals;
- copyable next actions are safest when the product knows whether they read or
  write before printing them as the first move.

The current repair and review-ready loop can recover from these states, but the
user still has to learn too much by trial. A deadline user should be able to
ask Shiplog whether setup is ready before they run intake, repair, or share.

## Target Users

Primary users:

- a first-time user who wants the smallest working local setup;
- a deadline user who needs to know whether a packet run will be useful now;
- a local-only user who intentionally has no provider tokens;
- a token-backed user who needs to know which providers are unavailable versus
  disabled by choice;
- a manager-share user who needs redaction readiness before rendering.

Secondary users:

- agents that need a machine-readable setup status before choosing whether to
  run intake, repair local files, ask for credentials, or continue local-only;
- maintainers checking that setup diagnostics do not become a second evidence
  engine;
- future OAuth or guided-configuration work that can plug into a stable source
  readiness contract.

## Product End State

The lane is done when the user can run:

```bash
shiplog doctor --setup
```

and get a compact readiness view like:

```text
Setup readiness: Needs setup

Sources:
- Local git: ready
- Manual journal: blocked - manual_events.yaml is missing version
- GitHub: unavailable - GITHUB_TOKEN not set
- GitLab: disabled
- Jira: disabled
- Linear: unavailable - LINEAR_API_KEY not set
- JSON import: ready

Share:
- Manager: blocked - SHIPLOG_REDACT_KEY missing
- Public: blocked - SHIPLOG_REDACT_KEY missing; strict scan requires a rendered public packet

Next:
1. shiplog init --guided
2. shiplog doctor --setup
3. shiplog intake --last-6-months --explain
```

The source-focused view should be available without share/profile noise:

```bash
shiplog sources status
```

Example:

```text
source_key  enabled  status       reason
git         yes      ready        repo readable
manual      yes      blocked      manual_events.yaml missing version
github      yes      unavailable  GITHUB_TOKEN not set
```

The setup loop should answer:

- which sources are enabled;
- which sources are ready;
- which sources are intentionally disabled;
- which sources are blocked by missing credentials;
- which local files are missing or malformed;
- which share profiles are renderable, blocked, or caveated;
- which command fixes each problem;
- which commands are read-only and which commands write.

## Machine End State

The exact schema belongs in the follow-up spec, but this proposal expects a
contract-backed setup readiness model instead of scattered CLI prose:

```text
setup_status:
  overall_status
  sources[]
  local_files[]
  credentials[]
  share_profiles[]
  next_actions[]
```

Each item should carry:

```text
key
label
enabled
status
reason
next_action
writes
receipt_refs
```

Suggested overall statuses:

```text
ready
ready_with_caveats
needs_setup
blocked
```

Suggested source statuses:

```text
ready
disabled
unavailable
blocked
stale_config
unknown
```

Suggested share statuses:

```text
ready
ready_with_caveats
blocked
not_generated
```

Required machine outcomes:

- setup readiness can be read without scraping Markdown;
- source keys and display labels stay canonical and non-duplicative;
- read-only and write-producing next actions are explicit;
- missing optional provider credentials do not make local-only setup fatal;
- share readiness reflects redaction/profile prerequisites without rendering
  profile artifacts;
- old report and no-report states degrade into rerun/setup guidance instead of
  misleading "ready" claims.

## Relationship To The Review-Ready Loop

Guided setup should feed the existing review-ready loop. It should not replace
intake, repair, packet quality, or share posture.

Boundary:

```text
doctor explains setup readiness.
intake produces evidence receipts.
repair consumes intake receipts.
share explain consumes report and share receipts.
```

Setup readiness is not the same signal as source evidence freshness, packet
readiness, or share posture:

- setup readiness says whether configured prerequisites are usable before a
  run;
- source evidence freshness says what intake actually collected or skipped;
- packet readiness says whether the evidence can support review work;
- share posture says whether a profile can safely explain, verify, or render
  an existing report.

Keeping those signals separate preserves the post-0.8 lesson: do not make a
finished packet look better just because a setup source disappeared, and do not
offer copyable write commands before prerequisites are known to be valid.

## Allowed Inspection

Doctor and source status may inspect:

- `shiplog.toml`;
- `manual_events.yaml`;
- local Git repository path and readability;
- environment variable presence for configured credential-backed sources;
- cached config or source metadata that already exists locally;
- latest report metadata when a command explicitly asks for latest-report
  context;
- share profile and redaction-key configuration.

Inspection should be deterministic, local, and no-network by default.

## Safety Rules

- Do not query GitHub, GitLab, Jira, Linear, or other provider APIs by default.
- Do not mutate provider state.
- Do not automatically rewrite config or manual journal files.
- Do not scrape `packet.md` as a machine source.
- Do not infer evidence health from provider token presence.
- Do not generate final performance-review prose.
- Do not leak token values, redaction keys, opaque provider IDs, or private
  source details.
- Do not treat missing optional provider tokens as fatal when local-only setup
  is otherwise usable.
- Do not make release execution part of this lane.

## Success Criteria

This lane succeeds when the following are true:

- `shiplog doctor --setup` prints a compact readiness summary for sources,
  local files, credentials, share profiles, and safe next actions;
- `shiplog sources status` prints a source-focused table with canonical
  `source_key`, display label, enabled state, status, reason, and next action;
- `shiplog init --guided` creates a valid local-first setup without pretending
  token-backed providers are ready;
- setup-blocked repair flows route users to doctor/setup before impossible
  repair commands;
- manager and public share setup readiness are visible without writing share
  artifacts;
- the setup readiness model is contract-backed and testable without network;
- old/no-report states remain understandable and prompt rerun/setup actions
  without panics or misleading readiness;
- docs teach local-only, manual-only, token-backed, manager-share, and
  public-share cautious modes.

## Non-Goals

This proposal does not include:

- OAuth implementation;
- live provider API checks by default;
- automatic config mutation;
- source adapter refactors;
- release execution;
- new public crate work;
- LLM-generated guidance;
- generated performance review prose;
- dashboards, team rollups, manager rollups, GUI, TUI, or plugin APIs;
- replacing intake, repair, packet quality, or share explanation.

## Alternatives Considered

### Keep relying on intake caveats

Rejected. Intake caveats are still necessary because they describe what happened
in a real run, but they are too late for setup problems that can be found before
the run starts.

### Make doctor perform live provider probes

Rejected for the default path. Live probes introduce latency, rate limits,
secret-handling risk, and ambiguous provider health semantics. A future explicit
probe mode may be useful, but the setup readiness contract should start with
local config, local files, environment presence, and durable receipts.

### Auto-fix setup during doctor

Rejected. Automatic config or journal mutation would make a diagnostic command
harder to trust. Guided init may write when explicitly invoked, but doctor and
source status should remain no-write surfaces.

### Fold guided setup into the paused 0.9 release

Rejected as the default lane posture. The 0.9 candidate already has review-ready
packet value and remains held for owner approval plus final preflight. Guided
setup can inform a future release decision, but it should not be used to justify
tagging or publishing 0.9 by momentum.

## Proposed Artifact Stack

Land the lane in small semantic PRs:

1. This proposal:
   `docs/proposals/SHIPLOG-PROP-0005-guided-setup-doctor.md`.
2. Setup readiness contract spec.
3. ADR: doctor is setup readiness, not intake.
4. Internal setup readiness model.
5. `shiplog doctor --setup` CLI surface.
6. `shiplog sources status` CLI surface.
7. `shiplog init --guided` defaults.
8. Repair integration for setup-blocked repairs.
9. Share profile readiness in doctor.
10. Product proof that guided setup avoids dead-end repair.
11. Guided setup and doctor guide.
12. Release decision update that keeps 0.9 paused unless explicitly approved.

The proposal explains why the lane exists. The spec defines the machine and
user-facing setup readiness contract. The ADR records the boundary that doctor
is not intake. Implementation PRs should preserve no-network, no-write doctor
behavior unless a later spec explicitly scopes otherwise.

## Proof Map

Existing proof surfaces to link from future specs and plans:

- [`docs/release/0.9.0-release-decision.md`](../release/0.9.0-release-decision.md):
  the current decision to keep the 0.9 hold active and move to guided setup as
  a non-release lane.
- [`docs/product/review-ready-dogfood-matrix.md`](../product/review-ready-dogfood-matrix.md):
  the dogfood matrix that exposed setup-adjacent states such as malformed
  manual journals, skipped providers, old reports, and share key gaps.
- [`docs/product/review-ready-loop-transcript.md`](../product/review-ready-loop-transcript.md):
  the local-history transcript proving the current repair/review-ready loop and
  fail-closed share behavior.
- [`docs/proposals/SHIPLOG-PROP-0004-review-ready-packet-quality.md`](SHIPLOG-PROP-0004-review-ready-packet-quality.md):
  the review-ready packet lane this proposal builds on.
- [`docs/specs/SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates.md`](../specs/SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates.md):
  the packet quality and claim-candidate contract that setup readiness must not
  replace.
- [`docs/adr/SHIPLOG-ADR-0007-claim-candidates-are-evidence-scaffolds.md`](../adr/SHIPLOG-ADR-0007-claim-candidates-are-evidence-scaffolds.md):
  the no-generated-review-prose boundary that doctor should preserve.
- [`docs/specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md`](../specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md):
  the repair item contract that setup-blocked repair handoffs should respect.
- [`docs/adr/SHIPLOG-ADR-0006-repair-actions-are-receipt-derived.md`](../adr/SHIPLOG-ADR-0006-repair-actions-are-receipt-derived.md):
  the receipt-derived repair boundary that guided setup should reinforce.
