# shiplog-team-render

Team packet rendering implementation carrier for `shiplog-team`.

This crate contains rendering-focused contracts for team mode:
- `TeamAggregateResult` and `TeamMemberSummary` output models.
- `render_packet_markdown` for deterministic default markdown rendering.
- Template-driven rendering via `shiplog-template`.

Aggregation and ledger loading remain in `shiplog-team-aggregate`.
