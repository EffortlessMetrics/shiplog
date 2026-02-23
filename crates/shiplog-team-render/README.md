# shiplog-team-render

Team packet rendering microcrate extracted from `shiplog-team-aggregate`.

This crate contains rendering-focused contracts for team mode:
- `TeamAggregateResult` and `TeamMemberSummary` output models.
- `render_packet_markdown` for deterministic default markdown rendering.
- Template-driven rendering via `shiplog-template`.

Aggregation and ledger loading remain in `shiplog-team-aggregate`.
