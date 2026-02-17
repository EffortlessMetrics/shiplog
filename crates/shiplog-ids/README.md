# shiplog-ids

Stable identifier types for shiplog domain objects.

## What it provides

- `EventId`: deterministic SHA-256 ID from source parts.
- `WorkstreamId`: deterministic SHA-256 ID for clustered work.
- `RunId`: timestamp-based run identifier helper.

## Example

```rust
use shiplog_ids::EventId;

let id = EventId::from_parts(["github", "pr", "owner/repo", "42"]);
assert_eq!(id.0.len(), 64);
```
