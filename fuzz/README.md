# Fuzzing

This directory is scaffolding for `cargo fuzz` (libFuzzer).

It is not part of the workspace by default.

## Why fuzz shiplog?

Two surfaces are worth punishing:

1) JSONL ingestion (`ledger.events.jsonl`)
2) YAML workstream edits (`workstreams.yaml`)

Both are user-controlled inputs. Fuzz them until the parser is boring.

## Setup

```bash
cargo install cargo-fuzz
cargo fuzz init
cargo fuzz run parse_event
```

Then wire the harness to call:

- `shiplog_ingest_json` line parser
- `serde_yaml` workstream loader
