# Security Scan Report

**Generated:** 2026-06-08
**Scan Type:** Weekly Scheduled
**Repository:** EffortlessMetrics/shiplog
**Severity Threshold:** medium

## Executive Summary

| Severity | Count | Auto-fixed | Manual Required |
|----------|-------|------------|-----------------|
| CRITICAL | 0 | 0 | 0 |
| HIGH | 0 | 0 | 0 |
| MEDIUM | 0 | 0 | 0 |
| LOW | 0 | 0 | 0 |

**Total Findings:** 0
**Auto-fixed:** 0
**Manual Review Required:** 0

## Scan Results Overview

No security vulnerabilities were identified at or above the medium severity threshold in the scanned code.

### Files Scanned
- Last 7 days of commits (1 commit: `3a6abad` — merge of PR #607 promoting shiplog-swarm through 6f89913)
- Commit range: 1 merge commit containing promotion of development branch (shiplog-swarm) to main
- Changed files: ~300 files (predominantly workflow definitions, dependency policy, documentation, snapshot fixtures, and source modules consolidated under `apps/shiplog/src/`)

### Security Controls Verified

| Control | Status | Notes |
|---------|--------|-------|
| Secrets Management | PASS | No hardcoded secrets detected; all credentials (GITHUB_TOKEN, GITLAB_TOKEN, JIRA_TOKEN, LINEAR_API_KEY, SHIPLOG_REDACT_KEY) sourced from environment variables; LLM API key passed via CLI flag or `SHIPLOG_LLM_API_KEY` env var |
| SQL Injection | PASS | SQLite cache uses `params!` macro and `query_row` parameterized bindings; no string-concatenated SQL observed in `apps/shiplog/src/cache/sqlite.rs` |
| Command Injection | PASS | `std::process::Command` usage is confined to test harness files invoking the `shiplog` binary itself; no user-controlled shell execution in production code |
| Unsafe Code | PASS | `unsafe_code = "deny"` enforced at workspace level in `Cargo.toml`; `rg "unsafe\s*\{"` returns no matches in repository |
| TLS Validation | PASS | All `reqwest::blocking::Client::builder()` call sites use default TLS validation; no `danger_accept_invalid_certs`, `verify = false`, or `skip_tls_verify` matches in production source |
| Input Validation | PASS | `anyhow::Result<T>` with contextual `.context()` and `.with_context()` throughout; no bare `unwrap()` on user-influenced paths; manual/YAML inputs parsed via `serde_yaml_ng` with type validation |
| Path Traversal | PASS | File operations use `PathBuf`/`Path` and join user-supplied paths without `..` escape; cached path operations emit debug-formatted paths via `{path:?}` only in error context |
| Redaction | PASS | HMAC-SHA256 deterministic aliasing (RFC 2104) with 3 profiles (internal/manager/public); key derived from `SHIPLOG_REDACT_KEY` env var; alias cache file (`redaction.aliases.json`) version-checked |
| LLM API Auth | PASS | `cluster_llm/client.rs` uses `Authorization: Bearer {api_key}` header; no token logged in error paths (`unwrap_or_default()` masks response bodies); configurable timeout (default 60s) |
| Token Exposure in Diagnostics | PASS | `doctor.rs` only reports `env_var_present()` boolean for `GITHUB_TOKEN`, `GITLAB_TOKEN`, `JIRA_TOKEN`, `LINEAR_API_KEY`, `SHIPLOG_REDACT_KEY`; never logs the value itself |
| Workflow Integrity | PASS | All `actions/checkout` references pinned to commit SHA `df4cb1c069e1874edd31b4311f1884172cec0e10` (v6.0.3) or `@v6.0.3`; Droid action pinned to `EffortlessMetrics/droid-action-safe@7c1377ccbacddc95560d1570547a5baa51de01ec`; no `pull_request_target` triggers (avoids fork-PR pwn) |
| Trusted Actor Gate | PASS | `droid.yml` `@droid` invocations require `author_association` in {OWNER, MEMBER, COLLABORATOR} on issue comments, PR review comments, PR review submissions, issue bodies, and PR bodies |
| Workflow Permissions | PASS | Least-privilege `permissions:` blocks declared on every workflow job; `droid-security-scan` and `droid-review` use `contents: write` only when required, `pull-requests: write` for review publication, `id-token: write` for OIDC where needed |
| Fuzzing | ACTIVE | 15 fuzz harnesses in `fuzz/fuzz_targets/` covering config parsing, GitHub/GitLab/Jira/Linear API parsing, JSONL, manual events, plugin manifests, templates, workstream YAML roundtrip, workstream clustering/layout/receipt policy, redaction, schema deser, and markdown rendering; `fuzzing.yml` + `fuzz-smoke.yml` workflows active |
| Property Testing | ACTIVE | `proptest` configured in workspace deps; property tests in `shiplog::redact` for redaction leak detection; `property-testing.yml` + `property-smoke.yml` workflows active |
| Mutation Testing | ACTIVE | `.cargo/mutants.toml` configured; `mutation-testing.yml` workflow present |
| Dependency Policy | ACTIVE | `deny.toml` configured for advisories (RustSec), licenses (allowlist), bans, and sources (crates.io only); `security.yml` runs `cargo deny check` on push to main and weekly cron |
| Dependabot | ACTIVE | `.github/dependabot.yml` configured for `cargo` (weekly) and `github-actions` (weekly) ecosystems |

### STRIDE Threat Model Assessment

Based on analysis of the merge commit and existing threat model:

| STRIDE Category | Assessment |
|-----------------|------------|
| Spoofing | LOW RISK — HMAC-SHA256 aliasing for user identities; tokens from environment or CLI; Droid `@droid` commands restricted to trusted actors (OWNER/MEMBER/COLLABORATOR) |
| Tampering | LOW RISK — Immutable event ledger (`ledger.events.jsonl`) with SHA256 EventIds; content-addressed SQLite cache with parameterized queries; alias cache version-checked on load |
| Repudiation | LOW RISK — Receipts-first design with evidence traceability; coverage manifest explicitly reports missing slices rather than silently omitting data |
| Information Disclosure | LOW RISK — 3-tier redaction (internal/manager/public) with deterministic keyed hashing; doctor module reports only token presence (boolean) not values; LLM response body truncated to `text.unwrap_or_default()` in error path; no `unwrap()` on response payloads in `cluster_llm/client.rs` |
| Denial of Service | LOW RISK — TTL-based SQLite cache expiry (24h default, configurable); `throttle_ms` for upstream API calls; LLM request timeout (default 60s); `cache_cleanup_expired` and `cache_cleanup_older_than` available; receipt limits capped at 10 per workstream; chunked LLM event submission respects `max_input_tokens` budget |
| Elevation of Privilege | LOW RISK — `unsafe_code = "deny"` workspace lint; no `eval`-equivalent patterns; no `Command::new` in production source; no deserialization gadget surface (uses `serde_json`/`serde_yaml_ng` with typed schemas); no `pull_request_target` workflow triggers |

## Critical Findings

None.

## High Findings

None.

## Medium Findings

None.

## Low Findings

None.

## Appendix

### Threat Model
- Version: 2026-05-11 (last refreshed 2026-06-05 via PR #607 promotion; confirmed current at 2026-06-08)
- Location: `.factory/threat-model.md`
- Note: Existing threat model is current (< 90 days old); no regeneration required for this scan

### Scan Metadata
- Commits Scanned: 1
- Commit Type: Merge (PR #607 — swarm promotion through 6f89913)
- Files Changed: ~300 (predominantly workflows, policies, docs, snapshot fixtures, consolidated source modules)
- Scan Date: 2026-06-08
- Scan Window: 2026-06-01 → 2026-06-08 (7 days)
- Skills Used: commit-security-scan, vulnerability-validation
- Skills Not Invoked: threat-model-generation (model current), security-review (no findings to patch)

### Files Reviewed (Representative Sample)

Production source modules reviewed (under `apps/shiplog/src/`):
- `cache/sqlite.rs` — SQL parameterization verified
- `cache/expiry.rs`, `cache/key.rs`, `cache/stats.rs`, `cache/mod.rs`
- `ingest/github.rs`, `ingest/gitlab.rs`, `ingest/jira.rs`, `ingest/linear.rs`, `ingest/json.rs`, `ingest/git.rs`, `ingest/manual/events.rs`, `ingest/manual/mod.rs`
- `redact/alias.rs`, `redact/policy.rs`, `redact/profile.rs`, `redact/projector.rs`, `redact/repo.rs`, `redact/mod.rs`
- `cluster_llm/client.rs`, `cluster_llm/config.rs`, `cluster_llm/prompt.rs`, `cluster_llm/parse.rs`, `cluster_llm/parse/{claims,stats,workstreams}.rs`
- `team/core.rs`, `team/aggregate.rs`, `team/render.rs`, `team/template.rs`
- `workstreams/{cluster,layout,receipt_policy,mod}.rs`
- `merge/mod.rs`
- `render/md/{coverage,receipt,source,mod}.rs`
- `engine/{mod,artifact_json}.rs`
- `commands/{collect,refresh,run,import,merge,mod}.rs`
- `coverage/{mod,windows}.rs`
- `schema/{event,coverage,bundle,freshness,workstream,mod}.rs`
- `intake_report_builder.rs`, `doctor.rs`, `status.rs`, `ids.rs`, `ports.rs`, `github_activity.rs`, `lib.rs`, `main.rs`

CI/CD reviewed (under `.github/workflows/`):
- `droid.yml`, `droid-review.yml`, `droid-security-scan.yml`
- `security.yml`, `ci.yml`, `ci-actuals.yml`
- `release.yml`, `ripr.yml`, `em-ci-routed-shiplog-rust.yml`
- `fuzzing.yml`, `fuzz-smoke.yml`, `property-testing.yml`, `property-smoke.yml`
- `mutation-testing.yml`, `bdd-testing.yml`, `bdd-smoke.yml`, `coverage.yml`, `pr-plan.yml`

Policy/config reviewed:
- `deny.toml`, `.github/dependabot.yml`, `.github/settings.yml`, `Cargo.toml`, `.cargo/{config,mutants}.toml`

### Residual Risk Notes

- **External API trust:** GitHub, GitLab, Jira, Linear, and LLM providers remain trusted input sources. Rate-limit headers are sanitized into `GithubRateLimitSnapshot` and `GithubSecondaryLimitEvent` types; raw response bodies are not echoed into logs.
- **LLM clustering feature:** Off by default (feature-gated behind `llm` Cargo feature per CLAUDE.md). When enabled, sends summarized event metadata (PR number/title/additions/deletions/files, review state, manual event type/title) to a user-configured endpoint. Operators must trust their endpoint choice; no data exfiltration by default.
- **Self-hosted runner scope:** The `droid.yml`, `droid-review.yml`, `droid-security-scan.yml`, and several other workflows route to a self-hosted runner when `github.repository == 'EffortlessMetrics/shiplog-swarm'`. This is the expected swarm promotion model; the swarm repo's own security posture is the trust boundary.
- **Test-only Command usage:** Confined to invoking `CARGO_BIN_EXE_shiplog` and well-defined `git`/`bash` invocations with literal arguments; no user input flows into these calls.

### Recommendations (Non-Required)

1. **Continue maintaining fuzzing infrastructure.** The 15-harness fuzz surface (incl. `redact_event`, `workstream_cluster`, `render_md_packet`, `parse_*_api`) provides strong input-validation coverage; keep `fuzzing.yml` running on schedule and triage findings promptly.
2. **Keep dependabot noise low.** Both `cargo` and `github-actions` ecosystems are on weekly cadence; consider grouping minor/patch updates via Dependabot groups to reduce review burden.
3. **Maintain threat model freshness.** The threat model was last refreshed 2026-06-05 and remains current. Continue to update on architectural changes (new external integrations, new credential surfaces, new LLM endpoints).
4. **Monitor cargo-deny advisories.** The `security.yml` workflow surfaces RustSec advisories weekly; treat any new advisory as P0 until triaged.
5. **Consider broadening redaction fuzz coverage.** The `redact_event` fuzz harness is present; consider adding structured-property fuzzing for the `manager` profile (currently the `public` profile path is the most aggressive) to surface any field-leak regressions.

---

*Report generated by Factory Droid (security-engineer plugin)*
