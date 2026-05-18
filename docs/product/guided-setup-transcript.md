# Guided setup dogfood transcript

> **Status:** setup-readiness operationalization receipt for the paused 0.9
> candidate.
> **Release posture:** this transcript does not approve tagging, publishing,
> GitHub release execution, release workflow dispatch, or release-install smoke.
> **Date:** 2026-05-18.

This records one real temporary-workspace run through the Guided Setup / Doctor
front door into the existing repair and share-explain loop. It is proof, not a
release decision.

## Setup

The run used a temporary empty workspace under
`target/codex-guided-setup-transcript-418/workspace/` with:

- no pre-existing `shiplog.toml`;
- no `.git` directory;
- no provider tokens;
- no `SHIPLOG_REDACT_KEY`;
- a local debug build of `shiplog`.

The output directory was:

```bash
target/codex-guided-setup-transcript-418/out
```

## Transcript

### 1. Guided init

```bash
shiplog init --guided
```

Observed:

- Exit status: success.
- Wrote local setup files: `shiplog.toml` and `manual_events.yaml`.
- Printed the front-door handoff including `shiplog doctor --setup`.
- Did not create the intake output directory.

### 2. Doctor

```bash
shiplog doctor --setup
```

Observed:

- Exit status: non-zero because setup still needed share redaction.
- `Setup readiness: Needs setup`.
- Manual journal was ready.
- Token-backed providers were disabled, not treated as failed evidence.
- Manager and public share were blocked by missing `SHIPLOG_REDACT_KEY`.
- No intake output directory was created.

### 3. Doctor JSON

```bash
shiplog doctor --setup --json
```

Observed:

- Exit status: non-zero because the same setup blocker remained.
- Stdout contained the machine-readable setup model.
- `overall_status`: `needs_setup`.
- Sources:
  - `manual`: enabled, `ready`;
  - `github`, `gitlab`, `jira`, `linear`, `git`, `json`: disabled.
- Share profiles:
  - `manager`: `blocked`;
  - `public`: `blocked`.
- Top-level next action:
  - `set SHIPLOG_REDACT_KEY`, `writes = false`.
- No secret values were printed.
- No intake or share artifacts were written.

This is the agent-control-plane behavior: an agent can parse stdout, see the
non-zero exit status, avoid share rendering, and still continue local-only
intake because manual evidence setup is valid.

### 4. Sources status

```bash
shiplog sources status
```

Observed:

- Exit status: success.
- Manual source was ready.
- Optional provider and local-git sources were disabled.
- Output did not include manager/public share rows.
- Output did not include `SHIPLOG_REDACT_KEY` or redaction setup noise.
- No intake output directory was created.

### 5. First intake

```bash
shiplog intake --out target/codex-guided-setup-transcript-418/out --last-6-months --explain --no-open
```

Run: `merge_1779084526661013000`.

Observed:

- Exit status: success.
- Included sources: `manual: 0`.
- Packet readiness: `needs_evidence`.
- The report exposed a safe journal repair item:
  `repair_001_manual_manual_evidence_missing_no_events`.

### 6. Repair plan

```bash
shiplog repair plan --out target/codex-guided-setup-transcript-418/out --latest
```

Observed:

- Exit status: success.
- The plan offered a copyable
  `shiplog journal add --from-repair repair_001_manual_manual_evidence_missing_no_events`
  command.
- This repair appeared after setup was valid enough for local manual repair,
  not during doctor/source-status setup reads.

### 7. Journal repair

```bash
shiplog journal add --from-repair repair_001_manual_manual_evidence_missing_no_events --out target/codex-guided-setup-transcript-418/out --latest
```

Observed:

- Exit status: success.
- Wrote one local manual event to `manual_events.yaml`.
- Printed the rerun intake command.

### 8. Rerun intake

```bash
shiplog intake --out target/codex-guided-setup-transcript-418/out --last-6-months --explain --no-open
```

Run: `merge_1779084527041907800`.

Observed:

- Exit status: success.
- Included sources: `manual: 1`.
- Packet readiness moved to `ready_with_caveats`.

### 9. Repair diff

```bash
shiplog repair diff --out target/codex-guided-setup-transcript-418/out --latest
```

Observed:

- Exit status: success.
- Cleared: 1.
- The cleared item was the manual no-events repair.

### 10. Runs diff

```bash
shiplog runs diff --out target/codex-guided-setup-transcript-418/out --latest
```

Observed:

- Exit status: success.
- Manual evidence movement was visible.
- The diff showed improvement without claiming provider evidence was collected.

### 11. Share explain

```bash
shiplog share explain manager --out target/codex-guided-setup-transcript-418/out --latest
```

Observed:

- Exit status: success.
- Status remained `blocked` because `SHIPLOG_REDACT_KEY` was missing.
- The command explained share posture without rendering.
- `profiles/manager/packet.md` was not written.

## What Looked Right

- `init --guided` was the only setup command in the transcript that wrote files.
- `doctor --setup`, `doctor --setup --json`, and `sources status` stayed
  read-only and created no output directory.
- Doctor JSON exposed the same setup state an agent needs without scraping
  terminal prose.
- Disabled providers stayed disabled instead of becoming weak evidence claims.
- The repair command appeared only after intake had a valid local manual source
  and a report-derived repair item.
- Rerun plus `repair diff` proved the manual repair cleared.
- `runs diff` showed manual evidence movement.
- `share explain manager` stayed read-only and did not render manager artifacts.

## Remaining Caveats

- This transcript used the empty-directory/manual-first path. It did not prove a
  token-backed provider setup.
- A preliminary Git fixture without an origin remote showed a local-git caveat:
  doctor can prove the repository path is readable, while intake may still need
  repository identity to collect Git evidence. That belongs in follow-up
  wording or compatibility hardening, not this transcript PR.
- Manager/public rendering remains blocked until a stable redaction key exists.
- Public strict verification was not exercised here; it remains covered by the
  share-readiness tests and review-ready matrix.

## Intentionally Not Fixed

- Did not set provider tokens.
- Did not render manager or public packets.
- Did not add OAuth, provider probing, dashboards, plugins, TUI, or new
  adapters.
- Did not generate review prose.
- Did not change release posture.

## Release Decision Signal

This transcript supports the setup-readiness lane by showing the front door can
start from an empty directory, diagnose setup, produce agent-readable state, run
manual-only intake, repair missing local evidence, prove the repair, and explain
share posture without rendering.

It does not approve `v0.9.0` release execution. A release decision still needs
current matrix review, compatibility hardening, front-door docs alignment, green
main CI, current hold receipts, and explicit owner approval.

## Cleanup

The temporary dogfood workspace under
`target/codex-guided-setup-transcript-418/` was removed after this transcript was
recorded.
