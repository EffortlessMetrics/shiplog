# shiplog-ids

Deterministic identifier types used across the shiplog workspace.

## Provides

- `EventId::from_parts(...)`
- `WorkstreamId::from_parts(...)`
- `RunId::now(prefix)`

`EventId` and `WorkstreamId` are SHA-256 hashes derived from stable input parts. `RunId` is a timestamp-based run identifier for output directories.

## Example

```rust
use shiplog_ids::EventId;

let id = EventId::from_parts(["github", "pr", "owner/repo", "42"]);
assert_eq!(id.0.len(), 64);
```
