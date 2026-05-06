# shiplog-workstreams

Workstream clustering and workstream-file lifecycle management.

## Main components

- `RepoClusterer`: default clusterer that groups events by repository
  (implemented as `shiplog_workstreams::cluster`).
- `WorkstreamManager`: handles curated/suggested file precedence.
- `load_or_cluster(...)`: load YAML when present, otherwise cluster.
- `write_workstreams(...)`: write `WorkstreamsFile` to YAML.

File precedence and artifact path contracts are owned by
`shiplog_workstreams::layout`; receipt caps are owned by
`shiplog_workstreams::receipt_policy`.
