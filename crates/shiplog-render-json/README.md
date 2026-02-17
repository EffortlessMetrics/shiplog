# shiplog-render-json

JSON artifact writer for canonical run files.

## Functions

- `write_events_jsonl(path, events)`: writes line-delimited events.
- `write_coverage_manifest(path, coverage)`: writes pretty JSON coverage manifest.

These files are the durable machine-readable receipts for downstream workflows.
