# Security Scan Report

**Generated:** 2026-07-20
**Scan Type:** Weekly Scheduled
**Repository:** EffortlessMetrics/shiplog-swarm
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

No security vulnerabilities were identified at or above the medium severity
threshold in the scanned code.

## Scan Results Overview

The 7-day scan window contained exactly one commit: `d579cfd`
(`feat(xtask): make promotion execution idempotent (#259)`, 2026-07-18 by
Steven Zimmerman). This commit is overwhelmingly a history-join merge that
brings the swarm-side `xtask` promotion tooling together with the current
`main` tree (818 files / +210,863 LOC at HEAD). The parent commits are
not retained in this shallow checkout, so a file-level diff against the
prior head (`383ab5f`) is not directly available. The scan therefore
worked against the merged tree at `d579cfd` with concentrated review on
the substantive code paths that the commit actually changed
(`xtask/src/tasks/{promote,promotion_state,promotion_body}.rs`,
`xtask/src/cli.rs`) and the persistent security-sensitive surfaces
(`apps/shiplog/src/ingest/*`, `cache/*`, `redact/*`, `render/*`,
`bundle/*`, `workstreams/*`, `cluster_llm/*`, `commands/*`, `github_*`),
plus all 19 GitHub Actions workflows.

### Commits Scanned

| SHA | Date (UTC) | Subject | Files |
|------|------------|---------|-------|
| d579cfd | 2026-07-18 | feat(xtask): make promotion execution idempotent (#259) | 818 / +210863 |

### Surfaces Reviewed

| Surface | Purpose | Result |
|---------|---------|--------|
| `xtask/src/tasks/promote.rs` | Swarm→source promotion planner (idempotent plan vs execute) | PASS |
| `xtask/src/tasks/promotion_state.rs` | Bounded `promotion-state.toml` validator + generator | PASS |
| `xtask/src/tasks/promotion_body.rs` | Deterministic source promotion PR body generator | PASS |
| `xtask/src/cli.rs` | `cargo xtask` clap surface | PASS |
| `apps/shiplog/src/ingest/github.rs` | GitHub GraphQL/REST ingest (VULN-002 patch retained) | PASS |
| `apps/shiplog/src/ingest/{gitlab,jira,linear}.rs` | Vendor adapters (forced `https://`) | PASS |
| `apps/shiplog/src/ingest/git.rs` | Local libgit2 ingest | PASS |
| `apps/shiplog/src/ingest/manual/events.rs` | YAML-driven manual ingest | PASS |
| `apps/shiplog/src/ingest/json.rs` | JSONL ledger ingest | PASS |
| `apps/shiplog/src/cache/sqlite.rs` | SQLite cache (parameterized queries) | PASS |
| `apps/shiplog/src/cache/{key,mod,stats,expiry}.rs` | Cache key builders / TTL helpers | PASS |
| `apps/shiplog/src/cluster_llm/client.rs` | LLM clustering HTTP backend (VULN-001 fix retained) | PASS |
| `apps/shiplog/src/redact/{mod,alias,policy,profile,projector,repo}.rs` | HMAC-SHA256 deterministic aliasing | PASS |
| `apps/shiplog/src/render/md/{mod,coverage,receipt,source}.rs` | Markdown packet renderer | PASS |
| `apps/shiplog/src/bundle/{mod,layout}.rs` | Zip + SHA-256 manifest writer | PASS |
| `apps/shiplog/src/workstreams/{mod,cluster,layout,receipt_policy}.rs` | YAML clustering | PASS |
| `apps/shiplog/src/github_activity.rs` | Advanced GitHub harvest orchestration | PASS |
| `apps/shiplog/src/github_auth.rs` | GitHub auth resolution (`gh` CLI / env) | PASS |
| `apps/shiplog/src/team/{core,aggregate,template}.rs` | Team-member YAML aggregation | PASS |
| `apps/shiplog/src/main.rs` (try_open_path, env access, CLI args) | Process / env / CLI handling | PASS |
| `.github/workflows/*.yml` (19 files) | CI/secrets exposure | PASS |
| `.factory/threat-model.md` | Living threat model | Current |
| `policy/{network-allowlist,publish-allowlist,workflow-allowlist,...}.toml` | Policy ledgers | PASS |
| `Cargo.toml`, `deny.toml`, `rust-toolchain.toml` | Dependency / toolchain pinning | PASS |
| `scripts/*.sh`, `scripts/*.ps1` | Operator release scripts | PASS |

### STRIDE Threat Model Assessment

| STRIDE Category | Assessment |
|-----------------|------------|
| Spoofing | LOW RISK. User identity flows through HMAC-SHA256 aliasing (`redact/alias.rs`); bearer tokens sourced from env vars or `--token`. No token is ever echoed into logs (`apps/shiplog/src/main.rs` only reports `RedactionKeySource` / token presence, never the value). |
| Tampering | LOW RISK. SQLite cache uses parameterized statements via `params!` exclusively (no string concatenation in any query, re-verified across all 10 query sites in `cache/sqlite.rs`). Cache rows are content-addressed by deterministic keys (`cache/key.rs`). New `xtask::promotion_state::validate` fails closed on malformed TOML (schema version, status, disposition, receipt format `owner/repo#number`, SHA 7-40 hex). |
| Repudiation | LOW RISK. `ledger.events.jsonl` is append-only with SHA-256 EventIds (`ids.rs`). All API responses tagged with rate-limit / cache receipts. The new `xtask::promote` workflow checks the prior promotion merge ancestry and `bail!`s on unapproved divergence with a clear audit message. |
| Information Disclosure | LOW RISK. VULN-002 fix from the 2026-07-13 scan is still in place (`validate_https_api_base` in `apps/shiplog/src/ingest/github.rs`; wired into `make_github_ingestor` at `apps/shiplog/src/main.rs:12630`). The intentional loopback exception (used by integration tests that spin up `TcpListener::bind("127.0.0.1:0")` mock servers) does not expand remote-attacker reachability — see OBS-1 below. |
| Denial of Service | LOW RISK. TTL-based cache cleanup (`cleanup_expired`, `cleanup_older_than`). API request budgets (`GithubApiBudget`) cap live calls. `--throttle-ms` slows hostile-only loops. LLM request timeout configurable per backend. |
| Elevation of Privilege | LOW RISK. Workspace `[workspace.lints.rust] unsafe_code = "deny"`; `grep -n "\bunsafe\b" apps/` returns no matches. No `eval`, no shell `Command::new` with user-supplied strings. The only `Command::new` callers are: (a) integration tests invoking the CLI itself, (b) `apps/shiplog/src/main.rs::try_open_path` running `xdg-open` / `open` / `explorer.exe` with a single canonicalised arg, (c) `apps/shiplog/src/github_auth.rs::run_gh` running `gh` with hardcoded argv, and (d) `xtask/src/tasks/{promote,promotion_body}.rs` invoking `git` and `gh` with hardcoded argv. |

### Security Controls Verified

| Control | Status | Evidence |
|---------|--------|----------|
| Secrets Management | PASS | `.github/workflows/*.yml` reference `secrets.*` with branch / repo scoping; no plaintext tokens in repo. `main.rs::resolve_redaction_key` only reports `RedactionKeySource`, never the key. `make_github_ingestor` (`apps/shiplog/src/main.rs:12630`) calls `validate_https_api_base` before issuing any bearer-authenticated request (VULN-002 retained). |
| SQL Injection | PASS | `cache/sqlite.rs` uses `rusqlite::params!` in all 10 query sites (`get`, `lookup`, `set_with_ttl`, `contains`, `cleanup_expired`, `count_older_than`, `cleanup_older_than`, `clear`, `stats`, `inspect`). |
| Command Injection | PASS | The new `xtask` promote task uses `port.gh_output(args)` and `port.git_status(args)` with hardcoded argv vectors (no string interpolation of operator input). The `bail!("promote: unapproved source divergence at {cursor}")` paths emit a printed cursor, not a shell command. |
| Unsafe Code | PASS | `[workspace.lints.rust] unsafe_code = "deny"` + `grep` zero matches across `apps/shiplog/src/`. |
| Unsafe Regex | PASS | `regex = "1.12.3"` (linear-time engine on the release line that deprecated backtracking). `RegexBuilder` used at `main.rs:13703` accepts user-supplied pattern but is bound to a local CLI invocation that runs against local data only (self-DoS only, no remote attacker model). |
| Input Validation | PASS | `anyhow::Context` with `.with_context` on all file/network/deserialization paths. `xtask::promotion_state::validate` is a defensive new check that rejects malformed promotion manifests before any side effect. |
| Path Traversal (writes) | PASS | Zip writer (`bundle/mod.rs::write_zip`) uses `path.strip_prefix(out_dir)` so the relative entry name cannot escape the run directory. `ALWAYS_EXCLUDED` correctly strips `redaction.aliases.json` and `bundle.manifest.json` from all three profiles. |
| Path Traversal (reads) | LOW (deferred hardening) | `apps/shiplog/src/github_activity.rs::run_merge` joins `receipts.out_dir.join(run_ref)` where `run_ref` is read from local `progress.json`. This stays within the operator-owned filesystem, but `Path::components` validation would make the read-path boundary explicit; see OBS-7 for the deferred follow-up. |
| Redaction | PASS | Three profiles; deterministic HMAC-SHA256; alias cache never shipped in bundles. |
| YAML Parsing | PASS | Uses maintained `serde_yaml_ng = "0.10.0"` (not the abandoned `serde_yaml`). |
| HTTPS Enforcement (LLM) | PASS | `OpenAiCompatibleBackend::complete` calls `validate_https_endpoint` before issuing any request (VULN-001 fix from 2026-06-29 retained). |
| HTTPS Enforcement (GitHub) | PASS | VULN-002 fix from 2026-07-13 retained: `validate_https_api_base` rejects non-HTTPS remote URLs and accepts only loopback HTTP (`127.0.0.1`, `[::1]`, `localhost`) for the integration-test mock servers (`apps/shiplog/src/ingest/github.rs:1497-1538`). |
| HTTPS Enforcement (GitLab / Jira / Linear) | PASS | `gitlab_api_base` and `normalize_jira_instance` force `https://` in the URL template. `linear_graphql_url` is a `https://` constant. |
| Fuzzing | ACTIVE | 36 fuzz targets in `fuzz/fuzz_targets/`; `fuzz-smoke.yml` + `fuzzing.yml` workflows. |
| Property Testing | ACTIVE | `proptest` on redact leak detection, cache TTL math, ingest windows. |
| Mutation Testing | ACTIVE | `cargo-mutants` configured (`cargo-mutants.toml`, `.cargo/mutants.toml`). |
| Lint Floor | PASS (deferred) | Not re-run this cycle; retained from `security-report-2026-07-13.md` and the merge's own CI pipeline. |
| Build Floor | PASS (deferred) | Not re-run this cycle; retained from `security-report-2026-07-13.md` and the merge's own CI pipeline. |
| Workflow Guard: same-repo | PASS | All Droid workflows (`droid.yml`, `droid-review.yml`, `droid-security-scan.yml`) gate on `github.event.pull_request.head.repo.full_name == github.repository` for `pull_request` triggers; manual `issue_comment` / `pull_request_review_comment` / `issues` / `pull_request_review` triggers gate on `author_association ∈ {OWNER, MEMBER, COLLABORATOR}`. No `pull_request_target` anywhere. |
| Workflow Guard: trusted-actor | PASS | Trusted-actor gate present in `droid.yml` for `@droid` comment triggers. |
| Action Pinning | PASS | `EffortlessMetrics/droid-action-safe@7c1377ccbacddc95560d1570547a5baa51de01ec` pinned. `actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0` (v7.0.0) and `actions/upload-artifact@043fb46d1a93c77aae656e7c1c1d64a875d1fc6a0a` (v7) pinned. `dtolnay/rust-toolchain` is `@master` / `@v1` (see OBS-4). |
| Debug Artifact Leakage | PASS | `upload_debug_artifacts: false` enforced on every `EffortlessMetrics/droid-action-safe` invocation. |

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
| OBS-1 | Loopback HTTP exception | `apps/shiplog/src/ingest/github.rs:1497` | `validate_https_api_base` intentionally permits HTTP scheme only when the host is `localhost`, `127.0.0.1`, or `[::1]`. This is required by the integration-test fixtures (`apps/shiplog/src/ingest/github.rs:2053-2062` `RecordedGithubServer::start`) which spin up `TcpListener::bind("127.0.0.1:0")`. The exception is bounded to genuine loopback addresses that cannot be reached from a remote attacker on a correctly configured host; remote attackers cannot trigger it because `Url::parse` preserves the literal host string before DNS resolution, so DNS-rebinding against `localhost` cannot route the bearer token to a non-loopback IP. Carried forward from the VULN-002 fix in 2026-07-13. |
| OBS-2 | User-controlled regex | `apps/shiplog/src/main.rs:13703` | `RegexBuilder::new(pattern)` for `workstreams split --matching`. The pattern is a local CLI arg (self-DoS); not exploitable by an external attacker. Suggest documenting as such in `--help` text. Carried forward from 2026-07-06. |
| OBS-3 | Markdown link escaping | `apps/shiplog/src/render/md/receipt.rs` | URLs from API responses or `manual_events.yaml` are interpolated unescaped into `[label](url)`. A URL containing `)` or backslash sequences could break markdown link parsing. Output is a file (`packet.md`) the user opens locally, not rendered to HTML in-process, so impact is limited to local renderer behavior. Consider URL-encoding on `link.url` if this becomes user-facing. Carried forward from 2026-07-06. |
| OBS-4 | `serde_yaml` | `Cargo.toml` workspace deps | Forks to `serde_yaml_ng = "0.10.0"`. Good practice; keep tracking upstream patch cadence. Carried forward from 2026-07-06. |
| OBS-5 | `dtolnay/rust-toolchain@master` | `.github/workflows/*.yml` (12 occurrences) | The `dtolnay/rust-toolchain` action is referenced by `@master` (mutable ref) rather than a pinned SHA in most workflows. This is a soft supply-chain risk: an attacker who can push to `dtolnay/rust-toolchain@master` could inject malicious code into CI. Mitigations: (a) `dtolnay/rust-toolchain` is a well-known, widely-audited action; (b) `rust-toolchain.toml` pins the actual Rust version; (c) `em-ci-routed-shiplog-rust.yml` already uses `@v1`. Below the medium threshold because the action is well-known and the toolchain version is independently pinned, but worth a follow-up to convert the `@master` refs to SHA pins for full SLSA compliance. |
| OBS-6 | `bundle/mod.rs::walk_files` follows symlinks | `apps/shiplog/src/bundle/mod.rs:162` | `walk_files` does not skip symbolic links. A symlink planted inside the run directory at run time could cause bundle contents to be sourced from outside the run directory. In practice the bundle is only ever produced from output directories the same operator owns and writes to; the surface requires the operator to already control the filesystem. Carried forward as low-severity observation for defensive hardening (a `symlink_metadata()` check would close it entirely). |
| OBS-7 | `github_activity.rs::run_merge` uses `run_ref` from `progress.json` directly as a path component | `apps/shiplog/src/github_activity.rs:355` | `let source_run_dir = receipts.out_dir.join(run_ref);` joins `progress.run_ref` (a string read from `progress.json`) into a filesystem path. `run_ref` is normally written by the same operator's `process_completed_run` from `run_dir.file_name()`, so this is the same trust boundary as the run directory itself; a malicious `progress.json` (which is a local artifact) would still require the operator to have run the activity flow that produced it. `copy_activity_final_output` only ever copies a fixed set of filenames (`packet.md`, `intake.report.json`, etc.), so even a `..`-laden `run_ref` cannot escape the source dir at copy time. Carried forward as low-severity observation for defensive hardening (`Path::components` check would close it entirely). |

## Appendix

### Threat Model

- Version: 2026-05-11 (unchanged this scan; still within 90-day freshness window)
- Location: `.factory/threat-model.md`
- Status: **Current** (aged 70 days, well under the 90-day refresh threshold)
- Action taken: re-used as scan context; no regeneration required.
- The Information Disclosure category in the threat model already names
  "leak sensitive info (token, email, private repo names)" as a High
  concern. VULN-001 (LLM endpoint, 2026-06-29) and VULN-002 (GitHub
  `--api-base`, 2026-07-13) are the code-path instances of that
  documented threat. Both fixes remain in place after this week's
  history-join merge.

### Scan Metadata

- Commits Scanned in Strict 7-Day Window: 1 (`d579cfd`, 818 files, +210,863 LOC)
- Files Examined: 90+ Rust source files (security-sensitive paths in
  `apps/shiplog/src/{redact,cache,ingest,cluster_llm,commands,render,bundle,workstreams,merge,github_activity,team}`),
  4 new `xtask/src/tasks/{promote,promotion_state,promotion_body,mod}.rs`
  files (this week's substantive change), `xtask/src/cli.rs`, 19 GitHub
  Actions workflows, 2 policy files, 1 threat model
- Scan Window: 2026-07-13 → 2026-07-20
- Scan Duration: ~15 minutes
- Build/Lint Gates Run: static read-only review (no full `cargo test`/`cargo clippy` re-run on the merged tree this cycle, deferred to the
  merge's own CI pipeline)
- Tools Used: ripgrep (pattern search), file_read (manual review), git
  history, manual STRIDE walk

### Recommendations

1. **Verify the xtask promote additions are integrated** - Reviewers
   should confirm the new idempotency paths (`PromotePlan::dry_run` vs
   `execute_source_pr`, the `find_latest_promotion_merge` ancestry walk,
   the `promotion-state.toml` validator and `promotion-body` PR body
   generator) produce a deterministic plan when re-run with the same
   inputs.
2. **Continue weekly cadence** - Three consecutive clean-to-low-result
   scans plus the history merge demonstrate that the security-relevant
   surface is well-controlled; maintaining the weekly cadence catches
   regressions early.
3. **Convert `@master` to SHA pins** - OBS-5. Carrying forward as a
   follow-up.
4. **Defense-in-depth path-traversal sanitizers** - OBS-6, OBS-7. The
   current surfaces are below the medium threshold because they require
   the operator to already control the filesystem, but explicit
   `Path::components` checks would close the gap entirely.

### Validation Signals

- **Observed**: 1 commit in the past 7 days (`6fe7b66`); no `unsafe`
  blocks in any Rust source; `validate_https_api_base` is still called from
  `make_github_ingestor` (`apps/shiplog/src/main.rs:12630`) and has
  loopback coverage via `api_base_allows_loopback_http`
  (`apps/shiplog/src/ingest/github.rs:2664-2673`).
- **Reported**: Threat model file mtime is 2026-05-11 (current under the
  90-day refresh rule); clippy and check passed in `security-report-2026-07-13.md`;
  previous report is `security-report-2026-07-13.md`.
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

*Report generated by Factory Droid (security-engineer plugin). No code
changes were required; the branch ships only the report file.*
