# Security Scan Report

**Generated:** 2026-06-01
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
- Last 7 days of commits (64 commits, primarily merge-commit promotions from `EffortlessMetrics/shiplog-swarm` plus 1 dependabot commit)
- Most active areas: `xtask` (repo-native policy runner), `.github/workflows/*` (CI controls), `Cargo.toml`/`Cargo.lock` (dependabot), `fuzz/Cargo.lock` (fuzz infrastructure)
- Changed file surface is dominated by infrastructure, configuration, documentation, and developer tooling

### Commits Reviewed (sample)
- `5bd3aae` Merge pull request #562 from EffortlessMetrics/promote/swarm-20260601-e5f3e0a
- `e5f3e0a` xtask: report local merged branches (#126)
- `eb07299` Merge pull request #561 from EffortlessMetrics/promote/swarm-20260601-cb1e120
- `cb1e120` xtask: clarify pr-body receipt ordering (#125)
- `53faf29` Merge pull request #560 from EffortlessMetrics/promote/swarm-20260601-6f2c9d0
- `6f2c9d0` ci: switch Droid BYOK to MiniMax M3 (#124)
- `ef6f66f` deps: bump rusqlite from 0.39.0 to 0.40.0 (#95)
- `d2e192b` deps: bump reqwest from 0.13.3 to 0.13.4 (#94)
- `7415ab8` ci: keep promoted workflows source-safe (#97)
- `a5e3310` ci: refresh fuzz nightly for sqlite update (#98)

### Security Controls Verified

| Control | Status | Notes |
|---------|--------|-------|
| Secrets Management | PASS | No hardcoded secrets detected; `MINIMAX_API_KEY` and `FACTORY_API_KEY` referenced via `${{ secrets.X }}`; Droid config writes API key into `$HOME/.factory/settings.json` from secret-only env (not logged) |
| SQL Injection | PASS | rusqlite 0.40.0 retained; SQLite uses `params!` macro / parameterized queries throughout the cache module |
| Command Injection | PASS | `xtask/src/tasks/repo_contract_report.rs` uses `Command::new(...).args(&[...])`; args are hardcoded constants (`SWARM_REPO`, `SWARM_BRANCH`) or pre-validated strings — no shell concatenation or user input flows into argv |
| Unsafe Code | PASS | Workspace lints set `unsafe_code = "deny"`; new `xtask` modules contain no `unsafe` blocks |
| Path Traversal | PASS | `xtask/src/tasks/pr_body.rs` validates `work_item.plan` via `relative_repo_path` (rejects absolute paths and any `..`/`/`-prefixed components); `xtask` CLI tests use `tempfile::tempdir` for fixture isolation |
| Input Validation | PASS | `clap` derive args; `anyhow::Context` for typed error context; `parse_optional_u32` handles GitHub Actions empty-string env quirk |
| Redaction | PASS | HMAC-SHA256 deterministic aliasing; 3 profiles (internal/manager/public); no redaction key changes in scan window |
| Supply Chain (cargo-deny) | PASS | `cargo deny check advisories` clean; no security advisories; one informational warning about a transitive `aes 0.9.0` yanked crate (via `zip 8.6.0`) — not a vulnerability and predates the 7-day window |
| Workflow Hardening | PASS | Pinned third-party actions (`actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd`, `EffortlessMetrics/droid-action-safe@7c1377ccbacddc95560d1570547a5baa51de01ec`); least-privilege `permissions:` blocks; `concurrency:` guards; `if: env.FACTORY_API_KEY != '' && env.MINIMAX_API_KEY != ''` skip conditions; `upload_debug_artifacts: false` |
| Fuzzing | ACTIVE | Fuzz harnesses refreshed for rusqlite 0.40; pinned nightly `2026-05-20`; quick-fuzz is label-gated + main-only; extended-fuzz nightly 1h x 9 targets |
| Property Testing | ACTIVE | proptest runs against redact module (deterministic aliasing, leak detection) |
| Mutation Testing | ACTIVE | `cargo-mutants` configuration retained |

### STRIDE Threat Model Assessment

Based on manual code review of changed files against the existing threat model (`.factory/threat-model.md`, version 2026-05-11, 21 days old — within the 90-day freshness window):

| STRIDE Category | Assessment | Notes |
|-----------------|------------|-------|
| Spoofing | LOW RISK | Droid BYOK continues to use HMAC-authenticated MiniMax M3 endpoint at `api.minimax.io` (allowlisted `net-minimax`); token stored in repo secret and written to `$HOME/.factory/settings.json` only when the secret is present; `${{ github.event.X.author_association }}` check restricts `@droid` invocations to OWNER/MEMBER/COLLABORATOR |
| Tampering | LOW RISK | `xtask` changes operate on `policy/*.toml` and `.codex/goals/*.toml` but are read-only inspection + report generation; no write paths to the event ledger; workstream artifact IDs validated against `doc-artifacts.toml` ledger |
| Repudiation | LOW RISK | Receipts-first design intact; pr-body and repo-contract-report emit deterministic receipts tied to `EffortlessMetrics/shiplog-swarm#` and `EffortlessMetrics/shiplog#` PR refs |
| Information Disclosure | LOW RISK | Droid workflow `upload_debug_artifacts: false`; API keys are referenced via `${{ env.X }}` not bare `${{ secrets.X }}` so they do not appear in the run log; `ANTHROPIC_AUTH_TOKEN` and `ANTHROPIC_BASE_URL` are explicitly blanked to prevent the underlying model SDK from bypassing MiniMax BYOK |
| Denial of Service | LOW RISK | `concurrency:` blocks on every workflow; fuzzing has time caps (`-max_total_time=30` quick, `3600` extended); `xtask` reads TOML via `toml::from_str` (size-bounded by file system) |
| Elevation of Privilege | LOW RISK | No new `unsafe` blocks; `Command::new` invocations use argv arrays (not shell strings); no `set_var`, no `process::Command::new("sh"...)`; no new dependency graph changes privilege surface |

## Critical Findings

None.

## High Findings

None.

## Medium Findings

None.

## Low Findings

None.

## Notes for Reviewers

- The 7-day window is dominated by `xtask` enhancements (repo-contract-report, pr-body, closeout, hygiene reporting) and CI workflow routing. The `xtask` crate is a **developer-only** tool (not part of `apps/shiplog` and not user-reachable), so any issue there is bounded to repository maintainers and CI; the production attack surface (`shiplog` CLI, the `reqwest`-based GitHub ingest) was not modified in this window.
- `droid.yml` switched Droid's BYOK model from `MiniMax-M2.7` to `MiniMax-M3`. Both use the same `api.minimax.io` endpoint and the same secret allowlist (`net-minimax`), so this is a no-op from a network and authentication surface perspective. `ANTHROPIC_AUTH_TOKEN` and `ANTHROPIC_BASE_URL` are explicitly cleared to prevent the upstream SDK from falling back to a non-allowlisted provider.
- `rust 1.95.0` toolchain pins are unchanged; `cargo-deny` advisories are clean; one informational warning about a transitive `aes 0.9.0` (yanked, pulled in by `zip 8.6.0` -> `shiplog-testkit`) is not a security advisory and predates the 7-day window.

## Appendix

### Threat Model
- Version: 2026-05-11 (21 days old — fresh, no regeneration required)
- Location: `.factory/threat-model.md`

### Scan Metadata
- Commits Scanned: 64 (last 7 days, includes 32 promotion merges + 32 swarm-side feature commits)
- Files Touched: ~50 unique files (xtask Rust code, CI workflows, policy TOML, docs, scripts, lockfiles)
- Build Status: `cargo build --workspace` PASS
- Lint Status: `cargo clippy --workspace --all-targets --all-features -- -D warnings` PASS
- Supply-chain Status: `cargo deny check` PASS (no advisories; one transitive-yanked informational warning)
- Scan Duration: ~5 minutes
- Skills Used: threat-model-generation (no-op — model fresh), commit-security-scan, vulnerability-validation, security-review

### References
- [CWE Database](https://cwe.mitre.org/)
- [STRIDE Threat Model](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [Rust Security Advisory Database](https://rustsec.org/)

---

*Report generated by Factory Droid (security-engineer plugin) on 2026-06-01.*
