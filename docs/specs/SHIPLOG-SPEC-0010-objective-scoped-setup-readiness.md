# SHIPLOG-SPEC-0010: Objective-Scoped Setup Readiness

Status: accepted
Owner: product/schema
Created: 2026-07-11
Related setup contract:
[`SHIPLOG-SPEC-0007-setup-readiness`](SHIPLOG-SPEC-0007-setup-readiness.md)
Related architecture decision:
[`SHIPLOG-ADR-0008-doctor-is-setup-readiness-not-intake`](../adr/SHIPLOG-ADR-0008-doctor-is-setup-readiness-not-intake.md)

## Purpose

The setup model inspects several independent surfaces: sources, local files,
credentials, and share profiles. A single global result is useful for agents
that need the complete picture, but it makes an intake user fail on a later
share prerequisite such as a missing redaction key.

This spec adds an objective-scoped result without changing the existing global
setup contract. The command answers both questions:

```text
overall_status    What is true across every inspected setup surface?
requested_status  Can the requested task proceed now?
```

## Scope

This spec owns:

- the `intake`, `manager-share`, `public-share`, and `all` objectives;
- the default objective for `doctor --setup`;
- the relationship between `overall_status` and `requested_status`;
- exit-code and human-headline selection;
- additive JSON compatibility for `requested_objective` and
  `requested_status`;
- objective-aware readiness proof and review-loop consumption.

Out of scope:

- changing the existing `overall_status` vocabulary;
- changing setup item status vocabulary;
- provider API calls or OAuth;
- GitHub credential discovery;
- config mutation or automatic repair;
- share rendering or redaction policy;
- changing `setup-readiness.v1` into a breaking schema revision.

## Command Contract

The supported command forms are:

```bash
shiplog doctor --setup
shiplog doctor --setup --for intake
shiplog doctor --setup --for manager-share
shiplog doctor --setup --for public-share
shiplog doctor --setup --for all
```

Omitting `--for` means `intake`. `--for all` preserves the old global
interpretation and is the compatibility escape hatch for callers that want all
inspected surfaces to participate in the result.

The command remains read-only, does not query providers by default, does not
render share artifacts, and does not print secret values.

## Objective Semantics

`requested_status` uses the existing `overall_status` vocabulary:

```text
ready
ready_with_caveats
needs_setup
blocked
```

The requested objective is evaluated as follows:

| Objective | Requested result includes | Must not block intake |
| --- | --- | --- |
| `intake` | Setup required to collect local and configured evidence | Manager/public share profile blockers |
| `manager-share` | Setup required to explain, verify, or render manager output | Public-only blockers |
| `public-share` | Setup required to explain, verify, or render public output | Manager-only blockers |
| `all` | Every inspected surface | Nothing excluded |

For `intake`:

- a usable local/manual setup returns `ready` or `ready_with_caveats` even
  when `SHIPLOG_REDACT_KEY` is absent;
- missing optional provider credentials are caveats when local intake can
  still proceed;
- malformed required local configuration remains `blocked`;
- source, credential, and share rows remain visible so the user can see later
  work without treating it as an intake blocker.

For `manager-share` and `public-share`, the relevant redaction and profile
prerequisites remain fail-closed. Missing redaction setup is a non-zero
blocker for the requested share objective.

For `all`, `requested_status` equals `overall_status`.

## JSON Compatibility

The existing `setup-readiness.v1` top-level fields remain required and retain
their meanings. Two optional fields are added:

```json
{
  "requested_objective": "intake",
  "requested_status": "ready_with_caveats"
}
```

The fields are an all-or-none pair:

- both fields absent is a valid legacy v1 document;
- both fields present identify the objective and its requested result;
- only one field present is invalid;
- `requested_status` uses the same enum as `overall_status`;
- `overall_status` continues to describe every inspected surface;
- `requested_status` answers whether the selected task can proceed.

Older consumers may continue reading `overall_status`. New consumers should
prefer `requested_status` when `requested_objective` is present and should
fall back to `overall_status` for legacy documents.

The command headline and exit code use `requested_status`:

- `ready` and `ready_with_caveats` return success;
- `needs_setup` and `blocked` return non-zero after printing the result;
- JSON is printed before the non-zero exit, preserving the existing behavior.

## Review-Loop Integration

`shiplog status` represents the intake workflow. Its setup projection and
pre-run decision must consume the `intake` requested result rather than
failing because a later share profile is blocked. It may still display share
profile caveats and must preserve share fail-closed behavior for share commands.

`--for all` remains available for diagnostics, support, and automation that
needs the global result.

## Safety And Compatibility

- No objective may cause doctor to contact a provider by default.
- No objective may cause doctor to write local files or provider state.
- No objective may serialize token or redaction-key material.
- Existing setup item keys, statuses, next-action write flags, and receipt refs
  remain stable.
- Schema, examples, docs, and contract tests change together.
- Implementation PRs must prove both legacy documents without requested fields
  and new documents containing the paired fields.

## Acceptance Criteria

- The four objectives and default `intake` behavior are documented.
- `overall_status` remains global and backward compatible.
- `requested_objective` and `requested_status` are additive and paired.
- Intake readiness ignores manager/public blockers while preserving visible
  caveats.
- Manager and public readiness remain fail-closed.
- Exit-code and JSON ordering rules are explicit.
- Review-loop status is specified to consume the intake objective.
- Schema, representative fixtures, and docs contract tests prove the additive
  shape and objective enum.

## Proof Mapping

- [`docs/schemas/setup-readiness-v1.md`](../schemas/setup-readiness-v1.md)
  documents the machine contract.
- [`contracts/schemas/setup-readiness.v1.schema.json`](../../contracts/schemas/setup-readiness.v1.schema.json)
  constrains the additive fields and pair semantics.
- [`examples/setup-readiness/`](../../examples/setup-readiness/) provides legacy
  and objective-scoped fixtures.
- `apps/shiplog/tests/docs_commands.rs` proves schema, fixture, and docs
  alignment.
- PR 2 must add implementation and CLI behavior proof.
- PR 3 must add objective independence, exit-code, no-write, and secret-hygiene
  product proof.
