# Setup Readiness v1

`setup-readiness.v1` is the JSON contract printed by:

```bash
shiplog doctor --setup --json
```

The schema lives at:

```text
contracts/schemas/setup-readiness.v1.schema.json
```

Examples live under:

```text
examples/setup-readiness/blocked.json
examples/setup-readiness/needs-setup.json
examples/setup-readiness/ready-with-caveats.json
examples/setup-readiness/manager-share-blocked.json
```

The JSON is the agent-readable form of the same setup model used by the human
`shiplog doctor --setup` output. It is setup readiness, not evidence freshness,
packet readiness, repair clearance, or share posture.

## Compatibility

The v1 contract is identified by the schema path and command surface rather than
an embedded `schema_version` field.

The following top-level fields are required:

```text
overall_status
sources
local_files
credentials
share_profiles
next_actions
```

The following top-level fields are optional and must appear together when
present:

```text
requested_objective
requested_status
```

They are defined by
[`SHIPLOG-SPEC-0010-objective-scoped-setup-readiness`](../specs/SHIPLOG-SPEC-0010-objective-scoped-setup-readiness.md).
Their absence is valid for legacy v1 documents.

Future compatible changes should be additive and must update the schema,
examples, and tests together. Removing required fields, renaming stable keys, or
changing status meanings requires a new schema version or an ADR.

## Overall Status

`overall_status` is one of:

```text
ready
ready_with_caveats
needs_setup
blocked
```

It answers whether setup is usable enough to choose the next command. It must
not be interpreted as evidence quality or review readiness.

## Requested Objective

`requested_objective` is one of:

```text
intake
manager-share
public-share
all
```

`intake` is the default for `shiplog doctor --setup`. `all` preserves the
global interpretation represented by `overall_status`.

`requested_status` uses the same values as `overall_status`, but answers
whether the selected objective can proceed. For example, intake may be
`ready_with_caveats` while manager and public share profiles remain blocked by
an absent redaction key.

Older consumers should use `overall_status` when the requested fields are
absent. New producers must emit `requested_objective` and `requested_status`
as a pair; emitting only one is invalid.

## Setup Items

The `sources`, `local_files`, `credentials`, and `share_profiles` arrays contain
items with the same shape:

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

Allowed `status` values are:

```text
ready
ready_with_caveats
disabled
unavailable
blocked
stale_config
unknown
missing
malformed
optional_absent
not_generated
```

`key` is the stable join key. `label` and `reason` are display text for humans.
Use `key`, `enabled`, `status`, `next_action.writes`, and `receipt_refs` for
agent control flow.

## Next Actions

`next_actions` is ordered by priority and stable key. Each action has:

```text
key
label
command
writes
reason
priority
receipt_refs
```

The `writes` flag is part of the safety contract. Agents should prefer
read-only actions before write-producing actions and should not run a command
without explicit user approval.

## Secrets

Setup readiness JSON must not include token values, redaction key material,
passwords, or other secret values. It may include environment variable names
such as `GITHUB_TOKEN`, `LINEAR_API_KEY`, and `SHIPLOG_REDACT_KEY` so users and
agents know what setup is missing.

The schema includes `propertyNames` hygiene for secret-value field names, and
tests keep known secret sentinels out of examples and generated JSON.

## Command Behavior

`shiplog doctor --setup --json`:

- reads local config and local setup state;
- checks environment variable presence without printing values;
- does not query providers by default;
- does not mutate config;
- does not render share artifacts;
- does not scrape `packet.md`;
- prints JSON to stdout before returning a non-zero exit status for
  `needs_setup` or `blocked`.
