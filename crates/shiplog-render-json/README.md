# shiplog-render-json

JSON artifact writers for canonical shiplog outputs.

## Functions

- `write_events_jsonl(path, events)`
- `write_coverage_manifest(path, coverage)`

`write_events_jsonl` emits one event per line (`ledger.events.jsonl`), and `write_coverage_manifest` writes pretty JSON (`coverage.manifest.json`).
