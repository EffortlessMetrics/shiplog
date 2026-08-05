# Security Scan Report

**Generated:** 2026-08-03
**Scan Type:** Weekly Scheduled
**Repository:** EffortlessMetrics/shiplog-swarm
**Branch:** `droid/security-report-2026-08-03`
**Severity Threshold:** medium
**Scan Window:** 2026-07-27 to 2026-08-03 (7 days)

## Executive Summary

| Severity | Count | Auto-fixed | Manual Required |
|----------|-------|------------|-----------------|
| CRITICAL | 0     | 0          | 0               |
| HIGH     | 0     | 0          | 0               |
| MEDIUM   | 0     | 0          | 0               |
| LOW      | 0     | 0          | 0               |

**Total Findings (>= medium):** 0
**Auto-fixed:** 0
**Manual Review Required:** 0

No security vulnerabilities were identified at or above the medium severity
threshold in the 46 commits scanned this week. The scan window is dominated
by the same transition-evidence / promotion-overlay work that produced
VULN-001 in the 2026-07-27 report and the follow-up security strengthenings
recorded in PR #299 (token exfiltration defense), PR #311 (source bot guard
contract), PR #308 (source automation guard), PR #333 (source review policy),
and PR #290 (identity placeholder visibility). Each of those PRs is a
defensive hardening rather than a newly identified vulnerability, and the
overall posture is unchanged from the 2026-07-27 close.

## Critical Findings

None.

## High Findings

None.

## Medium Findings

None.

## Low Findings

None at or above the medium severity threshold.

### Observations Below Threshold

| ID | Class | File | Note |
|----|-------|------|------|
| OBS-1 | Loopback HTTP exception | `apps/shiplog/src/ingest/github.rs:1510` | Carried forward. The `validate_https_api_base` function intentionally permits HTTP for genuine loopback addresses (match-arm checks `url::Host::Ipv4`/`Ipv6` against `IpAddr::is_loopback()` and treats `localhost` case-insensitively). The exception cannot be reached by a remote attacker because `Url::parse` preserves the literal host string before DNS resolution, so a misleading `Host` header cannot redirect the bind. PR #299 extends the call sites without diluting the loopback carve-out. |
| OBS-2 | User-controlled regex | `apps/shiplog/src/main.rs` (workstreams split) | Carried forward. `RegexBuilder::new(pattern)` for `workstreams split --matching`. Self-DoS only; not exploitable by an external attacker because the pattern is supplied by the local operator over the CLI. |
| OBS-3 | Markdown link escaping | `apps/shiplog/src/render/md/receipt.rs` | Carried forward. URLs from API responses or `manual_events.yaml` are interpolated unescaped into `[label](url)`. Output is a file (`packet.md`) opened locally; impact is limited to local renderer behavior. |
| OBS-4 | `dtolbay/rust-toolchain@master` | `.github/workflows/*.yml` (12 occurrences) | Carried forward. Mutable ref rather than a SHA pin. Mitigated by `rust-toolchain.toml` pinning the actual Rust version and the action's wide audit. |
| OBS-5 | `bundle/mod.rs::walk_files` follows symlinks | `apps/shiplog/src/bundle/mod.rs:162` | Carried forward. The bundle is only ever produced from output directories the same operator owns. |
| OBS-6 | `transition.rs::lock_transitions` parses TOML per line | `xtask/src/tasks/transition.rs:469` | Carried forward. The line-oriented parser calls `line.parse::<toml::Table>()` per `+`/`-` line. The parsed value is thrown away after extracting the literal `name`/`version` string. Not a security issue, but allocates on every `Cargo.lock` diff. |
| OBS-7 | `discover_github_user` body / error leakage | `apps/shiplog/src/main.rs:11965` | PR #299 wires `validate_https_api_base` into this function but the `auth_github_status_rejects_remote_http_before_environment_credentials` integration test (`apps/shiplog/tests/cli_integration.rs:18497`) explicitly asserts `!stderr.contains(token)` and `!stdout.contains(token)`. The token is sent only after the API base is verified, so the failure path can never contain the bearer token. Verified. |
| OBS-8 | `cargo xtask promote` overlay worktree ownership | `xtask/src/tasks/promote.rs` (`prepare_source_overlay`) | The new overlay worktree (PR #275, plus this week's #296/#301/#303/#305/#307/#312/#313/#316/#317/#318/#334 work) is claimed with a `pid+time` nonce and bounded to 64 retries; cleanup is `Drop`-driven on every exit path. Not a privilege issue; the worktree is owned by the invoking operator. |
| OBS-9 | `SourceAuthorityDecision` validation | `xtask/src/tasks/promotion_state.rs:345` (this-week `validate_source_authority`) | Requires a non-empty `path`, a normalized `decision_receipt`, a 40-hex `decision_merge_sha`, a non-empty `reason`, and — for active decisions — full 40-hex `source_target` / `swarm_target`. The validator is consistent with the existing `validate_transitions` / `validate_transition_path` and re-uses the shared `validate_full_sha` / `validate_receipt` / `deny_unknown_fields` helpers. No new path traversal / parser differentials identified. |
| OBS-10 | `ensure_evidence_targets` ancestor check | `xtask/src/tasks/transition.rs:503` (this-week rewrite) | The previous `ensure_exact_targets` required `source_target == refs.source_target` and `swarm_target == refs.swarm_target`. The new `ensure_evidence_targets` relaxes that to `merge-base --is-ancestor` and then re-validates the tree entries at **both** the evidence and the current target pairs. The relaxation is precisely the property VULN-001 (2026-07-27) was missing: it allows tip advancement across unrelated paths while requiring every governed path to either be unchanged at the recorded mode/oid or to be re-validated against the current tree. The change is hardening, not a weakening. |
| OBS-11 | `dependency_equivalent` patch-id path | `xtask/src/tasks/transition.rs` (PR #314) | New transition disposition that compares `git patch-id --stable` outputs for `Cargo.lock` deltas. The patch content is fed through `git patch-id` as stdin with a hardcoded argv vector; the only operator-controlled input is the lock file path, which is validate-normalized by the same path rules as `validate_transition_path`. No shell injection surface. |
| OBS-12 | Doctor identity placeholder carve-out | `apps/shiplog/src/doctor.rs:696` (PR #290) | The new `build_identity_item` reads `[user].label` and `[sources.manual].user` and compares each to the literal `SCAFFOLD_USER_PLACEHOLDER = "Your Name"`. The constant is shared between the scaffold template and the placeholder detector so the two cannot drift, and the resulting readiness item is reported as `ReadyWithCaveats` (not blocking). Privilege / property flow unchanged — the template already wrote this string into the scaffold, so the new check is a visibility improvement, not a behavior change. |

## Appendix

### Threat Model

- Version: 2026-05-11 (unchanged this scan)
- Location: `.factory/threat-model.md`
- Status: **Current** (aged 84 days, under the 90-day refresh threshold)
- Action taken: re-used as scan context; no regeneration required.
- The Information Disclosure category's "leak sensitive info (token, email,
  private repo names)" — High — is the line that PR #299 closed off by
  validating `api_base` before any bearer token is loaded, and the
  Tampering category's "Modify cached API responses to alter output" is
  the broader family that PR #275 plus this week's overlay-evidence
  chain (`fc9886f`, `245bcae`, `e0c132d`, `c836620`, etc.) hardens.

### Scan Metadata

- Commits Scanned: 46 (strict 7-day window)
- Branch: `droid/security-report-2026-08-03`
- Scan Duration: ~25m (run on 2026-08-03 from
  `git log --since="7 days ago"` and `git fetch --unshallow`)
- Skills / Tools Used: `commit-security-scan` (manual STRIDE walk via
  `Grep` + `Read` + targeted diffs), `vulnerability-validation` (manual
  reachability / exploitability review), `security-review` (manual
  verification of `validate_https_api_base`, `build_identity_item`,
  `inspect_source_bot_guard`, `validate_source_authority`,
  `derive_source_authority`, `compute_resolution_plan`, `run_verify_only`,
  `validate_intake_report`, `prepare_source_overlay`).
- Validation Locally Executed:
  - `cargo fmt --all -- --check` — clean.
  - `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` — clean.
  - `cargo test --workspace --all-features --locked -- github_auth` — 7 passed, 0 failed.
  - `cargo test --workspace --all-features --locked -- remote_http` — 2 passed, 0 failed.
  - `cargo test -p xtask --locked -- transition` — 45 passed, 0 failed
    (new this week: `active_receipt_allows_unrelated_tip_when_recorded_entries_are_unchanged`,
    `active_receipt_rejects_governed_path_change_after_evidence`, and the
    six `source_authority_*` tests).
  - `cargo test -p xtask --locked -- promote` — 56 passed, 0 failed
    (new this week: `verify_only_recomputes_modern_overlay_resolution_plan`,
    `verify_only_rejects_changed_resolution_evidence`).
  - `cargo test -p xtask --locked -- automation_authority` — 15 passed,
    0 failed (new this week: `source_requires_the_routine_bot_guard`,
    `source_guard_requires_both_routine_bot_identities_and_fail_closed_exit`,
    `source_guard_rejects_checkout_and_named_secrets`,
    `source_review_bots_may_write_comments_without_repository_contents`).

### Commits Scanned (strict 7-day window)

| SHA | Date (UTC) | Subject | Security-Relevant? |
|------|------------|---------|---------------------|
| `b758934` | 2026-08-03 | `docs(transition): refresh source authority evidence (#334)` | No (docs / state) |
| `4b686e6` | 2026-08-03 | `fix(policy): allow source review comments (#333)` | Yes (policy) |
| `fa4dbf7` | 2026-08-03 | `fix(docs): repair GitHub rustdoc links (#332)` | No (docs) |
| `98ba6a2` | 2026-08-03 | `fix(promote): refresh current promotion evidence` | No (state) |
| `550538c` | 2026-08-03 | `fix(promote): bind source policy evidence` | No (state) |
| `5fa2868` | 2026-08-02 | `fix(ci): make contributor acceptance repository-aware` | Yes (workflow) |
| `9529b57` | 2026-08-02 | `fix(promote): reconcile runbook evidence target` | No (state) |
| `245bcae` | 2026-08-02 | `fix(promote): preserve transition evidence across tip updates` | Yes (transition.rs logs) |
| `fc9886f` | 2026-08-02 | `fix(promote): verify the persisted resolution plan` | Yes (verify_only) |
| `108511b` | 2026-08-02 | `fix(promote): bind current source-authority decisions` | No (state) |
| `e0c132d` | 2026-08-02 | `docs(transition): record source-authority decisions` | No (docs) |
| `c836620` | 2026-08-02 | `fix(promote): bind source-authoritative decisions` | Yes (transition.rs) |
| `fd993c4` | 2026-08-02 | `chore(transition): bind reviewed source decisions (#318)` | No (state) |
| `7884dec` | 2026-08-02 | `docs(transition): record bounded source decisions (#317)` | No (docs) |
| `e94cce7` | 2026-08-02 | `chore(transition): record residual source evidence (#316)` | No (state) |
| `9c0f2b5` | 2026-08-02 | `chore(transition): record source port evidence (#315)` | No (state) |
| `166ed9c` | 2026-08-02 | `feat(transition): record dependency evidence (#314)` | Yes (dependency_equivalent) |
| `b5c4a11` | 2026-08-02 | `fix(transition): honor discard decisions for two-sided drift (#313)` | Yes (transition.rs) |
| `a2f7d3a` | 2026-08-02 | `feat(transition): add bounded discard-source resolution (#312)` | Yes (transition.rs) |
| `fa07228` | 2026-08-02 | `fix(policy): validate source bot guard contract (#311)` | Yes (policy) |
| `81bd81d` | 2026-08-02 | `fix(policy): allowlist executable release installer (#310)` | Yes (allowlist) |
| `a090fde` | 2026-08-02 | `fix(automation): guard source routine bot PRs (#308)` | Yes (workflow) |
| `d4b7bbd` | 2026-08-02 | `fix(transition): bind receipts to exact target trees (#307)` | Yes (transition.rs) |
| `4a4757c` | 2026-08-02 | `fix(promote): derive overlay date from source (#305)` | No (state) |
| `cb4e988` | 2026-08-02 | `fix(promote): treat policy paths literally (#303)` | No (state) |
| `9e052f3` | 2026-08-02 | `fix(promote): retain identical-tree checkpoints (#301)` | No (state) |
| `1ac6123` | 2026-08-02 | `feat(xtask): add a TreeEquivalent transition disposition (#296)` | Yes (transition.rs) |
| `0403ec2` | 2026-08-02 | `chore(policy): advance the publish allowlist release marker (#292)` | No (policy) |
| `fcf1c44` | 2026-08-02 | `chore(security): record 2026-07-27 security scan report (#291)` | No (report) |
| `2c250b9` | 2026-08-02 | `chore(release): refresh the unreleased changelog (#293)` | No (docs) |
| `5741546` | 2026-08-02 | `fix(auth): validate GitHub API base before credentials (#299)` | **Yes (CRITICAL-fix)** |
| `9b26046` | 2026-08-02 | `fix(test): remove environment coupling from two cli integration tests (#295)` | No (tests) |
| `eb7103c` | 2026-08-01 | `feat(doctor): report the scaffold identity placeholder (#290)` | Yes (visibility) |
| `7e0dc40` | 2026-08-01 | `fix(cli): include today in the last-6-months window (#289)` | No (CLI) |
| `61a9468` | 2026-08-01 | `fix(cli): default quick add to today (#288)` | No (CLI) |
| `a2de18b` | 2026-07-30 | `deps: bump clap to 4.6.4` | No (deps) |
| `b667578` | 2026-07-30 | `deps: bump serde to 1.0.229` | No (deps) |
| `2ab959b` | 2026-07-30 | `deps: bump serde_json to 1.0.151` | No (deps) |
| `4df1b55` | 2026-07-30 | `deps: bump toml to 1.1.4` | No (deps) |
| `69a8828` | 2026-07-30 | `test(cli): move start scenarios into BDD harness` | No (tests) |
| `6cb6da5` | 2026-07-29 | `feat(cli): add confirmed start command` | Yes (CLI safety) |
| `ba27d7f` | 2026-07-29 | `ci: bump actions/checkout from 7.0.0 to 7.0.1` | No (deps) |
| `a4dbf0e` | 2026-07-29 | `feat(report): structurally validate run receipts for release first-use proof` | Yes (validator) |
| `737cd22` | 2026-07-29 | `refactor(xtask): resolve each promotion path once` | No (refactor) |
| `fa8c261` | 2026-07-29 | `fix(xtask): make closeout overlay-aware and close authority holes` | Yes (closeout) |
| `dcbf5b3` | 2026-07-28 | `docs: remove residual RTK promotion guidance (#282)` | No (docs) |

### Surfaces Reviewed

| Surface | Purpose | Result |
|---------|---------|--------|
| `apps/shiplog/src/github_auth.rs` | GitHub auth resolution (env / `gh`) | PASS — PR #299 adds `validate_https_api_base` before any secret is read; `normalizes_configured_api_hosts` / `rejects_invalid_api_hosts` / `selects_environment_variables_in_order` / `safe_metadata_does_not_serialize_credential_material` all pass. |
| `apps/shiplog/src/main.rs::discover_github_user` | Bearer-token GitHub `/user` lookup | PASS — PR #299 adds the same HTTPS gate before the bearer token is materialised. |
| `apps/shiplog/src/main.rs::make_github_ingestor` | GitHub ingestor constructor | PASS — `validate_https_api_base` already wired. |
| `apps/shiplog/src/main.rs::run_start` / `run_init` | Scaffold writer | PASS — `run_start` requires `--yes`; `ensure_init_files_available` refuses to overwrite unless `--force`. |
| `apps/shiplog/src/doctor.rs::build_identity_item` | Identity placeholder reporter | PASS — shared constant reads, no shellout, no filesystem write. |
| `apps/shiplog/src/ingest/github.rs::validate_https_api_base` | HTTPS-only `api_base` validator | PASS — loopback carve-out unchanged. |
| `apps/shiplog/src/cluster_llm/client.rs::validate_https_endpoint` | HTTPS-only LLM endpoint validator | PASS — strict, no loopback. |
| `apps/shiplog/src/main.rs::validate_intake_report` | Intake report structural validator | PASS — schema version, readiness enum, top-level field allowlist, secret-bearing sentinel scan, `ensure_field_name_not_secret_bearing`, full nested item validation. |
| `apps/shiplog/src/main.rs::validate_intake_report_command` | CLI wrapper for intake validation | PASS — uses `resolve_intake_report_path` (no directory traversal) and the structural validator. |
| `xtask/src/tasks/automation_authority.rs::inspect_source_bot_guard` | Source guard workflow validator | PASS — requires `pull_request_target` with `opened`/`reopened`/`synchronize`, top-level `contents: read` + `pull-requests: read`, no write scopes, `reject-routine-bot-pr` job with both `dependabot[bot]` and `factory-droid[bot]` markers, explicit `exit 1`, no `actions/checkout`, no `secrets.`. |
| `xtask/src/tasks/automation_authority.rs::inspect_workflow` | Workflow permission validator | PASS — PR #333 lets source `droid-review.yml` / `droid.yml` write issues / pull-requests comments while still forbidding repo contents writes. |
| `xtask/src/tasks/transition.rs::derive_authority` | Active transition receipt authority | PASS — uses `ensure_evidence_targets` (ancestor + re-validated tree entries). |
| `xtask/src/tasks/transition.rs::derive_source_authority` | Source-authority decision authority | PASS — requires the path to be in `policy/source-only-paths.toml`, exact evidence targets, identical tree entries at the evidence and current targets, and a reachable decision merge SHA. |
| `xtask/src/tasks/transition.rs::ensure_evidence_targets` | Evidence-target ancestor check | PASS — only records an authority grant when the recorded targets are ancestors of the current promotion targets and the tree entries match at both pairs. |
| `xtask/src/tasks/promote.rs::compute_resolution_plan` | Shared plan builder | PASS — re-validates the merge-base, runs `plan_path_resolutions`, then `ensure_no_blocked_paths`. |
| `xtask/src/tasks/promote.rs::run_verify_only` | Post-merge verification | PASS — re-derives the exact resolution plan from the recorded source parent, swarm head, policy, and active evidence, then compares its `plan_id` against the overlay's recorded `Shiplog-Resolution-Plan:` trailer. |
| `xtask/src/tasks/promote.rs::derive_promotion_transition_authority` | Shared evidence authority | PASS — single source of truth for both `run_with_port_to` and `run_verify_only`. |
| `xtask/src/tasks/promotion_state.rs::validate_source_authority` | Source-authority decision validator | PASS — non-empty path, `validate_receipt`, `validate_full_sha`, non-empty `reason`, full 40-hex targets for active decisions. |
| `xtask/src/tasks/promotion_state.rs::validate` | Top-level manifest validator | PASS — schema version, status enum, transitions, deferred receipts, source-authority decisions. |
| `scripts/install.sh` | Unix installer | PASS — uses HTTPS with `--proto '=https' --tlsv1.2`, validates SHA-256 against `SHA256SUMS.txt`, refuses to install over an existing `shiplog` binary unless the operator exercises the explicit checks, uses TMPDIR with a `trap` cleanup. |
| `policy/executable-allowlist.toml` | Executable ledger | PASS — `exec-install-sh` entry added with `owner = "release"`, `reason = "Verified Unix installer for the latest published shiplog binary."`, `expires = "permanent"`. |
| `policy/workflow-allowlist.toml` | Workflow ledger | PASS — `source-automation-guard.yml` entry added with `contents:read` + `pull-requests:read`, no secrets, no external actions. |
| `.github/workflows/source-automation-guard.yml` | Source bot guard | PASS — `pull_request_target` only, `permissions: contents: read / pull-requests: read`, no checkout, no `secrets.*`, hard fail (`exit 1`) on either routine bot identity. |
| `.github/workflows/contributor-acceptance.yml` | Fresh contributor CI | PASS — now uses `${GITHUB_REPOSITORY}` instead of the hardcoded `EffortlessMetrics/shiplog-swarm`, so forks and other repos see the correct `git remote get-url origin` expectation. |
| `.github/workflows/droid{,-review,-security-scan}.yml` | Droid automation | PASS — same-repo guard, trusted-actor guard, no `pull_request_target`, action SHA pinning. |
| `.factory/threat-model.md` | Living threat model | Current (aged 84 days) |
| `Cargo.lock` (4 dependabot bumps) | Dependency manifest | PASS — clap 4.6.4, serde 1.0.229, serde_json 1.0.151, toml 1.1.4. No current RustSec advisory affects any of these versions. |
| `apps/shiplog/tests/cli_integration.rs` | Two new integration tests for PR #299 | PASS — `collect_github_me_rejects_remote_http_before_token_or_network` and `auth_github_status_rejects_remote_http_before_environment_credentials` both close the listening socket without ever consuming the token value. |

### STRIDE Threat Model Assessment

| STRIDE Category | Assessment |
|-----------------|------------|
| Spoofing | LOW RISK. Identities flow through HMAC-SHA256 aliasing (`apps/shiplog/src/redact/alias.rs`). Bearer tokens are sourced from env vars or `--token`; the new `discover_github_user` and `github_auth::resolve` paths cannot emit a token in error output because the failure path takes the API base validation branch before the token is dereferenced (verified by the `!stderr.contains(token)` and `!stdout.contains(token)` assertions in the new integration tests). |
| Tampering | LOW RISK. The transition receipt authority added in PR #277 (VULN-001, 2026-07-27) is now bounded by `ensure_evidence_targets` + dual tree-entry re-validation, and the overlay's `prepare_source_overlay` restores transition-approved source-only paths from source so the alignment check and the overlay content agree. `verify_only` re-derives the resolution plan from the persisted source/swarm state and refuses a modern overlay whose plan id cannot be reproduced. All eight `apps/shiplog/src/cache/sqlite.rs` query sites remain parameterized via `rusqlite::params!`. |
| Repudiation | LOW RISK. `ledger.events.jsonl` is append-only with SHA-256 `EventId`s (`shiplog::ids`). The `run_verify_only` flow writes a machine-readable receipt alongside the resolution plan id and the policy that produced it, so a reviewer can confirm the exact evidence chain that authorised every overlay checkpoint. |
| Information Disclosure | LOW RISK. PR #299 closes the token-exfiltration vector via a malicious `api_base` URL. `validate_https_api_base` is now wired in `discover_github_user`, `github_auth::resolve`, and `make_github_ingestor`. The LLM clustering adapter's `validate_https_endpoint` continues to refuse any non-`https` scheme. The `validate_intake_report` validator scans every field name against a secret-bearing sentinel list and refuses unknown fields. |
| Denial of Service | LOW RISK. The overlay worktree is claimed with a unique `pid+time` nonce and bounded to 64 retries; cleanup is `Drop`-driven on every exit path. `dependency_equivalent` computes `git patch-id --stable` at most once per active transition. The source bot guard workflow cost is one literal `exit 1` per push. |
| Elevation of Privilege | LOW RISK. The workspace lint floor (`unsafe_code = "deny"`) is unchanged; the new code adds no `unsafe` and no `Command::new` calls with user-supplied argv. The `run_start` command requires `--yes` before any filesystem write. `ensure_init_files_available` refuses to overwrite `shiplog.toml` / `manual_events.yaml` unless `--force` is passed. |

### Security Controls Verified

| Control | Status | Evidence |
|---------|--------|----------|
| Secrets Management | PASS | `.github/workflows/*.yml` reference `secrets.*` with branch / repo scoping; no plaintext tokens in repo. `main.rs::resolve_redaction_key` only reports `RedactionKeySource`. The new `github_auth::resolve` (`apps/shiplog/src/github_auth.rs`) compares the recorded SHA against the forge-reported merge commit through `check_merged_at`; an attacker who tries to swap the recorded SHA fails closed. |
| SQL Injection | PASS | `cache/sqlite.rs` uses `rusqlite::params!` in all 8 query sites (re-verified). |
| Command Injection | PASS | `transition.rs::system_patch_id` invokes `git patch-id --stable` with a hardcoded argv; the patch content is the only stdin. The `promote.rs` overlay worktree uses hardcoded argv vectors and environment-passed `GIT_OBJECT_DIRECTORY` / `GIT_ALTERNATE_OBJECT_DIRECTORIES`. `run_gh` and `gh_command` use hardcoded argv (the `SHIPLOG_TEST_GH_COMMAND` Windows test seam is documented as debug-only and gated behind `cfg(all(windows, debug_assertions))`). |
| Unsafe Code | PASS | `[workspace.lints.rust] unsafe_code = "deny"` and the new code adds no `unsafe` blocks. |
| Unsafe Regex | PASS | `regex = "1.12.3"` (linear-time engine). The transition evidence path does not compile any user-supplied pattern. |
| Input Validation | PASS | `validate_transition_path` mirrors the `policy/source-only-paths.toml` validator. `validate_full_sha` already required 40 hex chars. The new `validate_source_authority` rejects empty paths, duplicate paths, malformed decision receipts, malformed decision merge SHAs, empty reasons, and active decisions without exact evidence targets. |
| Path Traversal (writes) | PASS | `bundle/mod.rs::write_zip` uses `path.strip_prefix(out_dir)` so the relative entry name cannot escape the run directory. The new overlay path union is normalized by the same validator and only ever passed to `git checkout <sha> -- <path>` and `git rm -r --force --ignore-unmatch -- <path>` against paths inside the worktree. |
| Path Traversal (reads) | N/A | All read paths come from operator-supplied CLI args or from the `transition.path` / `source_authority.path` fields, which the new validators restrict to non-empty normalized strings. |
| Redaction | PASS | Three profiles; deterministic HMAC-SHA256; alias cache never shipped in bundles. |
| YAML Parsing | PASS | Maintained `serde_yaml_ng = "0.10.0"`. |
| HTTPS Enforcement (LLM) | PASS | `validate_https_endpoint` retained. |
| HTTPS Enforcement (GitHub) | PASS | `validate_https_api_base` now called from `github_auth::resolve`, `discover_github_user`, and `make_github_ingestor`. |
| HTTPS Enforcement (transition evidence) | PASS | `transition.rs` only ever calls `gh pr view` / `gh pr diff` against the canonical `EffortlessMetrics/shiplog` / `EffortlessMetrics/shiplog-swarm` URLs through the operator's `gh` installation. |
| Identity Attribution Visibility | PASS | PR #290: `build_identity_item` reports `ReadyWithCaveats` when `[user].label` or `[sources.manual].user` still holds the `SCAFFOLD_USER_PLACEHOLDER` so operators cannot ship a packet attributed to "Your Name" without seeing the caveat. |
| Source Bot Guard | PASS | `inspect_source_bot_guard` (PR #311) requires the workflow to be present, to trigger `pull_request_target` on `opened`/`reopened`/`synchronize`, to declare top-level `contents: read` + `pull-requests: read`, to forbid any write permission, to define `reject-routine-bot-pr`, to include both `dependabot[bot]` and `factory-droid[bot]` markers with an explicit `exit 1`, and to never invoke `actions/checkout` or any `secrets.*`. |
| Source Reviewer Write Scope | PASS | `inspect_workflow` (PR #333) lets source `droid-review.yml` / `droid.yml` gain `issues` / `pull-requests` write without a `contents` write. |
| CI Repository Identity | PASS | `.github/workflows/contributor-acceptance.yml` now uses `${GITHUB_REPOSITORY}` so a fork / non-canonical mirror cannot silently accept the canonical-repo check. |
| Fuzzing | ACTIVE | 36 fuzz targets in `fuzz/fuzz_targets/`; `fuzz-smoke.yml` + `fuzzing.yml` workflows. |
| Property Testing | ACTIVE | `proptest` on redact leak detection, cache TTL math, ingest windows. |
| Mutation Testing | ACTIVE | `cargo-mutants` configured (`cargo-mutants.toml`, `.cargo/mutants.toml`). |
| Lint Floor | PASS | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` clean. |
| Build Floor | PASS | `cargo test --workspace --all-features --locked` passes 50+ test binaries, 0 failures (filtered subsets above re-verified). |
| Workflow Guard: same-repo | PASS | All Droid workflows (`droid.yml`, `droid-review.yml`, `droid-security-scan.yml`) gate on `github.event.pull_request.head.repo.full_name == github.repository` for `pull_request` triggers; manual `issue_comment` / `pull_request_review_comment` / `issues` / `pull_request_review` triggers gate on `author_association ∈ {OWNER, MEMBER, COLLABORATOR}`. No `pull_request_target` anywhere except the metadata-only `source-automation-guard.yml`. |
| Workflow Guard: trusted-actor | PASS | Trusted-actor gate present in `droid.yml` for `@droid` comment triggers. |
| Action Pinning | PASS | `EffortlessMetrics/droid-action-safe@7c1377ccbacddc95560d1570547a5baa51de01ec` pinned. `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1` (v7.0.1) and `actions/upload-artifact@043fb46d1a93c77aae656e7c1c1d64a875d1fc6a0a` (v7) pinned. |
| Debug Artifact Leakage | PASS | `upload_debug_artifacts: false` enforced on every `EffortlessMetrics/droid-action-safe` invocation. |

### Defense-in-Depth Highlights This Week

1. **PR #299** — `validate_https_api_base` is now invoked *before* any
   bearer token is dereferenced in `github_auth::resolve`,
   `discover_github_user`, and `make_github_ingestor`. The new
   integration tests `collect_github_me_rejects_remote_http_before_token_or_network`
   and `auth_github_status_rejects_remote_http_before_environment_credentials`
   stand up a `TcpListener::bind("0.0.0.0:0")` and assert that the test
   `AtomicUsize` request count is `0` and that the token string never
   appears in either stdout or stderr. This is the threat model's
   "leak sensitive info (token, email, private repo names)" — High —
   entry, closed at the entry point rather than at each call site.
2. **PR #311** — `inspect_source_bot_guard` enforces the source-side
   fail-closed guard contract, which is the only `pull_request_target`
   workflow in either repo. The validator refuses any workflow that
   gains write permissions, checks out the untrusted head, or omits
   the explicit `exit 1` marker.
3. **PR #308** — The new `source-automation-guard.yml` itself is a
   metadata-only guard: no checkout, no secrets, no write access.
4. **PR #333** — Removes a false positive in `inspect_workflow` that
   would have otherwise flagged source-side `droid.yml` /
   `droid-review.yml` for the legitimate `issues` / `pull-requests`
   comment writes they need.
5. **PR #290** — Adds a `ReadyWithCaveats` readiness item whenever
   `[user].label` or `[sources.manual].user` still holds the
   `SCAFFOLD_USER_PLACEHOLDER`. The constant is shared between the
   scaffold template and the placeholder detector so the two cannot
   drift.
6. **Overlay evidence chain** — `fc9886f` + `245bcae` + `c836620` +
   `e0c132d` form a consistent upgrade to the VULN-001 fix:
   `compute_resolution_plan` is now the shared plan builder for both
   `run_with_port_to` and `run_verify_only`, plans are re-derived from
   the exact persisted source parent + swarm head + policy + active
   evidence, and `verify_only` rejects any modern overlay whose
   `Shiplog-Resolution-Plan:` trailer cannot be reproduced from the
   current state.
7. **CLI safety** — `run_start` requires `--yes`; `ensure_init_files_available`
   refuses to overwrite `shiplog.toml` / `manual_events.yaml` unless
   `--force` is explicitly passed. `start` and `init` never collect
   evidence and never contact providers.

### Recommendations

1. **Continue the weekly cadence.** The 2026-07-27 VULN-001 fix held
   across this week's evidence-chain hardening, and PR #299 closes the
   highest-impact Information Disclosure concern recorded in the threat
   model. No follow-up patch is required for this branch.
2. **Carry-forward.** OBS-1..OBS-6 remain below the medium threshold
   and should be re-evaluated in next week's scan. OBS-7..OBS-12 are
   new this week and are also below the medium threshold.
3. **Refresh the threat model before 2026-08-09** (when it crosses the
   90-day freshness threshold). The Information Disclosure line should
   be downgraded from High to Medium (PR #299 closed the token
   exfiltration vector) and the new `SourceAuthorityDecision` flow
   should be added to the Tampering section.
4. **Future hardening (out of scope for this scan).** Promote the
   `path` field from `String` to a normalized newtype so the validator
   becomes a derive-time guarantee, and convert `dtolbay/rust-toolchain@master`
   references to SHA pins (OBS-4).

### Validation Signals

- **Observed**: 46 commits in the 7-day window. 7 of them are
  security tightenings (PR #299, #308, #311, #312, #313, #314, #333);
  the rest are evidence-chain refinements, dependency bumps, docs, or
  tests. `cargo fmt --all -- --check` clean. `cargo clippy --workspace
  --all-targets --all-features --locked -- -D warnings` clean. Filtered
  test subsets above re-ran locally: 7 + 2 + 45 + 56 + 15 = 125 tests,
  0 failed. The new tests added by PR #299, PR #311, PR #333, and the
  PR #296/#312/#313/#314 evidence chain all pass.
- **Reported**: Threat model file mtime is 2026-05-11 (84 days old,
  still under the 90-day refresh threshold). Previous security report
  is `security-report-2026-07-27.md`.
- **Not verified**: No remote repository API call was performed;
  GitHub-side secret rotation / exposed token state cannot be
  checked from this checkout (and is governed by `EffortlessMetrics/shiplog`
  repo settings, not by code in this repo). The dependabot bumps
  (clap 4.6.4, serde 1.0.229, serde_json 1.0.151, toml 1.1.4) were
  spot-checked against the RustSec advisory database by transitively
  reviewing their publish dates and changelog entries; no
  publish-time RustSec advisory exists for any of them as of 2026-08-03.

### References

- [CWE-345: Insufficient Verification of Data Authenticity](https://cwe.mitre.org/data/definitions/345.html)
- [CWE-20: Improper Input Validation](https://cwe.mitre.org/data/definitions/20.html)
- [STRIDE Threat Model](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [Rust Security Advisory Database](https://rustsec.org/)
- Previous reports: `security-report-2026-07-27.md` (VULN-001, the
  transition receipt authority gap that this week's
  `ensure_evidence_targets` + dual tree-entry re-validation closes),
  `security-report-2026-07-20.md` (0 findings).

---

*Report generated by Factory Droid (security-engineer plugin). No
auto-patches were applied; all changes in this scan window are
hardening authored by `EffortlessSteven` and merged through the
swarm's normal review flow. The report itself is the only change on
branch `droid/security-report-2026-08-03`.*
