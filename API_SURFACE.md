# shiplog API Surface

shiplog is module-first. Public crates are deliberate external contracts, not
the default shape for internal SRP boundaries. During the 0.7 crate-surface
contraction, internal seams should move toward owner modules unless a spec and
ADR promote them as supported public APIs.

The product is the CLI workflow:

```text
collect -> curate -> render
```

The artifact contract is:

```text
packet + ledger + coverage + bundles
```

The mental model is: human narrative, machine receipts.

## Forward Public Surface

- `shiplog`: CLI product and composition root.

No other crate is public-supported for 0.7. `shiplog-schema` is internal;
JSON schemas under `contracts/schemas/` are the public machine contract.

The primary user path is still `cargo install shiplog --locked`.

## Development Support

- `shiplog-testkit`: dev-only fixtures and BDD helpers. It is not published.
- `xtask`: repository automation. It is not published.

## 0.6 Historical Crates

shiplog 0.6.0 published implementation crates that are no longer treated as
forward public contracts by default. They should not be yanked as routine
cleanup, but 0.7 release tooling should not republish them unless a support-tier
decision explicitly allows it.

Examples include:

- `shiplog-ids`
- `shiplog-ports`
- `shiplog-engine`
- `shiplog-coverage`
- `shiplog-workstreams`
- `shiplog-redact`
- `shiplog-bundle`
- `shiplog-cache`
- `shiplog-render-md`
- `shiplog-render-json`
- `shiplog-ingest-*`
- `shiplog-cluster-llm`
- `shiplog-team`
- `shiplog-merge`

## Internal Module Families

These are implementation seams, not package promises. They should live under the
nearest owning crate unless a later PR deliberately promotes them.

- Markdown rendering: `shiplog::render::md`.
- JSON artifact writing: `apps/shiplog/src/engine/artifact_json.rs`.
- Redaction internals: profile, policy, projector, repo, alias.
- Cache internals: `shiplog::cache::{key, stats, expiry, sqlite}`.
- Date windows and coverage calculations: `shiplog::coverage`.
- Output layout and bundle assembly: `shiplog::bundle`.
- Team phases: core, aggregate, render, template.
- Workstream phases: cluster, layout, receipt policy.
- LLM prompt/parse helpers.
- Source adapters: github, git, json, manual, gitlab, jira, linear.
- Manual event parsing.
- Generic data structures, queues, counters, random helpers, middleware,
  parsers, normalizers, paths, URLs, encoders, and other utility code.

## Optional Boundaries

Optional capabilities can still use Cargo features and owner modules. They earn
a public crate only when an external support contract exists.

- LLM clustering stays feature-gated and off by default.
- Team aggregation stays optional.
- Provider adapters remain internal until shiplog specifies a plugin or adapter
  API.

## Promotion Rule

Start every new boundary as a module. Promote it to a crate only when at least
all of these are true:

- External users are expected to import it directly.
- It has an independently meaningful API.
- It can be versioned and supported separately from the CLI package.
- Its docs explain its external contract.
- Removing it would break a real user, not only shiplog's internal layout.

If two public crates need the same shared type, move the shared contract into
`shiplog-schema`, `shiplog-ports`, or `shiplog-ids`. Do not hide production
dependencies in unpublished sibling crates without a clear owner.

If a production seam is not ready to be a public package, fold it under its
owning package as an SRP module.

## Feature Flags

Feature flags must represent user-visible capabilities or dependency
boundaries, not internal file layout.

Use flags like these when the capability exists:

- `llm`
- `team`
- `gitlab`
- `jira`
- `linear`
- `plugin` only after a plugin API is promoted.

Avoid flags named after internal implementation parts such as cache keys,
redaction policy, or output layout.

## Current State

The 0.7 contraction is in progress:

- `shiplog-render-md` has been inlined as `shiplog::render::md`.
- `shiplog-render-json` has been inlined into the engine artifact writer for
  this contraction slice.
- Source adapters have been inlined under `shiplog::ingest`.
- `shiplog-cache` has been inlined as `shiplog::cache`.
- `shiplog-coverage` has been inlined as `shiplog::coverage`.
- `shiplog-bundle` has been inlined as `shiplog::bundle`.
- Remaining implementation packages are classified by
  [`SHIPLOG-SPEC-0004`](docs/specs/SHIPLOG-SPEC-0004-public-crate-support-tiers.md)
  and the 0.7 crate-surface audit.

New implementation seams should start as owner modules and should not become
workspace crates unless a spec and ADR deliberately promote them.
