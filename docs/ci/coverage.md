# Coverage

Codecov coverage is execution-surface evidence.

It answers:

> Did tests execute this Rust surface?

It does not answer:

- whether GitHub/GitLab/Jira/Linear source ingestion is complete,
- whether the shiplog coverage manifest is complete,
- whether packets are review-ready,
- whether redaction is safe for a given audience,
- whether LLM clustering is semantically correct,
- whether mutation adequacy is strong,
- whether publish readiness is proven.

Those are separate proof lanes.

## Coverage workflow

The Coverage workflow runs on:

- push to `main`,
- `workflow_dispatch`,
- PRs labeled `coverage` or `full-ci`.

Codecov comments are disabled. Durable receipts are:

- `coverage.json`,
- `coverage.txt`,
- `lcov.info`,
- `coverage-receipt.json`,
- the GitHub Actions coverage artifact,
- the Codecov dashboard.

## Claim boundary

Codecov coverage is execution-surface evidence only. It does not prove:

- shiplog coverage-manifest completeness,
- packet quality,
- source adapter completeness,
- redaction safety,
- LLM clustering quality,
- mutation adequacy,
- publish readiness.
