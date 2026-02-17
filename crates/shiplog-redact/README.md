# shiplog-redact

Deterministic redaction engine for shiplog outputs.

## Profiles

- `internal`: full fidelity.
- `manager`: context preserved, sensitive detail reduced.
- `public`: aggressively anonymized for external sharing.

## Key type

- `DeterministicRedactor`: keyed alias generation and optional alias cache persistence (`redaction.aliases.json`).
