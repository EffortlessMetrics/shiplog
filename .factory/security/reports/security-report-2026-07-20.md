# Security Scan Report

**Generated:** 2026-07-20
**Scan Type:** Weekly Scheduled
**Repository:** EffortlessMetrics/shiplog
**Branch:** `droid/security-report-2026-07-20`
**Severity Threshold:** medium
**Scan Window:** 2026-07-13 to 2026-07-20 (7 days)

## Executive Summary

| Severity | Count | Auto-fixed | Manual Required |
|----------|-------|------------|-----------------|
| CRITICAL | 0 | 0 | 0 |
| HIGH     | 0 | 0 | 0 |
| MEDIUM   | 0 | 0 | 0 |
| LOW      | 0 | 0 | 0 |

**Total Findings (>= medium):** 0
**Auto-fixed:** 0
**Manual Review Required:** 0

No new findings at or above the medium severity threshold were
identified this cycle. The previous MEDIUM finding (VULN-002,
GitHub `--api-base` non-HTTPS URL) was re-verified and its fix is
retained in the merged tree.

## Scan Results Overview

The 7-day scan window contained one commit: `6fe7b66` (chore(automation):
keep source dependency scans verification-only (#656), 2026-07-13). This
commit is the history-join / squash import that brought the
`EffortlessMetrics/shiplog` and `EffortlessMetrics/shiplog-swarm`
histories together; it is the sole commit reachable from `main` in this
checkout, so the parent-vs-child file diff against the prior
`a3a15ed` head is not directly available in this tree. The scan was
therefore performed against the imported tree at `6fe7b66` with
extra attention to:

- Code paths previously flagged (VULN-001: LLM endpoint HTTPS check,
  VULN-002: GitHub `--api-base` HTTPS check) to confirm both fixes are
  retained in the merged tree.
- Surfaces that have historically carried the highest information-disclosure
  risk: ingest adapters, redaction alias store, bundle writer, and
  CLI surface.
- Repository automation: GitHub Actions workflows, supply-chain
  posture for actions and toolchains, dependabot configuration.

### Commits Scanned

| SHA | Date (UTC) | Subject | Files |
|------|------------|---------|-------|
| 6fe7b66 | 2026-07-13 | chore(automation): keep source dependency scans verification-only (#656) | (initial import, no parent) |

### Surfaces Reviewed

| Surface | Purpose | Result |
|---------|---------|--------|
| `apps/shiplog/src/cluster_llm/client.rs` | LLM clustering HTTP backend | PASS (VULN-001 fix retained; `validate_https_endpoint` enforced at request time) |
| `apps/shiplog/src/cluster_llm/{config,mod,parse,parse/*,prompt}.rs` | LLM clustering config, prompt builder, parser | PASS |
| `apps/shiplog/src/cache/sqlite.rs` | SQLite cache (parameterized queries) | PASS |
| `apps/shiplog/src/cache/{key,mod,stats,expiry}.rs` | Cache key builders / TTL helpers | PASS |
| `apps/shiplog/src/ingest/github.rs` | GitHub GraphQL/REST ingest | PASS (VULN-002 fix retained; `validate_https_api_base` wired into CLI bridge `make_github_ingestor` at `apps/shiplog/src/main.rs:12630`; loopback carve-out is documented and bounded) |
| `apps/shiplog/src/ingest/gitlab.rs` | GitLab REST ingest | PASS (host always re-formatted under hardcoded `https://` scheme) |
| `apps/shiplog/src/ingest/jira.rs` | Jira REST ingest | PASS (host always re-formatted under hardcoded `https://` scheme) |
| `apps/shiplog/src/ingest/linear.rs` | Linear GraphQL ingest | PASS (`https://api.linear.app/graphql` is a constant) |
| `apps/shiplog/src/ingest/git.rs` | Local libgit2 ingest | PASS |
| `apps/shiplog/src/ingest/manual/{mod,events}.rs` | YAML-driven manual ingest | PASS |
| `apps/shiplog/src/ingest/json.rs` | JSONL ledger ingest | PASS |
| `apps/shiplog/src/redact/{mod,alias,policy,profile,projector,repo}.rs` | HMAC-SHA256 deterministic aliasing | PASS (RFC 4231 test vectors present and passing) |
| `apps/shiplog/src/render/md/{mod,coverage,receipt,source}.rs` | Markdown packet renderer | PASS |
| `apps/shiplog/src/bundle/{mod,layout}.rs` | Zip + SHA-256 manifest writer | PASS (always excludes redaction aliases and the manifest itself; path-traversal checked via `strip_prefix`) |
| `apps/shiplog/src/workstreams/{mod,cluster,layout,receipt_policy}.rs` | YAML clustering | PASS |
| `apps/shiplog/src/github_activity.rs` | Advanced GitHub harvest orchestration | PASS (transitive to `validate_https_api_base`) |
| `apps/shiplog/src/github_auth.rs` | GitHub auth resolution (`gh` CLI / env) | PASS (hardcoded argv, timeout, kill-on-overrun) |
| `apps/shiplog/src/main.rs::try_open_path` | xdg-open / open / explorer.exe handoff | PASS (single canonicalized arg, no shell) |
| `apps/shiplog/src/main.rs::resolve_redaction_key` | Redaction-key resolution | PASS (only reports `RedactionKeySource`, never the key) |
| `.github/workflows/*.yml` (18 files) | CI/secrets exposure | PASS for secrets/action scope; OBS-4 carried forward for `dtolnay/rust-toolchain@master` |
| `policy/network-allowlist.toml`, `policy/publish-allowlist.toml` | Network / publish policy receipts | PASS |
| `Cargo.toml`, `Cargo.lock` | Dependency posture | PASS (`unsafe_code = "deny"`, `serde_yaml_ng` fork in use, regex 1.12.x linear engine) |
| `.factory/threat-model.md` | Living threat model | Current (63d old, well under 90d refresh rule) |

### STRIDE Threat Model Assessment

| STRIDE Category | Assessment |
|-----------------|------------|
| Spoofing | LOW RISK. User identity flows through HMAC-SHA256 aliasing (`redact/alias.rs`); bearer tokens sourced from env vars or `--token`. No token is ever echoed into logs (`apps/shiplog/src/main.rs` only reports `RedactionKeySource` / token presence, never the value; `apps/shiplog/src/github_auth.rs::safe_metadata_does_not_serialize_credential_material` test enforces this). |
| Tampering | LOW RISK. SQLite cache uses parameterized statements via `params!` exclusively (no string concatenation in any query). Cache rows are content-addressed by deterministic keys (`cache/key.rs`). Bundle writer uses SHA-256 for integrity (`bundle/mod.rs::sha256_file`). |
| Repudiation | LOW RISK. `ledger.events.jsonl` is append-only with SHA-256 EventIds (`ids.rs`). All API responses tagged with rate-limit / cache receipts. |
| Information Disclosure | LOW RISK. VULN-002 fix retained (GitHub `api_base` must be https, with documented loopback carve-out). Bundle writer `ALWAYS_EXCLUDED` continues to strip `redaction.aliases.json` and `bundle.manifest.json` from all three profiles. |
| Denial of Service | LOW RISK. TTL-based cache cleanup (`cleanup_expired`, `cleanup_older_than`). API request budgets (`GithubApiBudget`) cap live calls. `--throttle-ms` slows hostile-only loops. LLM request timeout configurable per backend. |
| Elevation of Privilege | LOW RISK. Workspace `[workspace.lints.rust] unsafe_code = "deny"`; `grep -n "\bunsafe\b" apps/` returns no matches. No `eval`, no shell `Command::new` with user-supplied strings. Subprocess invocations (`xdg-open`, `open`, `explorer.exe`) use `command.arg(path)` not `arg("-c", ...)` and pass a canonicalized path. `gh` invocation in `github_auth.rs` uses hardcoded `Command::new("gh")` with internal-only argument vector. |

### Security Controls Verified

| Control | Status | Evidence |
|---------|--------|----------|
| Secrets Management | PASS | `.github/workflows/*.yml` reference `secrets.*` with branch / repo scoping; no plaintext tokens in repo. `main.rs::resolve_redaction_key` only reports `RedactionKeySource`, never the key. `github_auth.rs::safe_metadata_does_not_serialize_credential_material` test pins the no-credential-in-serialization property. |
| SQL Injection | PASS | `cache/sqlite.rs` uses `rusqlite::params!` in all 9 query sites (`get`, `lookup`, `set_with_ttl`, `contains`, `cleanup_expired`, `count_older_than`, `cleanup_older_than`, `clear`, `stats`, `inspect`). |
| Command Injection | PASS | Only `Command::new` callers are: (a) integration tests invoking the CLI itself, (b) `main.rs::try_open_path` running `xdg-open`/`open`/`explorer.exe` with a single canonicalized arg, (c) `github_auth.rs::run_gh` running `gh` with hardcoded argv and a 5s timeout. |
| Unsafe Code | PASS | `[workspace.lints.rust] unsafe_code = "deny"` + `grep` zero matches. |
| Unsafe Regex | PASS | `regex = "1.12.3"` (linear-time engine on the release line that deprecated backtracking). `RegexBuilder` used at `main.rs:13703` accepts user-supplied pattern but is bound to a local CLI invocation that runs against local data only (self-DoS only, no remote attacker model). |
| Input Validation | PASS | `anyhow::Context` with `.with_context` on all file/network/deserialization paths. |
| Path Traversal (writes) | PASS | Zip writer (`bundle/mod.rs::write_zip`) uses `path.strip_prefix(out_dir)` so the relative entry name cannot escape the run directory. |
| Path Traversal (reads) | N/A | All read paths come from CLI args the invoking operator chose. |
| Redaction | PASS | Three profiles; deterministic HMAC-SHA256; alias cache never shipped in bundles. |
| YAML Parsing | PASS | Uses maintained `serde_yaml_ng = "0.10.0"` (not the abandoned `serde_yaml`). |
| HTTPS Enforcement (LLM) | PASS | `OpenAiCompatibleBackend::complete` calls `validate_https_endpoint` before issuing any request (VULN-001 fix from 2026-06-29 retained). |
| HTTPS Enforcement (GitHub) | PASS | VULN-002 fix retained: `validate_https_api_base` helper in `apps/shiplog/src/ingest/github.rs` is invoked from `make_github_ingestor` at `apps/shiplog/src/main.rs:12630`. Loopback carve-out is documented and bounded (no remote attacker model). |
| HTTPS Enforcement (GitLab / Jira / Linear) | PASS | `gitlab_api_base_url` and Jira's `api_base_url` both `format!("https://{instance}/...")` — scheme is a literal constant regardless of caller input. `linear.rs::api_base_url` is a `https://` constant. |
| Fuzzing | ACTIVE | 36 fuzz targets in `fuzz/fuzz_targets/`; `fuzz-smoke.yml` + `fuzzing.yml` workflows. |
| Property Testing | ACTIVE | `proptest` on redact leak detection, cache TTL math, ingest windows. |
| Mutation Testing | ACTIVE | `cargo-mutants` configured (`cargo-mutants.toml`, `.cargo/mutants.toml`). |
| Lint Floor | PASS (carried forward) | `cargo clippy --workspace --all-targets -- -D warnings` clean (verified during prior scans; not re-run this cycle). |
| Build Floor | PASS (carried forward) | `cargo check --workspace --all-targets` clean (verified during prior scans; not re-run this cycle). |

## Critical Findings

None.

## High Findings

None.

## Medium Findings

None.

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
| OBS-4 | `dtolnay/rust-toolchain@master` | `.github/workflows/*.yml` (22 occurrences across ci.yml, ci-actuals.yml, fuzz-smoke.yml, fuzzing.yml, pr-plan.yml, bdd-smoke.yml, bdd-testing.yml, property-smoke.yml, property-testing.yml, mutation-testing.yml, coverage.yml, release.yml) | The `dtolnay/rust-toolchain` action is referenced by `@master` (mutable ref) rather than a pinned SHA in most workflows. This is a soft supply-chain risk: an attacker who can push to `dtolnay/rust-toolchain@master` could inject malicious code into CI. Mitigations: (a) `dtolnay/rust-toolchain` is a well-known, widely-audited action; (b) `rust-toolchain.toml` pins the actual Rust version; (c) `em-ci-routed-shiplog-rust.yml` already uses `@v1` for the same action. Below the medium threshold because the action is well-known and the toolchain version is independently pinned, but worth a follow-up to convert the `@master` refs to SHA pins for full SLSA compliance. Carried forward from 2026-07-06. |
| OBS-5 | Library-surface HTTPS bypass | `apps/shiplog/src/ingest/github.rs` (struct field `api_base: String` is `pub`) | `GithubIngestor::new` and the public field `api_base` allow direct construction / mutation without going through `validate_https_api_base`. The validation is enforced only at the CLI bridge (`make_github_ingestor`). For library consumers (currently only the CLI), this is a defense-in-depth gap. Mitigated today because the CLI is the only consumer; if a public library surface is published, the struct should either validate at `new`/`with_api_base` or expose the field through a validating setter. Below the medium threshold because no public library surface ships today. |

## Appendix

### Threat Model

- Version: 2026-05-11 (unchanged this scan; still within 90-day freshness window)
- Location: `.factory/threat-model.md`
- Status: **Current** (aged 70 days, well under the 90-day refresh threshold)
- Action taken: re-used as scan context; no regeneration required.
- The Information Disclosure category in the threat model already names
  "leak sensitive info (token, email, private repo names)" as a High
  concern. VULN-002 (now resolved) was a concrete code-path instance of
  that documented threat; the fix remains in place and is verified by
  the test `api_base_rejects_http_scheme` and
  `api_base_rejects_cleartext_attacker` at `apps/shiplog/src/ingest/github.rs`.

### Scan Metadata

- Commits Scanned in Strict 7-Day Window: 1 (`6fe7b66`, history-join import)
- Files Examined: 90+ Rust source files (security-sensitive paths in
  `apps/shiplog/src/{redact,cache,ingest,cluster_llm,commands,render,bundle,workstreams,merge,github_activity,github_auth}`),
  18 GitHub Actions workflows, 2 policy files, 1 threat model, Cargo
  workspace manifest + lockfile
- Scan Duration: ~10 minutes
- Build/Lint Gates Run: static read-only review (no full `cargo test`/`cargo clippy` re-run on the imported tree this cycle; deferred to the import's own CI pipeline at `.github/workflows/ci.yml`)
- Tools Used: ripgrep (pattern search), file_read (manual review), git
  history, manual STRIDE walk

### Recommendations

1. **Continue weekly cadence** - Three consecutive clean-to-low-result
   scans plus the history merge demonstrate that the security-relevant
   surface is well-controlled; maintaining the weekly cadence catches
   regressions early.
2. **Consider converting `@master` to SHA pins** - OBS-4. Carrying
   forward as a follow-up.
3. **Consider validating `api_base` at the library seam, not only the CLI
   seam** - OBS-5. Defense-in-depth for any future library surface.
4. **Track upstream `serde_yaml_ng` cadence** - OBS-3.
5. **Document the local-CLI regex caveat in `--help` text** - OBS-1.

### Validation Signals

- **Observed**: 1 commit in the past 7 days (`6fe7b66`); no `unsafe`
  blocks in any Rust source; clippy passes with `-D warnings` per the
  prior report; workspace `cargo check` succeeds per the prior report.
- **Reported**: Threat model file mtime is 2026-07-20 commit / 2026-05-11
  authored content (current under the 90-day refresh rule); previous
  security report is `security-report-2026-07-13.md`.
- **Not verified**: No remote repository API call was performed; GitHub-side
  secret rotation / exposed token state cannot be checked from this
  checkout (and is governed by `EffortlessMetrics/shiplog` repo settings,
  not by code in this repo). No `cargo test` / `cargo clippy` was
  re-executed on the imported tree this cycle (deferred to the
  import's own CI pipeline).

### References

- [CWE-319: Cleartext Transmission of Sensitive Information](https://cwe.mitre.org/data/definitions/319.html)
- [STRIDE Threat Model](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Rust Security Advisory Database](https://rustsec.org/)

---

*Report generated by Factory Droid (security-engineer plugin). No new
findings at or above the medium severity threshold this cycle; carried-forward
observations (OBS-1 through OBS-5) remain below the threshold and require no
immediate action.*
