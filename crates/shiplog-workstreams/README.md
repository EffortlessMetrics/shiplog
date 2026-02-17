# shiplog-workstreams

Workstream clustering and file management.

## What it does

- Clusters events by repository via `RepoClusterer`.
- Supports `workstreams.suggested.yaml` + curated `workstreams.yaml` workflow.
- Provides load/write helpers and `WorkstreamManager` safeguards.

## Primary entry points

- `RepoClusterer`
- `load_or_cluster(...)`
- `write_workstreams(...)`
- `WorkstreamManager`
