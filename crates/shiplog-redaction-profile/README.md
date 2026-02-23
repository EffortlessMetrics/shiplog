# shiplog-redaction-profile

Profile semantics for shiplog redaction projections.

This crate isolates only profile concerns:

- `RedactionProfile` variants: `internal`, `manager`, `public`
- parsing from profile strings with stable fallback behavior
- canonical string rendering

Policy transforms remain in `shiplog-redaction-policy`.
