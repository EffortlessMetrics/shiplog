# shiplog-workstream-receipt-policy

Shared constants and helper predicates for policy decisions around workstream
receipt selection and rendering.

- Receipt caps are defined by event kind (`review`, `manual`, `pull-request`).
- Final cluster receipts are truncated to a hard total cap.
- Rendered main receipt list size is kept in sync across renderers.
