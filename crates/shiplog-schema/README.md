# shiplog-schema

Canonical data model shared by shiplog crates.

This is internal workspace support for the 0.7 crate-surface contraction. The
public machine contract is the JSON schema set under
[`contracts/schemas/`](../../contracts/schemas/).

## Modules

- `event`: `EventEnvelope`, payload variants (`PullRequest`, `Review`, `Manual`), source metadata.
- `coverage`: coverage windows, slices, and manifest completeness.
- `workstream`: workstream structure, stats, and receipt references.
- `bundle`: bundle manifest/checksum types and `BundleProfile`.

All ingestion, clustering, rendering, and redaction crates depend on these
serialized contracts.
