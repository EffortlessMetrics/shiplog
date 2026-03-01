# shiplog-workstream-layout

Single-responsibility helpers for workstream artifact resolution and persistence.

This crate owns the curated/suggested workstream workflow:

- `workstreams.yaml` (user-curated)
- `workstreams.suggested.yaml` (machine-generated)

It provides:

- Path helpers for artifact file names.
- Helpers to write and read `WorkstreamsFile` documents.
- `WorkstreamManager` precedence resolution:
  - curated workstreams first
  - suggested workstreams second
  - generate-and-write suggested when missing.
- A tiny `load_or_cluster` utility for optional preexisting YAML + fallback clustering.
