# shiplog-redact

Deterministic structural redaction for shiplog events and workstreams.

## Profiles

- `internal`: full fidelity.
- `manager`: keeps titles/context, strips sensitive detail fields.
- `public`: aliases repo/workstream names and strips sensitive fields/links.

## Key type

- `DeterministicRedactor`: keyed alias generation and profile projection.

Alias mappings can be persisted to `redaction.aliases.json` for stable aliases across reruns.
Deterministic alias/cache primitives are currently provided by the `shiplog-alias` implementation carrier.
Profile semantics are currently provided by the `shiplog-redaction-profile` implementation carrier.
Public repository aliasing/sanitization is currently provided by the `shiplog-redaction-repo` implementation carrier.
Policy transformation rules are currently provided by the `shiplog-redaction-policy` implementation carrier.
Profile-string projection dispatch is currently provided by the `shiplog-redaction-projector` implementation carrier.
