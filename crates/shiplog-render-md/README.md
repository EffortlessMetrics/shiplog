# shiplog-render-md

Markdown packet renderer for canonical shiplog data.

## Main type

- `MarkdownRenderer` implements `shiplog_ports::Renderer`.

## Output behavior

- Includes coverage summary, completeness, sources, and warnings.
- Renders workstream sections with claim scaffolds and receipt lists.
- Truncates long receipt lists in the main section and emits a full appendix.
- Includes artifact references (`ledger.events.jsonl`, `coverage.manifest.json`, etc.).

`MarkdownRenderer::render_packet_markdown` keeps the default full packet behavior.
`render_scaffold_markdown` emits coverage, summary, workstream prompts, and evidence
anchors without the full receipts appendix. `render_receipts_markdown` emits a dense
receipt/audit view.

The output is intentionally editable: users can refine narrative text directly in `packet.md`.
