# Review Loop Status v1

`review-loop-status.v1` is the JSON contract printed by:

```bash
shiplog status --latest --json
```

The schema lives at:

```text
contracts/schemas/review-loop-status.v1.schema.json
```

Examples live under:

```text
examples/review-loop-status/missing-setup.json
examples/review-loop-status/ready-to-collect.json
examples/review-loop-status/needs-evidence.json
examples/review-loop-status/needs-repair.json
examples/review-loop-status/repair-in-progress.json
examples/review-loop-status/ready-with-caveats.json
examples/review-loop-status/share-blocked.json
examples/review-loop-status/ready-to-share.json
examples/review-loop-status/unknown-old-report.json
examples/review-loop-status/malformed-report.json
```

The JSON is the agent-readable form of the same review-loop status model used
by the human `shiplog status --latest` output. It is setup/evidence/repair/diff
and share state, not packet prose.

## Compatibility

The v1 contract is identified by the schema path and command surface rather than
an embedded `schema_version` field.

The following top-level fields are required:

```text
overall_status
setup_summary
latest_run
packet_readiness
source_summary
repair_summary
diff_summary
share_summary
blocking_reasons
next_actions
receipt_refs
```

Future compatible changes should be additive and must update the schema,
examples, and tests together. Removing required fields, renaming stable keys, or
changing status meanings requires a new schema version or an ADR.

## Overall Status

`overall_status` is one of:

```text
unknown
needs_setup
ready_to_collect
needs_evidence
needs_repair
repair_in_progress
ready_with_caveats
ready_to_explain_share
share_blocked
ready_to_share
blocked
```

It answers where the review loop is and what stage owns the next safe action.
It must not be interpreted as performance quality, generated review prose, or a
release decision.

## Sections

`setup_summary` mirrors the typed setup readiness model used by
`doctor --setup --json`. Its status values are:

```text
ready
ready_with_caveats
needs_setup
blocked
unknown
```

`latest_run` is either `null` or names the resolved run ID, report path, and
receipt references.

`packet_readiness` summarizes report JSON packet-quality receipts. Its status
values are:

```text
ready
ready_with_caveats
needs_evidence
needs_repair
unknown
```

`source_summary` contains included source counts, unavailable source reasons,
disabled source reasons, and receipt references. Source availability is not the
same signal as evidence quality.

`repair_summary` contains open repair counts, safe write counts,
setup-blocked write counts, repair-applied-not-rerun state, and receipt
references.

`diff_summary` reports whether comparable repair or run-diff receipts are
available. Its status values are:

```text
available
no_prior_comparable_run
not_generated
unknown
```

`share_summary` contains profile posture. Profile status values are:

```text
ready
ready_with_caveats
blocked
not_generated
unknown
```

## Next Actions

`next_actions` is ordered by priority and stable key. Each action has:

```text
key
label
command
writes
reason
preconditions
priority
receipt_refs
```

The `writes` flag is part of the safety contract. Agents should prefer
read-only actions before write-producing actions and should not run a command
without explicit user approval.

`preconditions` is an array of machine-readable requirements that should already
be true before the command is run. It may be empty when the action is a
read-only inspection step.

## Blocking Reasons

`blocking_reasons` names the stage that prevents a later command from being
safe. Each reason has:

```text
key
label
status
reason
scope
receipt_refs
```

Common scopes include `setup`, `packet`, `repair`, `share`, and `receipt`.

## Receipt References

`receipt_refs` point to durable local machine sources:

```text
field
kind
path
key
```

`path` and `key` may be `null` when the receipt is a typed model field rather
than a file or item key.

Receipt references may name local paths, environment variable names, source
keys, repair keys, or profile keys. They must not include token values,
redaction key material, passwords, private provider payloads, or generated
review prose.

## Secrets

Review-loop status JSON must not include token values, redaction key material,
passwords, or other secret values. It may include environment variable names
such as `GITHUB_TOKEN`, `LINEAR_API_KEY`, and `SHIPLOG_REDACT_KEY` so users and
agents know what setup is missing.

The schema includes `propertyNames` hygiene for secret-value field names, and
tests keep known secret sentinels out of examples and generated JSON.

## Command Behavior

`shiplog status --latest --json`:

- reads typed setup state and durable review-loop receipts;
- summarizes setup readiness, latest run, packet readiness, source state,
  repair state, diff state, and share state;
- does not query providers;
- does not mutate config;
- does not run intake implicitly;
- does not run repair commands;
- does not render share artifacts;
- does not scrape `packet.md`;
- does not call an LLM;
- does not generate performance-review prose;
- does not execute release work.

The JSON is a control-plane surface. It should help humans and agents choose the
next safe receipt-producing command, not replace the underlying command-specific
receipts.
