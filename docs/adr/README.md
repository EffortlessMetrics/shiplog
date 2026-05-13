# Shiplog ADRs

Architecture Decision Records capture durable decisions that future agents and
maintainers must preserve unless a later ADR replaces them.

Use an ADR when the decision changes how shiplog assigns responsibility across
components, how receipts are trusted, or how user-visible evidence is allowed
to be inferred.

## Naming

Use:

```text
SHIPLOG-ADR-0001-short-slug.md
```

Keep ADR numbers stable. A later decision should supersede an ADR explicitly
instead of editing history into a different decision.

## Required Shape

Each ADR should include:

- status;
- context;
- decision;
- consequences;
- alternatives considered;
- affected specs, plans, tests, or schemas.

## Boundaries

ADRs are not product guides or work plans. They record durable architecture
choices such as:

- which layer owns a receipt boundary;
- whether JSON uses machine keys or display labels;
- when shiplog may emit a freshness state;
- which component is allowed to infer or merge evidence.

When an ADR affects behavior, link the relevant spec. When it affects rollout,
link the relevant plan. When it affects proof, link tests, schemas, or policy
ledgers.
