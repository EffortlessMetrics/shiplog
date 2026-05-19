# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

No user-facing changes yet after the held 0.9.0 candidate.

## [0.9.0] - 2026-XX-XX (unreleased candidate)

shiplog 0.9.0 is the **review-loop cockpit release**.

It turns shiplog from a packet generator into a setup-aware, receipt-backed
review loop: diagnose setup, collect evidence, inspect status, repair gaps,
rerun, compare, and share safely.

### Added

- Added `shiplog status --latest`, a read-only cockpit over setup, latest run,
  packet readiness, source state, repair, diff, share posture, blockers, next
  actions, and receipt refs (#429).
- Added `shiplog status --latest --json`, the same status model for agents and
  scripts, with stable keys, write-labelled next actions, blockers, and receipt
  references (#430).
- Added `shiplog init --guided`, `doctor --setup`, `sources status`, and
  `doctor --setup --json` as the setup-readiness front door before intake,
  repair, and share commands (#403-#405, #412).
- Added review-ready packet quality: Packet Readiness, evidence strength,
  receipt-backed claim candidates, missing-context prompts, share posture, and
  quality movement across reruns (#311-#316).
- Added `share explain manager|public` and `runs diff --latest` as read-only
  posture and comparison surfaces (#315, #316).

### Changed

- Intake and repair handoffs are read-first: repairable runs route through
  `repair plan`, setup-blocked repairs route through doctor/source status, and
  repaired reruns route toward diff before more planning.
- Share flows fail closed and explain posture before rendering manager or public
  packets.
- Workspace package versions moved from `0.8.0` to `0.9.0` as candidate
  metadata only; release execution remains held until explicit owner approval
  and current preflight exist.

### Fixed

- Fixed guided local Git configs using `repo = "."` without an `origin` remote.
- Fixed misleading zero-event source "Good" output.
- Fixed old/partial report and setup compatibility paths.
- Fixed Windows path and environment-variable display rough edges found in
  dogfood.

### Documentation and proof

- Added setup-readiness, review-ready, and review-loop status proposals, specs,
  ADRs, schemas, examples, dogfood matrices, and transcripts.
- Added the review-loop status transcript (#434), recurring review-loop guide,
  and README/front-door alignment so status is taught as the recurring cockpit.
- Reworked the root README as the 0.9 product front door: problem, candidate
  surfaces, install paths, setup, first packet, repair/share, status at a
  glance, docs map, safety boundaries, and public-surface summary (#438).
- Tightened the crates.io README into an install-and-first-loop page with
  command highlights, read/write posture, agent surfaces, docs links, and
  safety boundaries (#439).
- Updated 0.9 release readiness, release hold, release decision, and handoff
  docs so the held candidate is framed as the review-loop cockpit release with
  status human/JSON, schema, consistency proof, safe-next-action proof,
  dogfood transcript, recurring guide, and README alignment (#440).
- Aligned the guide map around the status-first workflow: recurring review loop
  as the flagship guide, rapid/setup/repair/review-ready guide cross-links,
  setup/status schema links, roadmap/install freshness, and release-proof map
  pointers in dogfood transcripts and matrices (#441).
- Replaced the flat README badge row with proof, distribution, and contract
  badge groups, added GitHub release and crates.io download badges, and added a
  checked-in `ripr+` Shields endpoint so the badge link is not broken (#442).
- Kept detailed post-0.8 soak and release-hold receipts in
  [`docs/release/0.9.0-readiness.md`](docs/release/0.9.0-readiness.md) and
  [`docs/release/0.9.0-release-hold.md`](docs/release/0.9.0-release-hold.md).

Key receipts: #307-#319, #337-#398, #399-#422, #424-#436.

Release execution is still paused: do not tag, publish to crates.io, create a
GitHub release, dispatch release workflows, or run release-install smoke for
`v0.9.0` while the release hold remains active.

## [0.8.0] - 2026-05-15

shiplog 0.8.0 is the **evidence repair loop release**. It turns the honest
first-run report from 0.6/0.7 into a guided repair path: inspect receipts, print
safe repair actions, add local manual evidence from a repair ID, rerun intake,
and see which repair keys cleared.

### Added

- Added receipt-derived `repair_items` to `intake.report.json`, with stable
  `repair_id`, `repair_key`, source identity, reason, safe action, clear
  condition, and receipt references. Repair items are derived from existing
  report receipts instead of live source rediscovery (#296).
- Added `shiplog repair plan --latest` to render the latest report's repair
  queue, including missing-report, legacy-report, invalid-report, and empty-queue
  handling (#297).
- Added `shiplog journal add --from-repair <repair_id>` so a manual-evidence
  repair item can append a local journal event without guessing, overwriting
  existing journal content, or mutating provider records (#298).
- Added `shiplog repair diff --latest` to compare the latest two compatible
  repair reports by `repair_key` and show cleared, new, still-open, and changed
  repair items (#299).

### Documentation

- Added the evidence repair loop source-of-truth stack: proposal, spec, ADR,
  implementation plan, active goal, user guide, README pointer, and release
  readiness/handoff notes (#293-#295, #300).

### Tests

- Added a product-level proof that a cold `Needs evidence` first run can produce
  a repair plan, add manual evidence from a repair ID, rerun intake, clear the
  no-events repair key, and produce a more useful packet without provider
  mutation (#300).

### Release

- Bumped workspace packages and normal workspace dependency requirements from
  `0.7.0` to `0.8.0`.

## [0.7.0] - 2026-05-14

shiplog 0.7.0 is the **crate-surface contraction release**. It preserves the
0.6.0 first-run review-pack behavior while making the forward crates.io support
surface intentional: `shiplog` is the supported public package, and the former
0.6 implementation crates are historical surfaces rather than ongoing public
APIs.

### Changed

- Collapsed the former render, ingest, cache, coverage, engine, bundle,
  redaction, merge, team, workstream, LLM, ID, schema, and port implementation
  crates into SRP modules inside the `shiplog` package while preserving
  first-run intake/open/report behavior (#254, #260-#267, #269, #271, #273,
  #275, #277, #279, #281).
- Kept JSON schemas under `contracts/schemas/` as the public machine contract
  and made Rust schema types internal to `shiplog` for 0.7 (#258, #281).
- Bumped all workspace packages and internal path dependency requirements from
  `0.6.0` to `0.7.0`.

### Release

- Added the 0.7 crate-surface source-of-truth stack: proposal, support-tier
  spec, SRP-module ADR, crate-surface audit, implementation plan, and active
  goal (#250-#253, #259).
- Added the 0.6 historical crate-surface receipt and kept those crates
  available as historical/transitional artifacts rather than yanking them as
  routine cleanup (#257).
- Enforced the 0.7 publish allowlist and package dependency closure so release
  tooling publishes only `shiplog` by default and fails if unsupported
  historical or non-public workspace packages re-enter the forward publish
  graph (#255, #256, #283).

## [0.6.0] - 2026-05-13

shiplog 0.6.0 is the **user-polish review-pack release**. It turns the first
intake path into a receipt-backed control loop: run intake, open the latest
report or packet, see which sources worked or were skipped, understand
freshness, and get the next repair command without rediscovering context.

### Added

- Added post-intake next-step output with the run directory, latest-artifact
  open commands, and repair/rerun guidance when evidence is still missing
  (#236).
- Added deterministic latest-run opening for `shiplog open intake-report
  --latest`, `shiplog open packet --latest`, and `shiplog open out --latest`
  (#237).
- Added skipped-source freshness rows so `source_freshness` and
  `source_decisions` agree on configured sources that could not run, including
  reason text for repair guidance (#243).
- Added `CacheLookup::{Fresh, Stale, Miss}` and wired GitHub freshness so
  `stale` is emitted only from a proven expired cache row (#244).

### Changed

- Canonicalized intake-report source identity: machine-readable JSON now emits
  `source_key` plus `source_label`, while Markdown keeps reviewer-friendly
  labels. The legacy `source` field remains as a v1 compatibility alias
  (#238).
- Bumped all workspace packages and internal path dependency requirements from
  `0.5.0` to `0.6.0`.

### Documentation

- Added the user-polish source-of-truth stack: proposal, first-intake/report
  specs, source-identity/freshness/open/latest/share/repair/CI-economics
  contracts, ADRs, implementation plan, and active goal manifest (#230-#235).
- Added this release readiness/handoff set for the 0.6.0 user-polish lane.

### Tests

- Added recorded GitHub HTTP fixtures that prove a full first run is `fresh`
  and a second run against the same cache is `cached`, without live network
  access (#245).

## [0.5.0] - 2026-05-10

shiplog 0.5.0 is **Operational Review Rescue + Rust 1.95 quality floor +
policy/CI economics foundation.** It bundles the operational hardening lane
already merged after v0.4.0 (PRs #125–#139), an MSRV bump from Rust 1.92 to
Rust 1.95, and a first cut of policy ledgers, a thin Rust-native `xtask`
runner, an advisory LEM-budgeted CI lane plan, and `ripr` advisory routing.
Hard CI budget enforcement is a follow-up release decision.

See [`docs/ci/rust-1.95-rollout.md`](docs/ci/rust-1.95-rollout.md) for the
rollout map and the 18-PR ladder (#140–#157).

### Changed

- **MSRV bumped from Rust 1.92 to Rust 1.95** (#145). `rust-toolchain.toml`,
  `Cargo.toml` `rust-version`, `clippy.toml` `msrv`, and the toolchain pin in
  every CI/release workflow are now `1.95.0`. Compatibility was probed in #144
  before the mechanical bump in #145.
- **`SourceSystem` serde** is now flat lowercase strings (`"github"`,
  `"json_import"`, `"local_git"`, `"manual"`, `"unknown"`); deserialization is
  case-insensitive for backward compatibility with old PascalCase ledger
  values.

### Added — Operational hardening lane (already shipped after v0.4.0)

- Added the release-install smoke test, the no-network demo, the intake-report
  v1 contract with `validate` and `summarize`, repair classifiers,
  `doctor --repair-plan`, source-failure receipts, share manifests with
  verification, stable fixup IDs, machine-readable intake actions, period
  inspection and comparison, and the agent-pack export from the intake
  report (PRs #125–#139).

### Added — Policy ledgers + xtask runner

- Added the `xtask/` Rust-native policy runner (#143) with eight task
  modules: `check-policy-schemas`, `ci plan`, `ci actuals`,
  `check-lint-policy`, `check-clippy-exceptions`, the seven file-policy
  checkers (#149), `check-no-panic-family` (#151), `package-boundary` and
  `package-version` shell wrappers, and `policy-report`.
- Added 18 policy ledger files under `policy/` (#141, #149, #150, #151, #152,
  #153, #154, #155): CI lanes, CI budget, CI risk packs, CI exceptions,
  Clippy lints + debt + exceptions, no-panic baseline (541 entries / 1112
  occurrences), non-Rust allowlist + companion ledgers (generated, executable,
  workflow, dependency, process, network), and `ripr-suppressions`. All
  ledgers ship with a common header (`schema_version`, `policy`, `owner`,
  `status`) and load via `cargo xtask check-policy-schemas`.
- Added Rust 1.95 lint floor (#152): `unsafe_op_in_unsafe_fn`,
  `unused_must_use`, `unexpected_cfgs`, `const_item_interior_mutations`,
  `function_casts_as_integer`, `unused_visibilities` activated as Rust lints;
  `same_length_and_capacity`, `manual_checked_ops`, `manual_take`,
  `duration_suboptimal_units`, `unnecessary_trailing_comma`,
  `needless_type_cast`, `manual_ilog2`, `decimal_bitwise_operands`,
  `unnecessary_sort_by` activated as Clippy lints. All bare `#[allow]` sites
  converted to `#[expect(..., reason = "policy:<id>")]` with citations into
  `policy/clippy-exceptions.toml`.
- Added the no-panic baseline (#151) with exact-identity matching
  `(path, family, selector_kind, selector_callee, snippet, count)` and
  no-new-debt enforcement.
- Added file-policy enforcement (#149) with seven checkers (workflow,
  generated, executable, dependency, process, network, non-Rust) sharing a
  `Mode { Advisory | BlockingAllowlist }` enum.

### Added — CI economics

- Added the **LEM (Linux Equivalent Minutes) cost model** as the canonical
  CI cost unit. Runner multipliers: ubuntu 1×, windows 2×, macos 10×, docker
  6×, AI 4×. See [`docs/ci/lem-budgeting.md`](docs/ci/lem-budgeting.md) and
  [`docs/ci/cost-and-verification-policy.md`](docs/ci/cost-and-verification-policy.md).
- Added the **PR Plan** advisory workflow (#146): `.github/workflows/pr-plan.yml`
  emits `target/ci/ci-plan.json` against
  [`contracts/schemas/ci-plan.v1.schema.json`](contracts/schemas/ci-plan.v1.schema.json),
  forecasting per-lane LEM, touched risk packs, and the planned set for the
  PR. Always advisory in v0.5.0.
- Added the **CI Actuals collector** (#148): `.github/workflows/ci-actuals.yml`
  triggers on `workflow_run` completion for every instrumented workflow,
  fetches per-job timings via `gh api`, joins to `policy/ci-lanes.toml` +
  `policy/ci-budget.toml`, and emits `target/ci/ci-actuals.json` against
  [`contracts/schemas/ci-actuals.v1.schema.json`](contracts/schemas/ci-actuals.v1.schema.json).
  Closes the LEM forecast/actual loop.
- Added cache economics (#147): every workflow uses `Swatinem/rust-cache@v2`
  with `save-if: ${{ github.ref == 'refs/heads/main' }}` so PRs restore-only;
  `paths-ignore` keeps docs-only PRs off compile-heavy non-required lanes.
- Added 18 CI operating contract docs (#142): `ci-lane-map`, `labels`,
  `risk-packs`, `branch-protection`, `required-check-migration`,
  `skipped-by-policy`, `bot-review-policy`, `cache-policy`, `ci-plan-json`,
  `ci-actuals`, `ripr`, `per-pr-acceptance-contract`, `policy-ledgers`,
  `cost-and-verification-policy`, `lem-budgeting`, `verification-ladder`,
  `test-evidence-lanes`, `coverage`, `mutation`.

### Added — Evidence lanes

- Added the **`ripr` advisory lane** (#153): `.github/workflows/ripr.yml`
  is wired as a v0.5.0 stub on every Rust-diff PR, emitting
  `target/ripr/ripr.json` (schema v1) and `target/ripr/ripr.sarif` (SARIF
  2.1.0) so downstream tooling (PR plan, CI actuals, future required-check
  promotion) can treat the lane as live. Real ripr integration is a
  follow-up release. Always advisory.
- Added **bounded smoke lanes** for the PR-fast tier (#154): `bdd-smoke.yml`
  (two critical multi-source merge + render scenarios), `property-smoke.yml`
  (`PROPTEST_CASES=64` with deterministic seed across foundation crates),
  `fuzz-smoke.yml` (touched-target fuzz at 30s based on changed paths).
- **Routed broad evidence lanes** off PR-on-every (#155): `bdd-testing.yml`,
  `property-testing.yml`, `fuzzing.yml#quick-fuzz`, `security.yml`, and
  `mutation-testing.yml` now require an explicit opt-in label
  (`bdd` / `property-tests` / `fuzz` / `security-audit` / `mutation`, or
  `full-ci`) on PR; nightly cron and dispatch unchanged. PR cargo-deny
  coverage is still blocking via `ci.yml#cargo-deny`.

### Optimized

- Hinted `core::hint::cold_path()` on four canonical fail-closed redaction
  paths (#156): alias cache version mismatch, bundle profile rendering
  prerequisite mismatch, share-command redaction-key required, and
  share-profile redaction-key required.

### Fixed — Release-prep cleanup (post-#157)

- Tolerated empty `GITHUB_PR_NUMBER` in `cargo xtask ci actuals` (#159). The
  `ci-actuals.yml` lane runs on `workflow_run` for every push to `main`,
  where the upstream PR field resolves to an empty string; clap's
  `Option<u32>` parser previously rejected the empty value and exited with
  `cannot parse integer from empty string`. The CLI now reads
  `GITHUB_PR_NUMBER` manually, treating absent and empty as `None`, which
  serializes as `"pr_number": null` (the schema already permits null).
  Four pure unit tests cover absent / empty / valid / garbage.
- Registered `scripts/publish-v0.5.0.sh` in
  `policy/executable-allowlist.toml` and anchored the demo rescue script
  defaults to `<repo>/target/release/shiplog` instead of `shiplog` on
  `$PATH` (#160). The publish script gained the executable bit when it
  landed in #158 but never had an allowlist entry, so
  `cargo xtask check-executable-files --mode blocking-allowlist` failed
  locally; file-policy is `Mode::Advisory` in v0.5.0 CI by design, so this
  never fired in CI but did block the readiness contract. Demo rescue
  scripts now resolve the binary deterministically against the workspace
  build; explicit overrides (`--shiplog-bin` / `-ShiplogBin`) are
  unaffected.

### Documentation

- Added the rollout map [`docs/ci/rust-1.95-rollout.md`](docs/ci/rust-1.95-rollout.md)
  (#140) covering current-vs-target table, existing CI/evidence inventory
  with tentative lane assignments, the 18-PR ladder, per-PR acceptance gates,
  per-PR operating contract, the self-review checklist, and the v0.5.0
  definition of done.
- Added policy doctrine docs: [`docs/CLIPPY_POLICY.md`](docs/CLIPPY_POLICY.md),
  [`docs/NO_PANIC_POLICY.md`](docs/NO_PANIC_POLICY.md),
  [`docs/FILE_POLICY.md`](docs/FILE_POLICY.md),
  [`docs/POLICY_ALLOWLISTS.md`](docs/POLICY_ALLOWLISTS.md).
- Added [`docs/release/0.5.0-readiness.md`](docs/release/0.5.0-readiness.md)
  and [`RELEASE_HANDOFF_0.5.0.md`](RELEASE_HANDOFF_0.5.0.md) (#157).
- Added the dependency-ordered v0.5.0 publish driver
  `scripts/publish-v0.5.0.sh` and updated the readiness ledger's
  "Tag / publish order" section with the canonical 22-crate sequence (#158).
- Backfilled the `[0.5.0]` CHANGELOG block with post-#157 fixes and refreshed
  [`RELEASE_HANDOFF_0.5.0.md`](RELEASE_HANDOFF_0.5.0.md) and the README
  Documentation pointers to land the four new policy doctrine docs (#161).

### Security

- Kept manager/public sharing fail-closed and added `cold_path()` hints on
  the two share-profile redaction-key required paths so the optimizer knows
  these are diagnostic-quality errors, not hot paths.

## [0.4.0] - 2026-05-09

### Added

- Added the review rescue intake path: `shiplog intake` creates a best-effort
  first packet, records skipped sources as coverage warnings, renders the
  packet, runs review inspection, and writes durable `intake.report.md` and
  `intake.report.json` artifacts.
- Added intake readiness summaries, source-decision explanations, source repair
  and rerun guidance, curation-preserving intake reruns, and `shiplog open
  intake-report`.
- Added `shiplog review`, `shiplog review weekly`, `shiplog review --strict`,
  `shiplog review fixups`, `--commands-only`, and `--journal-template` for
  packet-quality inspection and copy-ready next actions.
- Added manual evidence capture and correction with `shiplog journal add`,
  `shiplog journal list`, and `shiplog journal edit`.
- Added manager/public share convenience commands and read-only share preflight
  checks with `shiplog share manager|public` and `shiplog share verify
  manager|public`.
- Added `shiplog share verify public --strict` to scan an existing or in-memory
  public packet for obvious raw URLs and original names before sharing.
- Added `shiplog runs compare` for read-only cross-run comparison.
- Added named review periods in `shiplog.toml` for `intake`, `collect multi`,
  and `review`.
- Added `shiplog identify jira` and `shiplog identify linear` to print provider
  account/user IDs for source configuration.
- Added `scripts/verify-release.sh` to repeat public post-release verification
  for GitHub assets, checksums, crates.io install, and binary smoke checks.

### Documentation

- Made the README quick start intake-first and added a review-deadline guide
  covering 5-minute, 15-minute, and 30-minute rescue flows.
- Added binary-first install documentation with release binary downloads,
  checksum verification, `cargo install`, and package-manager tracking notes.
- Documented v0.3.0 release verification and updated the current-state record
  from release-ready to shipped.
- Added a `shiplog.toml` config reference covering versioning, defaults,
  source fields, env vars, validation, and redaction safety.

### Fixed

- Changed CLI-generated packets to open with coverage and gaps before summary,
  workstreams, and receipts, and made packet mode default to a receipt-summary
  appendix while receipts mode keeps the full audit appendix.
- Polished Markdown packet summary labels so event counts pluralize correctly
  and source names use reader-facing labels.
- Fixed Jira search response deserialization for realistic REST payload field
  names.
- Fixed the packet file-artifacts section so it lists generated run artifacts
  instead of the manual-event input file.
- Fixed public redaction so provider-specific source opaque IDs are stripped
  along with source URLs.
- Fixed internal bundle zips so an archive written inside the run directory
  cannot include itself.
- Corrected redaction docs and package metadata to describe keyed SHA-256
  aliasing instead of HMAC.
- Improved share-profile redaction-key failures with copy-ready recovery
  guidance.

### Testing

- Added golden intake fixtures for manual-only rescue, all-source intake,
  skipped-source visibility, readiness output, manager-share missing-key
  safety, durable report content, curation-preserving reruns, and strict public
  verification.
- Added no-network provider edge smoke coverage for enterprise/self-hosted
  source configuration, invalid provider filters, partial source failures, and
  intake repair guidance.
- Added recorded-style GitHub provider payload tests for search, pull request
  details, and review parsing through cached event conversion.
- Added recorded-style GitLab provider payload tests for merge request and
  review-note parsing and event conversion.
- Added recorded-style Jira and Linear provider payload tests for adapter
  parsing and event conversion.
- Added a documented Codecov baseline and first informational project coverage
  target.
- Added the first documented mutation baseline for the `shiplog-coverage` trust
  surface.
- Added the documented mutation baseline for the `shiplog-ids` deterministic ID
  contract.
- Recorded the `shiplog-ports` mutation scan result as a no-target trait
  contract crate.
- Added the documented mutation baseline for the `shiplog-schema` persisted
  data contract.
- Added the documented mutation baseline for the `shiplog-redact` privacy trust
  surface.
- Added the documented mutation baseline for the `shiplog-bundle` artifact
  integrity surface.

### Security

- Kept manager/public sharing fail-closed while adding explicit share verbs,
  read-only preflight checks, and strict public verification guardrails that do
  not print redaction keys or token values.

## [0.3.0] - 2026-05-07

### Added

- Added CLI-supported GitLab, Jira, and Linear sources across `collect`, `refresh`, and legacy `run`.
- Added local git parity for `refresh git` and `run git`.
- Added `shiplog merge` to combine existing source runs into one packet.
- Added `shiplog collect multi` to collect enabled `shiplog.toml` sources into one merged packet while recording partial source failures in coverage warnings.
- Added first-run commands and defaults: `shiplog init`, `shiplog doctor`, `shiplog config validate`, `shiplog config explain`, `shiplog config migrate`, relative date presets, latest-run aliases, and GitHub/GitLab `--me` identity inference.
- Added run discovery and artifact opening with `shiplog runs list/show` and `shiplog open packet/workstreams/out`.
- Added workstream curation commands for list, validate, create, rename, move, split, receipt selection, and delete.
- Added packet coverage and limits summaries, source/gap summaries, evidence anchors, claim prompts, render modes (`packet`, `scaffold`, `receipts`), receipt limits, and appendix density controls.
- Added cache visibility and cleanup with `shiplog cache stats`, `shiplog cache inspect`, and `shiplog cache clean`.
- Added example configs and a review-cycle guide with fixture-safe docs command tests.

### Changed

- Promoted GitLab, Jira, Linear, team, and merge surfaces into the v0.3.0 publish set instead of leaving production-looking crates unpublished.
- Aligned the workspace package surface around publishable public crates plus dev-only tooling.
- Versioned `shiplog.toml` with `[shiplog] config_version = 1` while keeping legacy configs as implicit v1.
- Expanded release smoke tests to exercise product command help on the downloaded release artifact and release build.

### Security

- Manager and public render/bundle profiles now fail closed unless `--redact-key` or the configured redaction-key environment variable is provided.

### Testing and Release Proof

- Added package boundary and package version audits to prevent unpublished production crates and mixed release versions.
- Added fixture-safe command coverage for the review-cycle documentation path.
- Kept mutation testing advisory while the baseline matures.

## [0.2.1] - 2026-02-17

### Added

- CLI `manual` source wiring for `collect`, `refresh`, and `run`, enabling YAML manual-event ingestion through `shiplog-ingest-manual`
- GitHub CLI cache controls: `--cache-dir <PATH>` and `--no-cache`

### Changed

- GitHub ingestion now enables persistent API caching by default at `<out>/.cache` unless `--no-cache` is used
- `collect json` now honors `--regen` for suggested workstream regeneration, matching other collect sources
- `GithubIngestor::with_cache` now creates missing cache directories automatically

## [0.2.0] - 2026-02-15

### Added

- **LLM-assisted workstream clustering** (`shiplog-cluster-llm`): optional OpenAI-compatible endpoint for semantic workstream grouping, with automatic fallback to repo-based clustering on failure
- **`import` subcommand**: re-render a pre-built ledger directory from an upstream system or previous shiplog run
- **Redaction alias cache persistence**: `redaction.aliases.json` provides stable alias mappings across runs
- **`--zip` flag**: write a zip archive next to the run folder (available on `collect`, `render`, `refresh`, `import`, `run`)
- **`--run-dir` flag**: explicit run directory for `refresh` (overrides auto-detection)
- **`SourceSystem::Other(String)` variant**: extension point for third-party source systems (`#[non_exhaustive]`)
- **Bundle exclusion of `redaction.aliases.json`**: alias cache is excluded from bundle manifests and zip archives to prevent redaction bypass
- **LLM feature gate**: `shiplog-cluster-llm` is now an optional dependency behind the `llm` feature, off by default; build with `--features llm` to include it
- Module-level documentation (`//!` doc blocks) for `shiplog-schema`, `shiplog-coverage`, `shiplog-workstreams`, `shiplog-engine`, and `shiplog-ports`
- CLI Reference section in README.md with full flag table
- LLM clustering flags (`--llm-cluster`, `--llm-api-endpoint`, `--llm-model`, `--llm-api-key`) documented across all 4 doc files
- Missing sections in GEMINI.md and copilot-instructions.md to sync with CLAUDE.md

### Changed

- Crate-specific descriptions for all 15 publishable crates (replacing generic workspace description)
- Added `keywords` and `categories` to all publishable crate Cargo.toml files
- Marked `shiplog-testkit` as `publish = false`
- `CACHE_FILENAME` in `shiplog-redact` is now `pub` for cross-crate documentation

## [0.1.1] - 2026-02-14

### Changed

- Refactored MarkdownRenderer for improved readability and consistency
- Enhanced documentation in CLAUDE.md with error handling, runtime, and output directory details
- Added package metadata (description, repository) for crates.io publishing
- Fixed internal crate dependencies to specify version requirements for publishing

## [0.1.0] - 2026-02-12

### Added

- **Core Ports and Traits** (`shiplog-ports`):
  - `Ingestor` trait for data collection adapters
  - `Renderer` trait for output format generation
  - `Redactor` trait for privacy-aware output filtering
  - `WorkstreamClusterer` trait for event clustering algorithms

- **GitHub Ingestor** (`shiplog-ingest-github`):
  - Fetch PRs and reviews from GitHub API
  - Adaptive date slicing to handle GitHub's 1000-result search cap
  - Support for both "merged" and "created" PR modes
  - Throttling support for rate limit compliance
  - GHES (GitHub Enterprise Server) support via custom API base
  - **SQLite caching** for PR details and reviews to reduce API calls

- **JSON Ingestor** (`shiplog-ingest-json`):
  - Import events from JSONL files
  - Coverage manifest validation

- **Manual Events** (`shiplog-ingest-manual`):
  - Track non-GitHub work (incidents, design docs, mentoring, launches, migrations)
  - YAML-based manual event definitions
  - Event type classification with emoji support

- **Local SQLite Cache** (`shiplog-cache`):
  - Durable caching for GitHub API responses
  - TTL-based expiration (default 24 hours)
  - Cache key builder for GitHub endpoints
  - In-memory cache support for testing
  - Cache statistics and cleanup utilities

- **Workstream Clustering** (`shiplog-workstreams`):
  - Repository-based automatic clustering
  - Curated workstreams via `workstreams.yaml`
  - Suggested workstreams generation (`workstreams.suggested.yaml`)
  - Persistent workstream management (user edits preserved)
  - Manager for curation workflow

- **Redaction System** (`shiplog-redact`):
  - Three redaction profiles: `internal`, `manager`, `public`
  - Deterministic keyed hash aliasing for repo names and workstream titles
  - Per-field redaction rules:
    - Public: strips titles, URLs, paths, descriptions
    - Manager: keeps titles/repos but removes sensitive details
    - Internal: no redaction
  - Property-based testing for leak detection

- **Markdown Renderer** (`shiplog-render-md`):
  - Self-review packet generation in Markdown
  - Coverage summary with completeness tracking
  - Event counts by type (PRs, reviews, manual)
  - Query slicing details and warnings
  - Receipt truncation with appendix for full listing
  - Claim scaffolds for narrative writing

- **JSON Renderer** (`shiplog-render-json`):
  - JSON output format for programmatic consumption

- **Bundle Format** (`shiplog-bundle`):
  - Zip archive generation for distribution
  - Manifest with integrity verification
  - Structured packet organization

- **Engine** (`shiplog-engine`):
  - `collect` command: Fetch events and generate workstream suggestions
  - `render` command: Regenerate packets from existing data
  - `refresh` command: Update events while preserving workstream curation
  - `run` command: Legacy combined collect+render mode

- **Schema** (`shiplog-schema`):
  - Event envelopes with unique IDs
  - Event types: PullRequest, Review, Manual
  - Coverage manifests with slicing metadata
  - Workstream definitions with receipts and stats
  - Manual event types and classification

- **IDs** (`shiplog-ids`):
  - Type-safe ID generation (EventId, RunId, WorkstreamId)
  - Timestamp-based run identifiers

- **Coverage** (`shiplog-coverage`):
  - Time window utilities (day, week, month windows)
  - Completeness tracking (Complete, Partial)
  - Coverage slicing for API cap handling

- **Testing** (`shiplog-testkit`):
  - Fixture generators for property-based tests
  - Redaction leak detection utilities

### Changed

- Enhanced `ApiCache` with `Clone` and `Debug` implementations
- Added `Serialize` derive to GitHub API response structs for cache storage
- Cleaned up all compiler warnings across the workspace

## [0.0.1] - Initial Release

### Added

- Initial project structure
- Basic workspace configuration with Cargo
- MIT/Apache-2.0 dual licensing

[Unreleased]: https://github.com/EffortlessMetrics/shiplog/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/EffortlessMetrics/shiplog/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/EffortlessMetrics/shiplog/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/EffortlessMetrics/shiplog/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/EffortlessMetrics/shiplog/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/EffortlessMetrics/shiplog/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/EffortlessMetrics/shiplog/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/EffortlessMetrics/shiplog/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/EffortlessMetrics/shiplog/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/EffortlessMetrics/shiplog/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/EffortlessMetrics/shiplog/compare/v0.0.1...v0.1.0
[0.0.1]: https://github.com/EffortlessMetrics/shiplog/releases/tag/v0.0.1
