# v0.4.0 Release Matrix

This matrix is the crates.io decision record for v0.4.0 Review Rescue. It keeps
the current invariant: production workspace packages are publishable public
surfaces; `publish = false` is reserved for dev-only packages.

## Decision Rules

- Publish crates that are stable contracts, product/trust surfaces, real
  adapters, renderers, or heavy optional feature boundaries.
- Do not publish dev-only tooling.
- If a published crate depends on another workspace crate, publish or dry-run
  the dependency first in topological order.
- Keep implementation seams as owner modules, not standalone unpublished
  production packages.

## Matrix

| Crate | Release v0.4.0? | Why | Required before yes |
|---|---:|---|---|
| `shiplog` | yes | CLI product and review-rescue composition root | package proof |
| `shiplog-ids` | yes | deterministic ID contract | package proof |
| `shiplog-schema` | yes | canonical persisted schema | package proof |
| `shiplog-ports` | yes | extension traits | package proof |
| `shiplog-coverage` | yes | coverage honesty | package proof |
| `shiplog-cache` | yes | supported API-cache facade and CLI cache UX | package proof |
| `shiplog-redact` | yes | privacy trust surface | package proof |
| `shiplog-bundle` | yes | share/checksum artifacts | package proof |
| `shiplog-workstreams` | yes | curation domain | package proof |
| `shiplog-merge` | yes | stable multi-source merge surface used by engine/CLI | package proof |
| `shiplog-render-md` | yes | primary packet/scaffold/receipt renderer | package proof |
| `shiplog-render-json` | yes | machine-readable renderer | package proof |
| `shiplog-ingest-json` | yes | stable JSONL import format | package proof |
| `shiplog-ingest-manual` | yes | manual evidence lane | package proof |
| `shiplog-ingest-git` | yes | local git adapter with collect/refresh/run CLI support | package proof |
| `shiplog-ingest-github` | yes | core GitHub adapter | package proof |
| `shiplog-ingest-gitlab` | yes | GitLab adapter with collect/refresh/run CLI support | package proof |
| `shiplog-ingest-jira` | yes | Jira adapter with collect/refresh/run CLI support | package proof |
| `shiplog-ingest-linear` | yes | Linear adapter with collect/refresh/run CLI support | package proof |
| `shiplog-cluster-llm` | yes | optional privacy/network boundary behind `llm` | package proof |
| `shiplog-team` | yes | optional team aggregation library surface | package proof |
| `shiplog-engine` | yes | orchestration API | package proof |
| `shiplog-testkit` | no | dev-only fixtures and scenarios | `publish = false` |

## Topological Publish Order

Use the scripted order for package proof, dry-run, and manual publication:

```text
shiplog-ids
shiplog-schema
shiplog-ports
shiplog-coverage
shiplog-cache
shiplog-redact
shiplog-bundle
shiplog-workstreams
shiplog-merge
shiplog-render-md
shiplog-render-json
shiplog-ingest-json
shiplog-ingest-manual
shiplog-ingest-git
shiplog-ingest-github
shiplog-ingest-gitlab
shiplog-ingest-jira
shiplog-ingest-linear
shiplog-cluster-llm
shiplog-team
shiplog-engine
shiplog
```

For a first publication of interdependent `0.4.0` crates, downstream
`cargo publish --dry-run` can fail until upstream `0.4.0` versions are visible
in the crates.io index. Resume with:

```bash
scripts/publish-dry-run.sh --from <package>
```

## Required Release Proof

Before publishing v0.4.0, run:

```bash
git diff --check
cargo metadata --format-version 1 --no-deps
scripts/package-boundary-audit.sh
scripts/package-version-audit.sh
scripts/package-proof.sh
```

`scripts/package-proof.sh` also runs formatting, clippy, workspace tests,
no-default CLI tests, fuzz-bin check, cargo-deny, and package listings.

During crates.io publication, dry-run and publish one package at a time in the
topological order above. A full `scripts/publish-dry-run.sh` pass is expected to
complete only after every upstream `0.4.0` dependency is visible in the
crates.io index.

After the tag workflow builds release assets, verify the public release with:

```bash
scripts/verify-release.sh v0.4.0
```
