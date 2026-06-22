# Security Scan Report

**Generated:** 2026-06-22
**Scan Type:** Weekly Scheduled
**Repository:** EffortlessMetrics/shiplog
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

No security vulnerabilities were identified at or above the medium severity threshold in the scanned code. The week's diff window contains documentation polish, test-assertion tightening, a routine GitHub Actions dependency bump, and lockfile patch updates for transitive dependencies.

### Files Scanned (last 7 days)

| Commit    | Type   | Description |
|-----------|--------|-------------|
| `6c8f844` | non-merge | docs: carry latest swarm promotion receipts with release polish |
| `758e2dc` | merge | Merge pull request #615 from EffortlessMetrics/docs/release-smoke-receipts-20260616 |
| `383ab5f` | merge | Merge pull request #616 from EffortlessMetrics/promote/swarm-20260616-1725b610 |

Files touched (unique paths across the window):

| File | Change Type | Risk Surface |
|------|-------------|--------------|
| `.codex/goals/active.toml` | receipt PR list extension | none (text only) |
| `apps/shiplog/tests/docs_commands.rs` | assertion expansion in `install_guide_documents_current_install_paths` | none (test only) |
| `docs/install.md` | smoke-test command list refresh and clarifying prose | none (docs only) |
| `plans/shiplog-swarm/implementation-plan.md` | promotion receipts narrative append | none (docs only) |
| `.github/workflows/coverage.yml` | `codecov/codecov-action` bumped from `@v6` to `@v7` | supply-chain pin style (see Notes) |
| `Cargo.lock` | patch-level dependency bumps: `chrono 0.4.44` to `0.4.45`, `hashlink 0.11.0` to `0.12.0`, `libsqlite3-sys 0.38.0` to `0.38.1`, `regex 1.12.3` to `1.12.4`, `regex-syntax 0.8.10` to `0.8.11`, `rusqlite 0.40.0` to `0.40.1` | dependency surface (see Notes) |

### Security Controls Verified

| Control | Status | Notes |
|---------|--------|-------|
| Secrets Management | PASS | No hardcoded secrets introduced; same env-var surface (`GITHUB_TOKEN`, `GITLAB_TOKEN`, `JIRA_TOKEN`, `LINEAR_API_KEY`, `SHIPLOG_REDACT_KEY`); `policy/workflow-allowlist.toml` continues to enumerate `CODECOV_TOKEN` only as the secret referenced by `coverage.yml` |
| SQL Injection | PASS | No SQL-touching code changed; rusqlite usage remains parameterised via `params!` and prepared statements |
| Command Injection | PASS | No new `Command::new`, `tokio::process`, or shell-substitution call sites were introduced. The only existing `Command::new` sites remain `explorer.exe`, `open`, and `xdg-open` in `apps/shiplog/src/main.rs` `try_open_path`, all invoked with a single `Path` argument and detached stdio |
| Unsafe Code | PASS | `unsafe_code = "deny"` workspace lint remains in effect; no `unsafe` block added in the window |
| Input Validation | PASS | anyhow `Result<T>` flow unchanged; no new entry points added |
| Path Traversal | PASS | No new filesystem calls; install docs reference only canonical README paths |
| Redaction | PASS | HMAC-SHA256 deterministic aliasing with three profiles (`internal`, `manager`, `public`) unchanged; no schema drift |
| Action Pinning | PASS (consistent) | Coverage workflow uses `codecov/codecov-action@v7`, a moving tag. This matches the surrounding style in the same file (`actions/checkout@v6.0.3`, `Swatinem/rust-cache@v2`, `taiki-e/install-action@v2`, `actions/upload-artifact@v7`); only `droid-security-scan.yml` and `droid.yml` use full-SHA pins for high-trust actions |
| Dependency Advisories | PASS | All `Cargo.lock` bumps are patch-level. None of the changed crates (`chrono 0.4.45`, `hashlink 0.12.0`, `libsqlite3-sys 0.38.1`, `regex 1.12.4`, `regex-syntax 0.8.11`, `rusqlite 0.40.1`) have a published RustSec advisory matching the new version. The repository's `deny.toml` continues to pull from `https://github.com/rustsec/advisory-db` and runs on push-to-main plus a weekly cron via `.github/workflows/security.yml` |
| Fuzzing | ACTIVE | Fuzz harnesses unchanged (`fuzz/fuzz_targets/...`) |
| Property Testing | ACTIVE | `proptest` redaction leak tests unchanged |
| Mutation Testing | ACTIVE | `.cargo/mutants.toml` unchanged; weekly `mutation-testing.yml` workflow |

### STRIDE Threat Model Assessment

Applied against `.factory/threat-model.md` (version 2026-05-11, 42 days old, within the 90-day refresh window):

| STRIDE Category | Assessment |
|-----------------|------------|
| Spoofing | LOW RISK — no auth/identity surface in this window. Tokens are still read from the environment and never written to repo files |
| Tampering | LOW RISK — no ledger, cache, or hash path was modified. `ledger.events.jsonl` append-only contract and `EventId = sha256(...)` derivation are untouched |
| Repudiation | LOW RISK — receipts-first design intact. `.codex/goals/active.toml` and `plans/shiplog-swarm/implementation-plan.md` only added cross-reference pointers to existing PR receipts |
| Information Disclosure | LOW RISK — deterministic redaction profiles unchanged; install smoke test now explicitly enumerates help-only commands and notes that the smoke path must not require provider tokens, write intake evidence, or render share artifacts, reinforcing the least-privilege boundary |
| Denial of Service | LOW RISK — no new loops, fan-out, or unbounded resource use. The help-only smoke test path consumes no network or disk beyond `shiplog init --dry-run` |
| Elevation of Privilege | LOW RISK — no new exec or eval surface; `unsafe_code = "deny"` and `unexpected_cfgs = "warn"` lint floors unchanged |

## Critical Findings

None.

## High Findings

None.

## Medium Findings

None.

## Low Findings

None.

## Notes (Informational, Below Threshold)

These are observations, not findings. They do not meet the medium severity threshold but are recorded for the next report to pick up if they recur.

1. **Policy drift in `policy/workflow-allowlist.toml`**: the allowlist entry for `.github/workflows/coverage.yml` still lists `codecov/codecov-action@v6`, while the workflow was bumped to `@v7`. This is a documentation/policy drift, not a runtime vulnerability; the action bump itself is a routine supply-chain update from the action publisher. Recommendation: update the allowlist entry to `@v7` in a follow-up PR so CI policy enforcement matches reality.
2. **Tag-pinned GitHub Actions**: actions used by the coverage workflow continue to be referenced by mutable tags (`@v7`, `@v2`, etc.). This is consistent with the prevailing style across most workflows in the repository. Only security-sensitive actions in `droid.yml` and `droid-security-scan.yml` are SHA-pinned. Recommendation: track upstream guidance and consider tightening pin style as a separate hygiene PR; not blocking.

## Appendix

### Threat Model
- Version: 2026-05-11
- Location: `.factory/threat-model.md`
- Age: 42 days (within the 90-day refresh window); no regeneration performed this cycle

### Scan Metadata
- Window: last 7 days (2026-06-15 to 2026-06-22, UTC)
- Commits Scanned: 3 (1 non-merge, 2 merge)
- Files Touched: 6
- Scan Duration: ~5 minutes
- Skills Used: commit-security-scan, vulnerability-validation (manual STRIDE pass over changed files)
- Scanner: Factory Droid (security-engineer plugin)

### References
- [CWE Database](https://cwe.mitre.org/)
- [STRIDE Threat Model](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [RustSec Advisory DB](https://github.com/rustsec/advisory-db)

---

*Report generated by Factory Droid (security-engineer plugin).*
