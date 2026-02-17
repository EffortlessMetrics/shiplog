# shiplog-redact

Deterministic structural redaction for shiplog events and workstreams.

## Profiles

- `internal`: full fidelity.
- `manager`: keeps titles/context, strips sensitive detail fields.
- `public`: aliases repo/workstream names and strips sensitive fields/links.

## Key type

- `DeterministicRedactor`: keyed alias generation and profile projection.

Alias mappings can be persisted to `redaction.aliases.json` for stable aliases across reruns.
