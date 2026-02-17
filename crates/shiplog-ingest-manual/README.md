# shiplog-ingest-manual

Manual-event ingestion from YAML.

`ManualIngestor` converts `manual_events.yaml` entries into canonical shiplog events and coverage output.

## Helpers

- `read_manual_events(path)`
- `write_manual_events(path, file)`
- `create_empty_file()`
- `create_entry(...)`

Use this adapter to capture work that does not appear in GitHub activity.
