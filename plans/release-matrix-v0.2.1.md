# v0.2.1 Release Matrix

This matrix is the crates.io decision record for v0.2.1. It intentionally does
not treat every workspace member as a publish target.

## Decision Rules

- Publish crates that are stable contracts, product/trust surfaces, real
  adapters, or required optional feature boundaries.
- Hold conditional adapters until their CLI story, auth model, examples, and
  tests are release-grade.
- Do not publish dev-only tooling.
- If a published crate depends on another workspace crate, the dependency must
  either be published first or removed from the public feature surface.

## Matrix

| Crate | Release v0.2.1? | Why | Required before yes |
|---|---:|---|---|
| `shiplog` | yes | CLI product | package proof |
| `shiplog-ids` | yes | deterministic ID contract | package proof |
| `shiplog-schema` | yes | canonical persisted schema | package proof |
| `shiplog-ports` | yes | extension traits | package proof |
| `shiplog-coverage` | yes | coverage honesty | package proof |
| `shiplog-redact` | yes | privacy trust surface | package proof |
| `shiplog-bundle` | yes | share/checksum artifacts | package proof |
| `shiplog-workstreams` | yes | curation domain | package proof |
| `shiplog-cache` | yes | supported cache facade | package proof |
| `shiplog-render-md` | yes | primary artifact renderer | package proof |
| `shiplog-render-json` | yes | machine-readable renderer | package proof |
| `shiplog-engine` | yes | orchestration API | package proof |
| `shiplog-merge` | yes | engine dependency and merge API surface | package proof |
| `shiplog-ingest-github` | yes | core GitHub adapter | package proof |
| `shiplog-ingest-git` | yes | local git adapter used by the CLI `collect git` path | document refresh/run limitation |
| `shiplog-ingest-json` | yes | stable import format | package proof |
| `shiplog-ingest-manual` | yes | manual evidence lane | package proof |
| `shiplog-ingest-gitlab` | hold | conditional adapter, not wired into CLI docs | CLI/auth examples and release-grade tests |
| `shiplog-ingest-jira` | hold | conditional adapter, not wired into CLI docs | CLI/auth examples and release-grade tests |
| `shiplog-ingest-linear` | hold | conditional adapter, not wired into CLI docs | CLI/auth examples and release-grade tests |
| `shiplog-cluster-llm` | yes | optional privacy/network boundary behind `llm` | package proof and fallback/privacy docs |
| `shiplog-team` | yes | optional `team` feature dependency from the CLI | package proof; examples may follow after release |
| `shiplog-template` | yes | dependency of `shiplog-team` | package proof; do not promote template syntax as stable without docs |
| `shiplog-testkit` | no | dev-only fixtures | `publish = false` |

## Topological Publish Order

Use this order for `scripts/publish-dry-run.sh` and for manual crates.io
publication. For first publication of a new interdependent version, dry-run and
publish one crate at a time, then resume from the next crate:

```bash
scripts/publish-dry-run.sh --from shiplog-schema
```

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
shiplog-cluster-llm
shiplog-template
shiplog-team
shiplog-engine
shiplog
```

GitLab, Jira, and Linear are workspace members but are held from this release
set. If package dry-run proves that a public crate still depends on one of
them, either publish that dependency deliberately or remove the dependency
before tagging.

`shiplog-template` is versioned independently at `0.3.0`; it is included here
only because the published optional `team` feature depends on it.

## Observed Dry-Run Boundary

On the clean v0.2.1 readiness branch, `scripts/publish-dry-run.sh` proved
`shiplog-ids` and then stopped at `shiplog-schema` because crates.io had
`shiplog-ids 0.2.0` but not `shiplog-ids 0.2.1`:

```text
failed to select a version for the requirement `shiplog-ids = "^0.2.1"`
candidate versions found which didn't match: 0.2.0
```

That is a registry-state limitation of first-time multi-crate publication, not
a package-list failure. The release process must interleave dry-run and publish
steps in the topological order until each required dependency version is visible
on crates.io.
