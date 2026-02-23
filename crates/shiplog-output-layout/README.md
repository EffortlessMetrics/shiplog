# shiplog-output-layout

Shared contracts for shiplog run artifacts.

## Scope

This crate owns the canonical artifact filenames and path helpers used across
shiplog crates:

- `packet.md`
- `ledger.events.jsonl`
- `coverage.manifest.json`
- `bundle.manifest.json`
- `profiles/{internal,manager,public}` conventions
- Profile zip naming (`run_dir.zip` and `run_dir.<profile>.zip`)

## Stability

Keep these contracts as small, additive changes only. They are intended to be a
stable interop boundary between microcrates.
