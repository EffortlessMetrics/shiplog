# shiplog-redaction-projector

Profile-string projection dispatch for shiplog redaction.

This crate isolates one job:

- Parse raw profile strings (`internal`, `manager`, `public`)
- Dispatch to structural policy transforms in `shiplog-redaction-policy`

It does not own alias persistence or keyed alias generation.
Those concerns remain in `shiplog-alias` and `shiplog-redact`.

## API

- `parse_profile`
- `project_events_with_aliases`
- `project_workstreams_with_aliases`
- Re-exports: `AliasResolver`, `RedactionProfile`
