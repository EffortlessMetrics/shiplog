# shiplog-ingest-json

JSON ingestion adapter for prebuilt shiplog artifacts.

## Main type

- `JsonIngestor`

It reads:
- `ledger.events.jsonl`
- `coverage.manifest.json`

and returns canonical `shiplog_ports::IngestOutput` for downstream rendering/processing.
