# Shiplog Source-of-Truth Stack

Shiplog separates repository truth by artifact role so humans and agents can
trace **why → what → how → what now → what proves it** without relying on chat
history.

## Artifact taxonomy

- `ROADMAP.md` owns release direction and major themes.
- `docs/proposals/` owns problem framing, user value, alternatives, and success criteria.
- `docs/specs/` owns required behavior, non-goals, acceptance, and proof obligations.
- `docs/adr/` owns durable architecture decisions and consequences.
- `plans/` owns PR-sized sequencing, proof commands, rollback notes, and the
  source/swarm promotion transaction.
- `.codex/goals/active.toml` owns current Codex execution state.
- `docs/status/SUPPORT_TIERS.md` owns product-claim-to-proof mapping.
- `policy/*.toml` owns enforceable ledgers and exceptions.
- `docs/governance/` owns human guides to enforced repository-governance
  boundaries, including [automation authority](governance/automation-authority.md).
- `docs/release/release-preparation.md` owns the living release-preparation,
  staging, publication, and closeout procedure.
- versioned files under `docs/release/` and root `RELEASE_HANDOFF_*` files own
  historical release decisions and execution evidence, not current commands.
- `docs/handoffs/` owns initiative closeout outcomes and residual work.

## Linking rule

Every material initiative should be linked across the stack:

`roadmap -> proposal -> spec -> ADR (if needed) -> plan -> active goal -> issue/PR -> proof and policy receipts`.

A release adds a separate bounded evidence chain:

`merged swarm candidate -> exact source promotion -> release decision -> readiness ledger -> immutable tag proof -> public-state handoff`.

Start a new release from [`docs/release/README.md`](release/README.md). Do not
copy standing instructions from a historical versioned release receipt.
