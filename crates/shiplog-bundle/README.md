# shiplog-bundle

Bundle and manifest generation for run outputs.

## Functions

- `write_bundle_manifest(out_dir, run_id, profile)`
- `write_zip(out_dir, zip_path, profile)`

## Notes

- Computes SHA-256 checksums for included files.
- Applies profile-scoped inclusion (`internal`, `manager`, `public`).
- Excludes sensitive/internal bookkeeping files such as `redaction.aliases.json`.
