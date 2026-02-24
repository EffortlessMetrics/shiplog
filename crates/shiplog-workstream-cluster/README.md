# shiplog-workstream-cluster

Repository-based workstream clustering strategy used across shiplog.

## Responsibilities

- `RepoClusterer`: default `WorkstreamClusterer` implementation that groups events by
  `event.repo.full_name` and builds canonical receipt/stat surfaces for each bucket.

This crate intentionally only owns event clustering policy; stateful IO and file orchestration
live in `shiplog-workstreams`.
