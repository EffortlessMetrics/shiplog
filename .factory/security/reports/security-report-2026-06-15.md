# Security Scan Report

**Generated:** 2026-06-15
**Scan Type:** Weekly Scheduled
**Repository:** EffortlessMetrics/shiplog
**Severity Threshold:** medium

## Executive Summary

| Severity | Count | Auto-fixed | Manual Required |
|----------|-------|------------|-----------------|
| CRITICAL | 0     | 0          | 0               |
| HIGH     | 0     | 0          | 0               |
| MEDIUM   | 0     | 0          | 0               |
| LOW      | 0     | 0          | 0               |

**Total Findings:** 0
**Auto-fixed:** 0
**Manual Review Required:** 0

## Scan Results Overview

No security vulnerabilities were identified at or above the medium severity
threshold in the scanned code.

### Files Scanned

- Last 7 days of commit activity. The repository's last commit on
  `main` (`3a6abad`) was authored 2026-06-05, ten days before this scan, and
  is the only change in the 30-day window. It is a single merge commit
  promoting `shiplog-swarm` through `6f89913`.
- Commit: `3a6abad` - merge(swarm): promote shiplog-swarm through 6f89913
- Changed files: extensive; primary focus was on the Rust production code
  introduced by the merge (cache, redact, cluster_llm, ingest, bundle,
  render, manual ingest, workstreams, redact).
- Changed files are primarily code, configuration, and documentation. No
  third-party binary updates, secrets, or workflow permission changes.

### Security Controls Verified

| Control | Status | Notes |
|---------|--------|-------|
| Secrets Management | PASS | No hardcoded secrets detected; tokens come from environment variables (`GITHUB_TOKEN`, `GITLAB_TOKEN`, `JIRA_TOKEN`, `LINEAR_API_KEY`, `SHIPLOG_REDACT_KEY`, `SHIPLOG_LLM_API_KEY`). API keys are read via `with_api_key`/env-var resolution. |
| SQL Injection | PASS | All SQLite queries in `apps/shiplog/src/cache/sqlite.rs` use the `params!` macro with positional placeholders. No string concatenation. |
| Command Injection | PASS | Only `std::process::Command::new` usages are `explorer.exe`/`open`/`xdg-open` for opening a local file with `Command::arg(&Path)`. Argument form is safe from shell metacharacter injection; the path is canonicalized and verified to exist before invocation. |
| Unsafe Code | PASS | No `unsafe` blocks anywhere under `apps/shiplog/src` (`Grep "unsafe"` returned no matches). |
| HTTP / TLS | PASS | `reqwest::blocking::Client::builder()` is used without `danger_accept_invalid_certs`, no cert-skip flags. `https://` is hardcoded for hosted integrations and the user-supplied `--api-base`/`--llm-api-endpoint` defaults to HTTPS. `api_url`/`html_base_url` do not silently downgrade. |
| Input Validation | PASS | Errors propagate via `anyhow::Context` with descriptive `.with_context`. URL inputs are parsed with `url::Url::parse` and rejected on failure. Out-of-bounds LLM response indices are filtered. |
| Path Traversal | PASS | Bundle writer uses `path.strip_prefix(out_dir)` and relative path construction; the alias cache joins a constant filename into a user-supplied out directory. The CLI's `try_open_path` calls `canonicalize` before launching the OS handler. |
| Zip Slip | PASS | `bundle::write_zip` writes entries using paths produced by `strip_prefix(out_dir)`; entries cannot escape the run directory. |
| Redaction | PASS | `DeterministicRedactor` is HMAC-SHA256 keyed (RFC 2104) and re-uses the resulting aliases for stability. Public profile strips PR titles, opaque IDs, source URLs, and link lists. Manager profile strips touched paths and link lists. |
| Caching | PASS | `ApiCache` enforces a 24-hour default TTL, supports per-call TTLs, and exposes `cleanup_expired`/`cleanup_older_than`. Read-only mode exists for diagnostic introspection. |
| Replay / Tamper | PASS | `bundle.manifest.json` carries SHA-256 checksums for every artifact included in a profile. `redaction.aliases.json` is explicitly excluded from bundles to avoid leaking the alias map. |
| LLM Backend | PASS | The `OpenAiCompatibleBackend` uses an explicit `timeout` on the `reqwest` client and a fixed request shape (`response_format: json_object`); the user controls the endpoint but TLS validation is the default. |
| Fuzzing | ACTIVE | Fuzz harnesses present in `fuzz/` directory. |
| Property Testing | ACTIVE | `proptest` is used heavily in cache, redact, ingest, and URL builders. |
| Mutation Testing | ACTIVE | `cargo-mutants` configuration present. |
| Dependency Audit | ACTIVE | `security.yml` runs `cargo deny check advisories` weekly. |

### STRIDE Threat Model Assessment

| STRIDE Category | Assessment |
|-----------------|------------|
| Spoofing | LOW RISK - All remote APIs authenticate via per-provider bearer tokens sourced from the environment. `redact::alias` derives a stable HMAC-SHA256 alias keyed on the operator-supplied redaction key; spoofing aliases requires guessing the key plus the kind/value pair. |
| Tampering | LOW RISK - The ledger is append-only JSONL; the bundle manifest carries SHA-256 checksums for every emitted artifact. `bundle.manifest.json` and `redaction.aliases.json` are excluded from bundles to prevent self-referential checksums and to keep the alias map private. |
| Repudiation | LOW RISK - Receipts-first design: every claim in a packet points back to a fetched event with stable `EventId`s, and `coverage.manifest.json` is mandatory in the output directory. |
| Information Disclosure | LOW RISK - Three structural redaction profiles (internal/manager/public) with documented removals (titles, opaque IDs, source URLs, link lists, manual `description`/`impact`, touched paths, repo URL in public). The `cache_entries` SQLite file lives under the run directory and is per-run, not shared. |
| Denial of Service | LOW RISK - The GitHub ingestor has a hard-coded 1000-result cap (`max_pages = 10 * per_page = 100`), TTL-based cache expiry, `max_size_bytes` knob, and an explicit `api_budget` (`max_search_requests` / `max_core_requests`). The LLM backend enforces a per-call `timeout_secs` (default 60). |
| Elevation of Privilege | LOW RISK - No `unsafe` blocks, no shell exec, no dynamic loading, no plugin/eval paths. Aliases are HMAC-derived rather than user-supplied, and the redaction profile parser defaults unknown values to `Public` (most restrictive) rather than `Internal`. |

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
- Version: 2026-05-11
- Location: .factory/threat-model.md
- Note: Existing threat model is current (35 days old, well within the 90-day
  refresh window). No regeneration required for this scan.

### Scan Metadata
- Commits Scanned: 1 (the 7-day and 14-day windows each return the same
  single commit; the 30-day window also returns only this commit)
- Commit: `3a6abad` - merge(swarm): promote shiplog-swarm through 6f89913
- Primary surface reviewed:
  - `apps/shiplog/src/cache/sqlite.rs` (SQL)
  - `apps/shiplog/src/cluster_llm/{client,config,mod,parse,prompt}.rs` (LLM)
  - `apps/shiplog/src/redact/{alias,mod,policy,profile}.rs` (redaction)
  - `apps/shiplog/src/bundle/{layout,mod}.rs` (zip + manifest)
  - `apps/shiplog/src/ingest/{github,gitlab,jira,linear,git,manual,manual/events,manual/mod,json}.rs` (HTTP)
  - `apps/shiplog/src/ingest/github.rs` `client()` / `api_url()` /
    `get_json()` paths
  - `apps/shiplog/src/workstreams/{cluster,layout}.rs` (YAML I/O)
  - `apps/shiplog/src/main.rs` `try_open_path`/`open_or_print_path`
- Scan Duration: ~10 minutes
- Build verification: `cargo build --workspace` finished successfully
  (`Finished 'dev' profile [optimized + debuginfo]`)
- Skills Used: threat-model-generation (no-op, model current),
  commit-security-scan, vulnerability-validation, security-review

### Recommendations

1. **Continue weekly cadence** - The `droid-security-scan.yml` workflow
   already runs Monday 08:00 UTC; keep it that way. The repository is
   currently in a low-finding steady state.
2. **Track alias cache key rotation** - `redact::alias` is correct under a
   stable key. When rotating `SHIPLOG_REDACT_KEY`, treat
   `redaction.aliases.json` as cache, not as the source of truth; the
   parser now bails on `version != 1`, so a manual delete is the safe
   path during rotation.
3. **Watch `--api-base` / `--llm-api-endpoint` changes** - These are
   stringly-typed. A future enhancement could refuse non-`https://`
   schemes, but the current default is HTTPS and the field is
   operator-controlled.
4. **Keep `cargo deny` on main + weekly cron** - `security.yml` already
   does this; no change required.

### References

- [CWE Database](https://cwe.mitre.org/)
- [STRIDE Threat Model](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [RFC 2104 - HMAC: Keyed-Hashing for Message Authentication](https://datatracker.ietf.org/doc/html/rfc2104)
- [Rust Security Advisory Database](https://rustsec.org/)

---

*Report generated by Factory Droid (security-engineer plugin)*
