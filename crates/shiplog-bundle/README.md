# shiplog-bundle

Bundle manifest and zip generation for run directories.

## Functions

- `write_bundle_manifest(out_dir, run_id, profile)`
- `write_zip(out_dir, zip_path, profile)`

## Profile scoping

- `internal`: includes full internal artifact set.
- `manager`: includes `profiles/manager/packet.md` and `coverage.manifest.json`.
- `public`: includes `profiles/public/packet.md` and `coverage.manifest.json`.

`redaction.aliases.json` is always excluded to avoid alias reverse-mapping leaks.
