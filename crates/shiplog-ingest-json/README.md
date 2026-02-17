# shiplog-ingest-json

Ingestor for prebuilt JSON artifacts.

`JsonIngestor` reads:

- `ledger.events.jsonl`
- `coverage.manifest.json`

and returns them as `shiplog_ports::IngestOutput`.

This is useful for tests, fixture-driven workflows, and re-rendering without API access.
