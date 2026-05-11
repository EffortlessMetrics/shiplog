# Threat Model - shiplog

## System Overview

shiplog is a Rust-based CLI tool for generating changelogs and engineering metrics from various data sources (GitHub, GitLab, Jira, Linear, manual events).

## Architecture Layers

1. **CLI Layer** (apps/shiplog)
2. **Engine Layer** (shiplog-engine) - Orchestration
3. **Adapter Layer** (ingest adapters: github, gitlab, jira, linear, json, manual)
4. **Foundation Layer** (schema, ports, ids, coverage, cache, redact, render-md, render-json)
5. **Utilities** (testkit, bundle)

## Trust Boundaries

1. External APIs (GitHub, GitLab, Jira, Linear) - Untrusted input
2. Local filesystem - User-controlled files
3. Configuration files - User-controlled
4. Manual event YAML/JSON files - User-controlled

## STRIDE Analysis

### Spoofing
- **Threat**: Attacker impersonates another user in commit/PR data
- **Mitigation**: HMAC-SHA256 aliasing for user identities
- **Severity**: Medium

### Tampering
- **Threat**: Modify cached API responses to alter output
- **Mitigation**: SQLite database isolation, content-addressed storage
- **Severity**: Low

### Repudiation
- **Threat**: User denies making changes captured by the tool
- **Mitigation**: Immutable ledger.events.jsonl with SHA256 EventIds
- **Severity**: Low

### Information Disclosure
- **Threat**: Leak sensitive info (token, email, private repo names)
- **Mitigation**: 3-tier redaction profiles (internal/manager/public)
- **Severity**: High

### Denial of Service
- **Threat**: Exhaust disk space with unbounded cache
- **Mitigation**: TTL-based expiry, max_size configuration
- **Severity**: Low

### Elevation of Privilege
- **Threat**: Execute arbitrary code via malicious config
- **Mitigation**: No eval, no unsafe code, sandboxed adapters
- **Severity**: Low

## Key Security Features

1. **Deterministic Redaction**: Same input + key = same alias via HMAC-SHA256
2. **Receipts-First**: Every claim traces to fetched evidence
3. **Immutable Ledger**: ledger.events.jsonl is append-only
4. **Cache Expiry**: Automatic TTL-based cleanup
5. **Input Validation**: Comprehensive fuzzing and property testing

## Data Flow

1. **Ingest**: Events fetched from external APIs or local files
2. **Cache**: API responses cached in SQLite with TTL
3. **Cluster**: Events grouped into workstreams via user-curated YAML
4. **Redact**: Sensitive data masked via HMAC-SHA256 aliasing
5. **Render**: Output generated as markdown or JSON

## External Dependencies

- rusqlite (SQLite bindings)
- reqwest (HTTP client for API calls)
- serde (serialization)
- chrono (date/time handling)
- tokio (async runtime, feature-gated)

## Security Controls

- Fuzzing harnesses in fuzz/ directory
- Property-based tests via proptest
- Mutation testing via cargo-mutants
- Clippy linting with strict warnings
- No unsafe code blocks allowed
- Parameterized SQL queries (no string concatenation)

---

*Generated: 2026-05-11*
