# shiplog-schema

Canonical data model for the shiplog pipeline.

All adapters and the engine depend on this crate for shared types.

## Modules

- `event`: event envelope and payload types.
- `workstream`: workstream structures and stats.
- `coverage`: coverage manifests and query slices.
- `bundle`: bundle profile and manifest types.

Use this crate as the single source of truth for serialized artifacts.
