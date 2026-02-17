# shiplog-render-md

Markdown packet renderer for canonical shiplog data.

## Main type

- `MarkdownRenderer` implements `shiplog_ports::Renderer`.

## Output behavior

- Includes coverage summary, completeness, sources, and warnings.
- Renders workstream sections with claim scaffolds and receipt lists.
- Truncates long receipt lists in the main section and emits a full appendix.
- Includes artifact references (`ledger.events.jsonl`, `coverage.manifest.json`, etc.).

The output is intentionally editable: users can refine narrative text directly in `packet.md`.
