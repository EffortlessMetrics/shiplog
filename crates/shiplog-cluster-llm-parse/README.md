# shiplog-cluster-llm-parse

Single-purpose parser for LLM clustering responses.

## Public API

- `parse_llm_response(json_str, events)`:
  Parse the LLM JSON contract into a `WorkstreamsFile`, resolve duplicate assignments safely,
  preserve event ordering, and collect unassigned events into an `Uncategorized` workstream.

## Contracts

- Invalid `event_indices` are ignored.
- Duplicate event claims are first-wins across declared workstreams.
- `receipt_indices` are honored only when they reference valid claimed events.
- A capped `Uncategorized` workstream is added for all unclaimed events.
