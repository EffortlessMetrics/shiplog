# Shiplog Specs

Specs define behavior contracts between product intent and agent execution.
They should make shiplog's first-run review-pack experience executable:
obvious output, source receipts, freshness posture, safe sharing, repair
guidance, and proof-backed next commands.

Specs should not become user guides or PR plans. A spec says what must remain
true; the implementation plan says how to land it safely.

## Naming

Use:

```text
SHIPLOG-SPEC-0001-short-slug.md
```

Keep the numeric identifier stable once referenced by tests, plans, issues, or
goal manifests.

## Required Shape

Each spec should include:

- status and owner;
- scope and non-goals;
- user-visible contract;
- machine-readable contract when schemas or receipts are involved;
- acceptance criteria;
- proof mapping to tests, schemas, policy ledgers, or docs;
- compatibility and migration notes when the contract changes an artifact.

## Boundaries

Specs define contracts. Plans define PR order. Guides teach users. Policy
ledgers record exceptions, proof, or enforcement posture.

In practice:

- Put CLI behavior, receipt shape, schema versioning, source identity, cache
  semantics, and redaction contracts in specs.
- Put branch ordering, rollback, and proof commands in plans.
- Put "run this command, then read this file" instructions in guides.
- Put allowlists, skipped-by-policy receipts, and enforcement exceptions in
  policy ledgers.

## Proof

Every spec needs at least one proof surface. Good proof surfaces include:

- integration tests under `apps/shiplog/tests/`;
- JSON schemas under `contracts/schemas/`;
- schema documentation under `docs/schemas/`;
- policy ledgers under `policy/`;
- product or guide docs that intentionally explain the user path.

The proof mapping should link to existing surfaces when possible instead of
duplicating their content.
