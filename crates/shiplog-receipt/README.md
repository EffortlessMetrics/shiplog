# shiplog-receipt

Single-responsibility receipt formatting for shiplog.

This crate owns one contract:

- format canonical `EventEnvelope` values into human-readable Markdown receipt lines

It is intentionally narrow and side-effect free so other renderers can reuse the
same receipt presentation logic.
