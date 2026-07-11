# SHIPLOG-SPEC-0007: Setup Readiness

Status: proposed
Owner: product/schema
Created: 2026-05-17
Related proposal:
[`SHIPLOG-PROP-0005-guided-setup-doctor`](../proposals/SHIPLOG-PROP-0005-guided-setup-doctor.md)
Related report spec:
[`SHIPLOG-SPEC-0002-intake-report-v1`](SHIPLOG-SPEC-0002-intake-report-v1.md)
Related repair spec:
[`SHIPLOG-SPEC-0005-evidence-repair-loop`](SHIPLOG-SPEC-0005-evidence-repair-loop.md)
Related packet-quality spec:
[`SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates`](SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates.md)
Related objective-scoped extension:
[`SHIPLOG-SPEC-0010-objective-scoped-setup-readiness`](SHIPLOG-SPEC-0010-objective-scoped-setup-readiness.md)

## Purpose

This spec defines the setup readiness contract for guided setup and doctor
surfaces. The contract lets Shiplog tell a user whether local setup is ready
before they discover problems through intake caveats, blocked repair actions,
or share failures.

The intended front-door path is:

```bash
shiplog init --guided
shiplog doctor --setup
shiplog sources status
shiplog intake --last-6-months --explain
shiplog repair plan --latest
shiplog share explain manager --latest
```

Setup readiness is a prerequisite signal. It is not evidence collection, packet
quality, repair completion, or share rendering.

## Scope

This spec owns:

- the setup readiness status vocabulary;
- the machine-readable setup status model;
- source setup readiness for configured and autodetected sources;
- local file readiness for config, manual journal, Git path, and latest-report
  metadata when explicitly requested;
- credential presence checks that never expose secret values;
- share profile setup readiness for manager and public profiles;
- read-only versus write-producing next-action metadata;
- compatibility rules for old reports and existing `doctor` behavior;
- proof expectations for future implementation PRs.

Out of scope:

- OAuth implementation;
- live provider API probing by default;
- automatic config or manual-journal mutation from doctor;
- provider-side mutation;
- source adapter refactors;
- release execution;
- generated performance-review prose;
- dashboards, GUI, TUI, team rollups, manager rollups, or plugin APIs.

## Signal Boundary

Setup readiness must stay separate from related product signals:

| Signal | Owner | Answers | Must not imply |
| --- | --- | --- | --- |
| Setup readiness | doctor/setup | Are prerequisites usable before a run? | Evidence was collected. |
| Source freshness | intake report | What did intake collect, cache, skip, or find unavailable? | Setup is globally ready. |
| Repair readiness | repair plan | What can be safely repaired from intake receipts? | Provider setup is fixed. |
| Packet readiness | packet quality | Can current evidence support review work? | Setup was complete. |
| Share posture | share explain | Can an existing report/profile be explained, verified, or rendered safely? | Rendering already happened. |

Boundary:

```text
doctor explains setup readiness.
intake produces evidence receipts.
repair consumes intake receipts.
share explain consumes report and share receipts.
```

Doctor and source status must not scrape `packet.md`, inspect Markdown headings
as machine data, or infer evidence quality from token presence.

## User Contract

`shiplog doctor --setup` prints a compact readiness summary for the whole local
setup.

Example:

```text
Setup readiness: Needs setup

Blocked:
- Manual journal: manual_events.yaml is missing version
- Manager share: SHIPLOG_REDACT_KEY missing

Ready:
- Local git
- JSON import

Unavailable:
- GitHub: GITHUB_TOKEN not set

Next:
1. repair manual_events.yaml
2. set SHIPLOG_REDACT_KEY
3. shiplog intake --last-6-months --explain
```

`shiplog sources status` prints the source-only view without share profile
noise.

Example:

```text
source_key  enabled  status       reason
git         yes      ready        repo readable
manual      yes      blocked      manual_events.yaml missing version
github      yes      unavailable  GITHUB_TOKEN not set
```

`shiplog sources status --json` emits the same source-scoped projection for
agents and scripts: a `needs_action` flag (the read-only exit signal), the
`sources[]` rows, and the deduplicated source `next_actions[]`. It is derived
from the same `setup_status` model as `doctor --setup --json`, scoped to
sources, so the JSON and text views cannot drift.

Both commands should keep read-only and write-producing actions visibly
separate. Doctor and source status are read-only by default. `init --guided`
and repair commands may write only when the user invokes them explicitly.

## Machine Contract

The internal model is `setup_status`. Future JSON output or schemas should use
this shape unless a later spec replaces it.

```text
setup_status:
  overall_status
  sources[]
  local_files[]
  credentials[]
  share_profiles[]
  next_actions[]
```

`sources status --json` is the source-scoped projection of that model:

```text
sources_status:
  needs_action
  sources[]
  next_actions[]
```

Required fields for every readiness item:

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

Required fields for top-level `next_actions[]`:

```text
key
label
command
writes
reason
priority
receipt_refs
```

Field rules:

- `key` is a stable machine key, not a display label.
- `label` is user-facing and may change for clarity.
- `enabled` describes whether the source, file check, credential requirement,
  or share profile is active in the current setup.
- `status` uses the vocabulary defined below.
- `reason` is short, human-readable, and safe to print.
- `next_action` may be empty only when no safe action exists.
- `writes` is true only when running the action can create, modify, or delete
  local files, profile artifacts, provider state, or release artifacts.
- `receipt_refs` point to local config, local file, environment-presence,
  latest-report, or share-profile checks; they must not contain secret values.
- `priority` sorts next actions from safest first move to later optional work.

The model may be rendered as human text first. If a future `--json` surface is
added, it must preserve this contract and add schema validation in the same PR.

## Status Vocabulary

### Overall Status

- `ready`: configured setup can run the next recommended intake/share path
  without known setup blockers.
- `ready_with_caveats`: setup can run, but optional sources, old reports,
  missing share artifacts, or local-only limits should be understood first.
- `needs_setup`: one or more setup tasks should be handled before the best
  path, but Shiplog can still provide useful guidance.
- `blocked`: no safe next run or requested share path exists until a setup
  blocker is fixed.

### Source Status

- `ready`: the source prerequisites are locally usable.
- `disabled`: the source is intentionally off.
- `unavailable`: an enabled or autodetected source is missing an external
  or local prerequisite such as an environment variable or configured path.
- `blocked`: a local setup defect prevents safe source use, such as malformed
  manual journal syntax.
- `stale_config`: config exists and can be read, but uses incompatible,
  deprecated, or migrated source settings. Missing configured paths are
  `unavailable`, not `stale_config`.
- `unknown`: Shiplog cannot determine readiness from allowed local checks.

### Local File Status

- `ready`: the file exists and validates for the requested role.
- `missing`: the file is absent and required or recommended for the current
  setup.
- `malformed`: the file exists but cannot be parsed or fails schema validation.
- `optional_absent`: the file is absent but not required for the current mode.
- `stale_config`: the file uses old or incompatible configuration.
- `unknown`: readiness cannot be determined from local checks.

### Credential Status

- `ready`: the expected environment variable or configured credential reference
  is present.
- `disabled`: the source or profile that would require the credential is
  disabled.
- `unavailable`: the credential reference is missing.
- `blocked`: the credential setup is internally inconsistent, such as a config
  requiring a named env var that is empty or invalidly named.
- `unknown`: Shiplog cannot determine presence without reading a secret value
  or making a network request.

Credential readiness may name environment variable names such as
`GITHUB_TOKEN`, `LINEAR_API_KEY`, or `SHIPLOG_REDACT_KEY`. It must not print,
hash, store, or compare secret values.

### Share Status

- `ready`: the profile prerequisites are present for the requested operation.
- `ready_with_caveats`: the profile can be explained or prepared, but missing
  evidence, strict-review caveats, or non-generated artifacts should be
  reviewed.
- `blocked`: redaction key, profile, verification, or policy prerequisites are
  missing.
- `not_generated`: no profile artifact exists yet, but setup can still explain
  what would be needed.

## Allowed Inputs

Doctor and source status may inspect:

- `shiplog.toml`;
- `manual_events.yaml`;
- local Git repository path and readability;
- environment variable presence for configured credential-backed sources;
- configured source enablement and disablement;
- cached config or source metadata that already exists locally;
- latest report metadata only when the command explicitly asks for latest-report
  context;
- share profile configuration and redaction-key presence.

Doctor and source status must not:

- call provider APIs by default;
- mutate provider state;
- write local files unless the invoked command is explicitly a writer such as
  `shiplog init --guided`;
- render manager or public share artifacts;
- scrape `intake.report.md` or `packet.md` as a machine source;
- infer evidence health from setup state.

## Source Readiness Rules

Initial canonical source keys:

```text
git
manual
github
gitlab
jira
linear
json
unknown
```

Rules:

- local Git is `ready` only when the configured or current directory is a Git
  worktree and basic repository metadata is readable;
- manual is `disabled` when config turns manual evidence off, and disabled
  manual evidence does not require the journal file to exist or validate;
- enabled manual evidence is `ready` only when the configured journal file
  exists and validates;
- enabled manual evidence is `unavailable` when the configured journal file is
  missing;
- enabled manual evidence is `blocked` when the configured journal exists but
  is malformed;
- manual may also report `missing` or `malformed` through `local_files[]`; the
  source status still follows the enabled/disabled rules above;
- token-backed providers are `unavailable` when enabled but the required env var
  is absent;
- token-backed providers are `disabled` when config turns them off;
- autodetected but unconfigured providers may be shown as `unavailable` or
  `disabled` only when the reason is explicit;
- JSON import is `ready` only when its configured file or directory path exists
  and is readable.

Missing optional provider credentials should not make a local-only setup
`blocked`. They may make the overall status `ready_with_caveats` or
`needs_setup` depending on whether the user asked for token-backed coverage.

## Share Readiness Rules

Initial profile keys:

```text
manager
public
```

Rules:

- manager share is `blocked` when `SHIPLOG_REDACT_KEY` or the configured
  redaction-key env var is absent, even when manager is not the default
  profile, because manager rendering remains available after setup;
- public share is `blocked` when the redaction key is absent;
- public share is `ready_with_caveats` or `not_generated` when a strict scan or
  rendered public packet is required before calling the profile ready;
- share readiness must not render profile artifacts;
- share readiness must stay consistent with `share explain`, but it may be less
  specific when no compatible latest report exists.

Doctor may recommend:

```text
shiplog share explain manager --latest
shiplog share verify manager --latest
shiplog share manager --latest
```

only in an order that keeps read-only commands before write-producing render
commands.

## Next Action Rules

Next actions are part of the machine contract because they determine whether
the front door is safe.

Rules:

- prefer read-first actions before write actions;
- do not offer `journal add --from-repair` when the manual journal is malformed;
- do not offer provider repair completion when only credential setup is missing;
- do not offer share rendering before share explanation or verification when
  setup is caveated;
- include `writes: true` for `init --guided`, `journal add`, share rendering,
  config mutation, and any future provider mutation;
- include `writes: false` for `doctor --setup`, `sources status`, `repair plan`,
  `repair diff`, `runs diff`, `open --print-path`, `share explain`, and
  validation-only commands.

When no safe action exists, the item should say so with a reason instead of
printing a copyable command that cannot succeed.

## Compatibility Notes

The global setup model remains the compatibility baseline. Objective-scoped
requested results and the `doctor --setup --for <objective>` command contract
are defined by
[`SHIPLOG-SPEC-0010-objective-scoped-setup-readiness`](SHIPLOG-SPEC-0010-objective-scoped-setup-readiness.md).
That extension is additive and does not change the meaning of
`overall_status`.

This spec does not itself change the current intake report schema, existing
`shiplog doctor` behavior, or CLI output. It defines the contract future PRs
must satisfy.

Compatibility requirements:

- preserve existing `shiplog doctor` and `doctor --repair-plan` behavior unless
  a later spec explicitly migrates it;
- add `doctor --setup` without making existing doctor invocations write files;
- add `sources status` as a read-only source readiness view;
- keep reports without `packet_quality`, `repair_items`, or share posture valid
  and route users to rerun/setup guidance instead of panics;
- do not embed setup readiness into `intake.report.json` v1 without a schema
  update, compatibility note, and report-validation tests;
- if setup readiness gets a JSON schema, include explicit secret-field rules for
  credential metadata without allowing secret values.

## Producers And Consumers

Producers:

- guided init defaults;
- setup readiness builder;
- config loader and validator;
- manual journal validator;
- local Git readiness checker;
- source configuration readers;
- environment-presence checker;
- share profile readiness checker;
- latest-report metadata reader when explicitly requested.

Consumers:

- `shiplog doctor --setup`;
- `shiplog sources status`;
- `shiplog repair plan --latest` for setup-blocked repair handoffs;
- `shiplog share explain <profile> --latest` for consistent share setup
  wording;
- guided setup docs and examples;
- future agent surfaces that choose whether to run intake, repair local files,
  ask for credentials, disable optional providers, or continue local-only.

## Acceptance Criteria

The setup readiness contract is implemented when:

- `shiplog doctor --setup` renders setup readiness from a typed model;
- `shiplog sources status` renders the source subset from the same model;
- the typed model includes sources, local files, credentials, share profiles,
  and next actions;
- source and share statuses use the vocabulary in this spec;
- every item includes a stable key, label, enabled state, status, reason,
  next-action metadata, write posture, and receipt references;
- provider checks are local and no-network by default;
- doctor and source status are no-write by default;
- manual journal malformed/missing states are distinguished;
- missing optional provider tokens do not make local-only setup fatal;
- share redaction readiness is visible without rendering profile artifacts;
- setup-blocked repair handoffs route to doctor/setup before impossible repair
  commands;
- old/no-report states degrade gracefully and prompt rerun/setup actions;
- tests prove ready, ready-with-caveats, needs-setup, blocked, disabled,
  unavailable, malformed, no-write, and read-first next-action cases.

## Proof Mapping

Expected proof surfaces:

- [`docs/proposals/SHIPLOG-PROP-0005-guided-setup-doctor.md`](../proposals/SHIPLOG-PROP-0005-guided-setup-doctor.md)
  for product intent and non-goals.
- [`docs/adr/SHIPLOG-ADR-0008-doctor-is-setup-readiness-not-intake.md`](../adr/SHIPLOG-ADR-0008-doctor-is-setup-readiness-not-intake.md)
  for the durable decision that doctor is setup readiness, not intake.
- Future setup readiness model tests under `apps/shiplog/tests/` or the
  implementation module test tree.
- Future CLI tests for `doctor --setup` and `sources status` no-write behavior.
- Future repair tests proving setup-blocked items route to doctor before
  impossible journal or provider repair commands.
- Future share tests proving doctor reports manager/public redaction readiness
  without rendering profile artifacts.
- [`docs/specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md`](SHIPLOG-SPEC-0005-evidence-repair-loop.md)
  for the repair handoff contract.
- [`docs/specs/SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates.md`](SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates.md)
  for packet readiness and share posture boundaries.
- [`docs/release/0.9.0-release-decision.md`](../release/0.9.0-release-decision.md)
  for the current release hold decision that makes this a non-release lane.

Useful validation commands for docs-only PRs:

```bash
cargo fmt --all -- --check
git diff --check
cargo test -p shiplog --test docs_commands
```
