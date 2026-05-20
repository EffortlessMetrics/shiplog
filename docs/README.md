# Shiplog Source-of-Truth Stack

Shiplog separates repository truth by artifact role so humans and agents can
trace **why → what → how → what now → what proves it** without relying on chat
history.

## Artifact taxonomy

- `ROADMAP.md` owns release direction and major themes.
- `docs/proposals/` owns problem framing, user value, alternatives, and success criteria.
- `docs/specs/` owns required behavior, non-goals, acceptance, and proof obligations.
- `docs/adr/` owns durable architecture decisions and consequences.
- `plans/` owns PR-sized sequencing, proof commands, and rollback notes.
- `.codex/goals/active.toml` owns current Codex execution state.
- `docs/status/SUPPORT_TIERS.md` owns product-claim-to-proof mapping.
- `policy/*.toml` owns enforceable ledgers and exceptions.
- `docs/handoffs/` owns closeout outcomes and residual work.

## Linking rule

Every material initiative should be linked across the stack:

`roadmap -> proposal -> spec -> ADR (if needed) -> plan -> active goal -> issue/PR -> proof and policy receipts`.
