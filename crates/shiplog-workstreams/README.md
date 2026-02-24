# shiplog-workstreams

Workstream clustering and workstream-file lifecycle management.

## Main components

- `RepoClusterer`: default clusterer that groups events by repository
  (re-exported from `shiplog_workstream-cluster` to keep clustering SRP isolated).
- `WorkstreamManager`: handles curated/suggested file precedence.
- `load_or_cluster(...)`: load YAML when present, otherwise cluster.
- `write_workstreams(...)`: write `WorkstreamsFile` to YAML.

File precedence and artifact path contracts are owned by
`shiplog-workstream-layout` and re-exported here for compatibility.
