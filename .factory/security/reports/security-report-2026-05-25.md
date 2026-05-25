# Security Scan Report

**Generated:** 2026-05-25
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

No actionable findings emitted.

---

## Inspection Record

**Inspected surfaces:**
- `apps/shiplog/src/cache/sqlite.rs` - SQLite cache with parameterized queries
- `apps/shiplog/src/redact/alias.rs` - HMAC-SHA256 alias generation
- `apps/shiplog/src/redact/mod.rs` - Redaction policy implementation
- `apps/shiplog/src/ingest/github.rs` - GitHub API integration with cache
- `apps/shiplog/src/ingest/gitlab.rs` - GitLab API integration
- `apps/shiplog/src/ingest/json.rs` - JSONL/JSON ingestion adapter
- `apps/shiplog/src/ingest/git.rs` - Local git repository ingestor
- `apps/shiplog/src/cluster_llm/client.rs` - LLM API client (OpenAI-compatible)
- `apps/shiplog/src/cluster_llm/config.rs` - LLM configuration

**Checks performed:**
- STRIDE analysis (Spoofing, Tampering, Repudiation, Information Disclosure, Denial of Service, Elevation of Privilege)
- SQL injection vulnerability check in cache operations
- Command injection check in process execution
- Hardcoded secret detection
- Authentication token handling
- Input validation on API responses

**Why no comments:**
- All cache operations use `rusqlite::params![]` macro for parameterized queries, preventing SQL injection
- HMAC-SHA256 implementation follows RFC 2104 correctly with proper key padding
- GitHub/GitLab API clients use proper header construction without string concatenation
- LLM API key is loaded from configuration, not hardcoded in source
- Local git ingestor uses the `git2` crate safely without shell command execution
- Process execution in main.rs uses hardcoded application launchers (explorer.exe, open, xdg-open) with fixed arguments, not user-controlled command injection
- JSON deserialization uses `serde_json` which is safe from injection attacks

**Residual risk:** Low - codebase follows secure coding practices with proper input validation, parameterized queries, and no eval/shell execution.

**Validation signal:**
- Observed: All reviewed code paths use safe APIs
- Reported: No security vulnerabilities detected
- Not verified: N/A - no findings to verify

---

## Appendix

### Threat Model
- Version: 2026-05-11 (last updated 2026-05-25)
- Location: .factory/threat-model.md

### Scan Metadata
- Commits Scanned: 1 (40a3ce6a9cd4ea2dbd90918cfc24584a2aca968c)
- Files Changed: 200+ (full repository)
- Skills Used: Manual security review (no automated findings)

### References
- [CWE Database](https://cwe.mitre.org/)
- [STRIDE Threat Model](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
