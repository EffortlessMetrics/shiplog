# shiplog-render-md

Markdown packet renderer.

`MarkdownRenderer` implements `shiplog_ports::Renderer` and produces an editable self-review packet with:

- coverage summary
- workstream sections
- receipt lists
- appendix receipts

The renderer is intentionally low-magic so users can edit output directly.
