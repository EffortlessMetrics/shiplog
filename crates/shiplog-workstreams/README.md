# shiplog-workstreams

Workstream clustering and workstream-file lifecycle management.

## Main components

- `RepoClusterer`: default clusterer that groups events by repository.
- `WorkstreamManager`: handles curated/suggested file precedence.
- `load_or_cluster(...)`: load YAML when present, otherwise cluster.
- `write_workstreams(...)`: write `WorkstreamsFile` to YAML.

## File precedence

`WorkstreamManager::load_effective` resolves workstreams in this order:
1. `workstreams.yaml` (curated)
2. `workstreams.suggested.yaml` (generated)
3. newly clustered suggestions

This keeps user curation as the source of truth.
