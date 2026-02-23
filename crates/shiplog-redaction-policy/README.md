# shiplog-redaction-policy

Profile-based structural redaction rules for shiplog events and workstreams.

This crate isolates policy from alias/cache persistence and adapter wiring.
Profile parsing/canonicalization lives in `shiplog-redaction-profile` and is
re-exported here for compatibility. Public repository redaction and alias
resolver contracts live in `shiplog-redaction-repo` and are re-exported here.

## API

- `RedactionProfile`: `internal`, `manager`, `public`
- `AliasResolver`: alias abstraction used by public profile
- `redact_event_with_aliases` / `redact_events_with_aliases`
- `redact_workstream_with_aliases` / `redact_workstreams_with_aliases`

`shiplog-redact` composes this crate with `shiplog-alias` for deterministic
alias generation and cache persistence.
