# shiplog-redact

Deterministic structural redaction for shiplog events and workstreams.

## Profiles

- `internal`: full fidelity.
- `manager`: keeps titles/context, strips sensitive detail fields.
- `public`: aliases repo/workstream names and strips sensitive fields, links, and source IDs.

## Key type

- `DeterministicRedactor`: keyed alias generation and profile projection.

Alias mappings can be persisted to `redaction.aliases.json` for stable aliases across reruns.

Redaction internals live as private modules inside this crate:

- `alias`: deterministic keyed aliases and alias-cache persistence.
- `profile`: canonical profile parsing.
- `repo`: public-profile repository aliasing.
- `policy`: structural event and workstream transforms.
- `projector`: profile-string dispatch into the policy layer.
