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

## Current baseline

At `main` commit `8827f6a`, the Codecov-equivalent Rust surface measured:

| Metric | Covered | Total | Coverage |
| ------ | ------: | ----: | -------: |
| Lines | 14,958 | 17,624 | 84.87% |
| Regions | 23,429 | 28,450 | 82.35% |
| Functions | 1,311 | 1,568 | 83.60% |

The local receipt used the same package scope as the workflow:

```bash
cargo llvm-cov clean --workspace
cargo llvm-cov nextest --workspace --exclude shiplog-testkit --locked --all-features --no-report
cargo llvm-cov report --json --summary-only \
  --ignore-filename-regex 'crates[\\/]+shiplog-testkit' \
  --output-path target/coverage-summary.json
```

`shiplog-testkit` is excluded from the publish/product coverage surface through
`codecov.yml`, so the baseline excludes it from the reported totals.

## Advisory ratchet

The first project-level Codecov target is `80%` with a `5%` threshold and
`informational: true`. Patch coverage remains informational. This makes
coverage drift visible without blocking product fixes while the baseline
matures.

## Claim boundary

Codecov coverage is execution-surface evidence only. It does not prove:

- shiplog coverage-manifest completeness,
- packet quality,
- source adapter completeness,
- redaction safety,
- LLM clustering quality,
- mutation adequacy,
- publish readiness.
