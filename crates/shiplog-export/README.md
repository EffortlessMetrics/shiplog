# shiplog-export

Shared export contracts used across shiplog crates.

## Scope

- Export format models (`Json`, `Csv`, `Markdown`, etc.)
- Exporter trait and utility helpers
- Canonical artifact path constants and `RunArtifactPaths`
- Stable zip-path utility for profile-aware archives

## Compatibility

These APIs are intentionally small and versioned with workspace crate releases so other
microcrates can depend on them consistently.
