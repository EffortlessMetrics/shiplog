# Support tiers

This file maps product and governance claims to proof commands.

## Tier definitions

| Tier | Meaning |
|---|---|
| Stable | User-facing claim is supported by required proof. |
| Stabilizing | Works for documented paths but is still maturing. |
| Experimental | Available for exploration; broad claims are not allowed. |
| Advisory | Informational only and not branch-protection blocking. |
| Not supported | Explicitly outside current support claims. |

## Claim map

| Surface | Tier | Claim | Proof command | Notes |
|---|---|---|---|---|
| Document artifact links | Stabilizing | Proposal/spec/ADR/plan artifacts are linked by policy ledger. | `cargo xtask check-doc-artifacts` | Dedicated checker validates files, IDs, kinds, statuses, and linked artifact refs. |
| Package boundary | Stable | Published crate surface is policy-ledger controlled. | `cargo xtask check-package-boundary` | Guardrail against durable unpublished production crates. |
| Policy ledgers | Stable | Policy files parse and are schema-audited. | `cargo xtask check-policy-schemas` | Requires ledger updates when claims change. |
