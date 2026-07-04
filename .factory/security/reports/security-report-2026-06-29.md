# Security Scan Report

**Generated:** 2026-06-29
**Scan Type:** Weekly Scheduled
**Repository:** EffortlessMetrics/shiplog-swarm
**Severity Threshold:** medium

## Executive Summary

| Severity | Count | Auto-fixed | Manual Required |
|----------|-------|------------|-----------------|
| CRITICAL | 0 | 0 | 0 |
| HIGH     | 0 | 0 | 0 |
| MEDIUM   | 0 | 0 | 0 |
| LOW      | 0 | 0 | 0 |

**Total Findings:** 0
**Auto-fixed:** 0
**Manual Review Required:** 0

## Scan Results Overview

No security vulnerabilities were identified at or above the medium severity threshold in the scanned code.

### Commits Scanned (Last 7 Days)

The strict 7-day window (2026-06-22 through 2026-06-29) contains only one category of change in this repository: dependabot-driven GitHub Actions version bumps. All scanned commits are non-product code and were reviewed in addition to the most recent product-code change (`aebab19f`, 9 days old) because it is the only meaningful Rust change since the previous scan.

| Commit | Date | Author | Subject | Type |
|--------|------|--------|---------|------|
| `c19f7959` | 2026-06-25 | dependabot[bot] | ci: bump actions/checkout from 6.0.3 to 7.0.0 | CI only |
| `9994913c` | 2026-06-25 | dependabot[bot] | Merge 2c6990f7 into 1725b610 (dependabot PR #195) | Merge |
| `2c6990f7` | 2026-06-25 | dependabot[bot] | ci: bump actions/cache from 5 to 6 | CI only |
| `aebab19f`* | 2026-06-20 | Claude / Anthropic | feat: add sources status --json agent surface | Product (out of strict 7-day window, included for completeness) |

\* `aebab19f` was reviewed because no other Rust changes landed in the last two weekly scan windows. It is the latest substantive code change before the dependabot bumps.

### Files Inspected

**.github/workflows (CI-only, dependabot bumps):**
- `.github/workflows/bdd-smoke.yml`, `bdd-testing.yml`, `ci-actuals.yml`, `ci.yml`, `coverage.yml`, `droid-review.yml`, `droid-security-scan.yml`, `droid.yml`, `em-ci-routed-shiplog-rust.yml`, `fuzz-smoke.yml`, `fuzzing.yml`, `mutation-testing.yml`, `pr-plan.yml`, `property-smoke.yml`, `property-testing.yml`, `release.yml`, `ripr.yml`, `security.yml`
- Changes are limited to `actions/checkout@v6.0.3 -> @v7.0.0` and `actions/cache@v5 -> @v6` version pins; no logic, secret handling, permissions, or expression changes were introduced.

**Product code (aebab19f):**
- `apps/shiplog/src/doctor.rs` - adds `SourcesStatusView` struct and `build_sources_status_view` projection
- `apps/shiplog/src/commands/mod.rs` - extends dispatcher to pass through the new `--json` flag
- `apps/shiplog/src/main.rs` - adds `--json` flag to `SourcesStatusArgs` and conditional JSON serialization in `run_sources_status`
- `apps/shiplog/tests/cli_integration.rs` - new CLI integration test
- `CHANGELOG.md`, `docs/CURRENT_STATE.md`, `docs/guides/guided-setup-doctor.md`, `docs/specs/SHIPLOG-SPEC-0007-setup-readiness.md` - documentation

### Security Controls Verified

| Control | Status | Notes |
|---------|--------|-------|
| Secrets Management | PASS | No hardcoded secrets detected; uses environment variables (`GITHUB_TOKEN`, `GITLAB_TOKEN`, `JIRA_TOKEN`, `LINEAR_API_KEY`, `SHIPLOG_REDACT_KEY`) |
| SQL Injection | PASS | SQLite usage (cache module) is parameterized via `rusqlite::params!`; no string concatenation observed in scanned files |
| Command Injection | PASS | The only `Command::new` call sites in `apps/shiplog/src/main.rs` invoke `explorer.exe`, `open`, or `xdg-open` with hardcoded literals to open the in-tree `Rendered/index.html` after a successful run; no user-controlled arguments reach a shell or external process. No `std::process::Command` use exists in the product code touched by `aebab19f`. |
| Unsafe Code | PASS | No `unsafe` blocks exist anywhere in `apps/shiplog/src`, `apps/shiplog/tests`, or `crates/` |
| Input Validation | PASS | `anyhow::Result` with `.context(...)` is the consistent error propagation pattern; the new code follows it (`context("serialize sources status json")`) |
| Path Traversal | PASS | All file operations in the touched code paths use `Path::new(...).join(...)` and are bounded to caller-supplied CLI args or hardcoded paths |
| JSON Serialization Safety | PASS | Output uses `serde_json::to_writer_pretty(std::io::stdout(), &view)`. `SourcesStatusView` is a narrow projection derived from `SetupStatus` that **explicitly excludes** the `credentials`, `local_files`, `share_profiles`, and `next_actions` (full-list) fields from `SetupStatus`. The only credential-related strings it can produce are the existing env-var **names** (e.g. `"set GITHUB_TOKEN"`) sourced from the static setup catalog, never the token values. |
| Information Disclosure in New Surface | PASS | The CLI integration test asserts the JSON output does not contain `Manager share`, `SHIPLOG_REDACT_KEY`, or `redaction_key`, establishing a guardrail that the source-scoped projection cannot drift toward leaking credential/share metadata |
| Redaction | PASS | HMAC-SHA256 deterministic aliasing with three profiles remains unchanged; the new JSON surface is unrelated to redaction flows |
| Fuzzing | ACTIVE | Fuzz harnesses remain in `fuzz/` directory |
| Property Testing | ACTIVE | `proptest` configuration in `shiplog::redact` for redaction leak detection |
| Mutation Testing | ACTIVE | `cargo-mutants` configuration present |
| CI Supply Chain | PASS (improved) | Bumping `actions/checkout` to v7.0.0 and `actions/cache` to v6 updates to the Node.js 24 runtime; both are pinned to MAJOR tags rather than mutable `main` references, which is consistent with the existing supply-chain hardening (`deny.toml`, `dependabot.yml`, pinned third-party actions via SHA in `droid-security-scan.yml`) |
| Permissions Model | PASS | No workflow `permissions:` blocks were modified; all workflow files retain their existing permission scopes |

### STRIDE Threat Model Assessment

Based on the recently changed files plus the existing threat model:

| STRIDE Category | Assessment |
|-----------------|------------|
| Spoofing | LOW RISK - HMAC-SHA256 aliasing for user identities; tokens supplied via environment variables; no new identity surfaces introduced |
| Tampering | LOW RISK - Immutable event ledger with SHA256 `EventId`; content-addressed cache; the new `sources status --json` is a read-only projection and the integration test verifies it writes no run artifacts (compared `before`/`after` filesystem manifests) |
| Repudiation | LOW RISK - Receipts-first design with evidence traceability; output remains reproducible |
| Information Disclosure | LOW RISK - `SourcesStatusView` deliberately omits the credential and share-profile rows from `SetupStatus`; the integration test pins this property in CI; the JSON flag preserves the same exit-on-needs-action semantics, so a token-missing agent invocation still surfaces `needs_action: true` instead of leaking the token's status through other channels |
| Denial of Service | LOW RISK - Output size is bounded by the number of configured sources (typically <= 10); TTL-based cache expiry unchanged; no unbounded operations introduced |
| Elevation of Privilege | LOW RISK - No `unsafe`, no `eval`, no shell calls; the new code path executes inside the same clap-driven CLI process under the same OS user as before |

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
- Path: `.factory/threat-model.md`
- Last Modified: 2026-05-11
- Status: Reused (age: ~49 days, below the 90-day regeneration threshold)
- Note: Existing threat model and its STRIDE breakdown accurately cover the surfaces affected by this week's changes; no regeneration required.

### Scan Metadata
- Scan Window: 2026-06-22 to 2026-06-29 (strict 7 days). Adjacent commit `aebab19f` (2026-06-20) was also reviewed for completeness as it is the latest product-code change.
- Commits Scanned: 4 (`c19f7959`, `9994913c`, `2c6990f7`, `aebab19f*`)
- Files Touched: ~22 workflow YAML files (CI only) plus 4 Rust source files plus 1 integration test and 4 doc files
- Output Branch: `droid/security-report-2026-06-29`
- Report Path: `.factory/security/reports/security-report-2026-06-29.md`
- Skills Used (manual equivalents): commit-security-scan (manual code review against STRIDE), vulnerability-validation (manual reachability/exploitability check), security-review (no patches generated)
- Result: Clean scan, zero findings at or above the `medium` threshold

### Recommendations

1. **Keep merging dependabot PRs promptly**: The two dependabot bumps in this window already moved `actions/checkout` and `actions/cache` to their current major versions. Continuing to merge dependabot PRs on the existing cadence is the simplest way to keep CI supply-chain hygiene tight.
2. **Maintain the new `--json` guardrails**: The integration test in `aebab19f` asserts that `sources status --json` does not contain `Manager share`, `SHIPLOG_REDACT_KEY`, or `redaction_key`. If new fields are ever added to `SourcesStatusView`, extend that test before merging.
3. **Consider a future lane**: When the `claude/happy-lamport-2pl31g` branch lands on `main`, the next scheduled scan should re-verify the new JSON surface once it is reachable from `main` and review whether the broader `SourcesStatusView` shape (serialized `SetupItem` and `SetupNextAction`) still aligns with the threat model.

### References

- [CWE Database](https://cwe.mitre.org/)
- [STRIDE Threat Model](https://learn.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [actions/checkout releases](https://github.com/actions/checkout/releases) - v7.0.0 release notes
- [actions/cache releases](https://github.com/actions/cache/releases) - v6 release notes

---

*Report generated by Factory Droid (security-engineer plugin) for `EffortlessMetrics/shiplog-swarm`.*
