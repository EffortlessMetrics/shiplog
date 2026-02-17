# shiplog-ingest-manual

YAML manual-event ingestor for non-GitHub work.

## Main type

- `ManualIngestor`

`ManualIngestor` reads `manual_events.yaml`, filters entries to the requested date window, and converts them into canonical manual events plus a coverage manifest.

## Helper API

- `read_manual_events(path)`
- `write_manual_events(path, file)`
- `create_empty_file()`
- `create_entry(...)`

Use this when important work is not represented by GitHub artifacts.
