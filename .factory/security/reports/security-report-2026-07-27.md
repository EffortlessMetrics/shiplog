# Security Scan Report

**Generated:** 2026-07-27
**Scan Type:** Weekly Scheduled
**Repository:** EffortlessMetrics/shiplog-swarm
**Branch:** `droid/security-report-2026-07-27`
**Severity Threshold:** medium
**Scan Window:** 2026-07-20 to 2026-07-27 (7 days)

## Executive Summary

| Severity | Count | Auto-fixed | Manual Required |
|----------|-------|------------|-----------------|
| CRITICAL | 0     | 0          | 0               |
| HIGH     | 0     | 0          | 0               |
| MEDIUM   | 1     | 1          | 0               |
| LOW      | 0     | 0          | 0               |

**Total Findings (>= medium):** 1
**Auto-fixed:** 1
**Manual Review Required:** 0

One medium-severity finding was identified this week in the new transition
receipt authority added in commit `01f7e4ef`. The fix is included on this
branch (commit `a5d9b5c`); the same branch ships this report.

## Critical Findings

None.

## High Findings

None.

## Medium Findings

### VULN-001: Transition receipt authority can be granted to paths the source PR did not actually change

| Attribute | Value |
|-----------|-------|
| **Severity** | MEDIUM |
| **STRIDE Category** | Tampering |
| **CWE** | CWE-345 (Insufficient Verification of Data Authenticity) |
| **File** | `xtask/src/tasks/transition.rs:148` (pre-fix) |
| **Introduced by** | `01f7e4ef` (`feat(xtask): earn transition divergence authority from checked evidence (#277)`, 2026-07-25) |
| **Status** | Patched on this branch (commit `a5d9b5c`) |
| **Reachable from user input** | No (the receipt is committed in `plans/shiplog-swarm/promotion-state.toml` and validated through `cargo xtask promote`; no remote attacker model) |
| **Exploitability** | Conditions: maintainer-level access to add a transition receipt, or a legitimate receipt with a path error |
| **Impact** | The promotion overlay silently replaces the source's content at any path the manifest names with the swarm's content, while the alignment check reports the difference as approved source-only drift |

**Description:**

The transition receipt authority added in PR #277 grants the
`missing_in_swarm` disposition to a path purely on the strength of the
receipt's source PR identity (PR number + merge SHA). The receipt
validator confirms the PR is merged, the recorded SHA is its merge commit,
and the merge is reachable from the source ref, but it never confirms that
the path the receipt names is actually in the source PR's diff. A receipt
that names a real, well-formed source PR can therefore list arbitrary
paths the PR did not touch.

`derive_authority` then adds those paths to `TransitionAuthority.source_only`,
and `ensure_source_only_alignment` (in `xtask/src/tasks/promote.rs`) accepts
divergence on them as approved source-only drift.

`prepare_source_overlay` is the symmetry break: it only restores paths from
`policy/source-only-paths.toml` into the overlay, not transition-approved
paths. The overlay therefore silently overwrites the source's content at
transition-receipted paths with the swarm's content, even though the
alignment check reports the difference as "approved".

A receipt for any real source PR can therefore silently replace
security-relevant source content (e.g. `.github/workflows/security.yml`,
`policy/automation-authority.toml`, `deny.toml`) in a release overlay
that is about to be pushed and reviewed on the source side. The receipt
validation passes, the alignment check passes, and the only artifact a
reviewer sees is the overlay diff itself, where the "swarm" content now
wins.

**Evidence (pre-fix `xtask/src/tasks/transition.rs:check_path`):**

```rust
TransitionDisposition::MissingInSwarm => {
    if !path.swarm_chain.is_empty() {
        bail!("missing_in_swarm must not name swarm PRs");
    }
    // Source changed it and swarm has not caught up yet, so the
    // difference is one-sided until a promotion reconciles it.
    authority.source_only.insert(path.path.clone());
    return Ok(());
}
```

**Exploit sketch (concrete):**

1. A maintainer (or compromised account) adds a transition receipt for a
   real source PR that changed only `Cargo.lock`, listing the path
   `policy/automation-authority.toml` with `disposition =
   "missing_in_swarm"`.
2. `validate_transitions` accepts the receipt: `source_pr` is a real
   receipt, `source_merge_sha` is a real 40-character SHA, and the path
   parser at the time only rejects empty strings.
3. `derive_authority` adds `policy/automation-authority.toml` to
   `TransitionAuthority.source_only`.
4. `ensure_source_only_alignment` adds the same path to the
   `approved` set, so the alignment check passes.
5. `prepare_source_overlay` does not restore the path from source, so
   the overlay's version of `policy/automation-authority.toml` is the
   swarm's version. The PR review sees "swarm contents replace source
   contents" without any reason to suspect the receipt authorised it.

**Fix applied (commit `a5d9b5c`):**

1. `xtask/src/tasks/transition.rs`: Each `missing_in_swarm` path is now
   required to actually appear in the source PR's diff. The path must
   match a `diff --git` section in the patch returned by
   `gh pr diff --patch`; receipts that name paths the source PR did not
   touch are rejected with `source PR ... does not touch this path`.
2. `xtask/src/tasks/promotion_state.rs`: A new `validate_transition_path`
   helper mirrors the validator already used for
   `policy/source-only-paths.toml`. It rejects empty, whitespace-padded,
   absolute, backslash-separated, or `..`-laden path values, so the
   path cannot escape the repository through the manifest.
3. `xtask/src/tasks/promote.rs`: The overlay now restores
   transition-approved source-only paths from source, in addition to
   the permanent policy paths. The `overlay_source_only_paths` set is
   `source_only_paths ∪ transition_authority.source_only` (deduped
   through a `BTreeSet`), so the alignment check's approval and the
   overlay's content actually agree.

**Patch (`xtask/src/tasks/transition.rs`):**

```diff
         TransitionDisposition::MissingInSwarm => {
             if !path.swarm_chain.is_empty() {
                 bail!("missing_in_swarm must not name swarm PRs");
             }
+            // The receipt is an authority grant for a specific source change,
+            // not a free-form allowlist. Require the source PR patch to contain
+            // this path before allowing it to authorize source-only divergence.
+            patch_for_path(source_patch, &path.path).with_context(|| {
+                format!(
+                    "source PR {} does not touch this path",
+                    entry.source_pr.as_str()
+                )
+            })?;
             // Source changed it and swarm has not caught up yet, so the
             // difference is one-sided until a promotion reconciles it.
             authority.source_only.insert(path.path.clone());
             return Ok(());
         }
```

**Patch (`xtask/src/tasks/promote.rs`):**

```diff
         &state.transition,
     )?;
+    // Transition-authorized source-only paths must be restored from source into
+    // the overlay just like permanent policy paths. Otherwise the alignment
+    // check would approve the divergence and the subsequent checkout would
+    // silently replace the source-only change with the older swarm tree.
+    let overlay_source_only_paths = source_only_paths
+        .iter()
+        .cloned()
+        .chain(transition_authority.source_only.iter().cloned())
+        .collect::<BTreeSet<_>>()
+        .into_iter()
+        .collect::<Vec<_>>();
     // Commits the ancestry walk may step over: approved governance, plus source
@@ ...
         &source_head,
         &swarm_sha,
-        &source_only_paths,
+        &overlay_source_only_paths,
         inputs.dry_run,
     )?;
```

**Patch (`xtask/src/tasks/promotion_state.rs`):**

```diff
         for path in &entry.path {
-            if path.path.trim().is_empty() {
-                bail!("transition {} has an empty path", entry.source_pr);
-            }
+            validate_transition_path(&entry.source_pr, &path.path)?;
             if !seen_paths.insert(path.path.as_str()) {
                 ...
             }
         }
@@ ...
+/// Transition receipt paths are later passed as repository path arguments,
+/// so they must stay within the repository and name exactly one normalized path.
+fn validate_transition_path(source_pr: &str, value: &str) -> Result<()> {
+    if value.trim().is_empty()
+        || value != value.trim()
+        || value.starts_with('/')
+        || value.contains('\\')
+        || value
+            .split('/')
+            .any(|part| part.is_empty() || part == "." || part == "..")
+    {
+        bail!(
+            "transition {source_pr} path {value:?} must be a normalized repository-relative path"
+        );
+    }
+    Ok(())
+}
```

**Regression test added (`xtask/src/tasks/transition.rs`):**

```rust
#[test]
fn missing_in_swarm_requires_the_source_pr_to_touch_the_path() {
    let patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
    let port = StubPort::new().merged(
        &format!("{SOURCE}#657"),
        "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
        &patch,
    );
    let mut transition = entry(TransitionDisposition::MissingInSwarm, &[]);
    transition.path[0].path = "not-touched.txt".to_string();
    let error = derive_authority(&port, Path::new("."), &refs(), &[transition])
        .expect_err("a receipt must not authorize a path absent from its source PR");
    assert!(
        format!("{error:#}").contains("does not touch this path"),
        "unexpected error: {error:#}"
    );
}
```

**Validation:**

- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` clean.
- `cargo test --workspace --all-features --locked` — 50+ test binaries,
  all `ok`, 0 failed. The new test
  `tasks::transition::tests::missing_in_swarm_requires_the_source_pr_to_touch_the_path`
  is part of the 298 xtask unit tests and passes. The pre-existing
  `missing_in_swarm_grants_source_only_until_consumed`,
  `conflicting_disposition_blocks`,
  `equivalent_requires_matching_patch_id_for_that_path`,
  `supersession_requires_a_contiguous_chain_from_the_source_start`, and
  the rest of the transition suite continue to pass.

**Confidence:** HIGH. The receipt authority is bounded by an explicit
allowlist (the source PR's diff) plus an explicit restore step in the
overlay; the only trust remaining is the recorded SHA, which the
existing `check_merged_at` already validates against the forge.

## Low Findings

None at or above the medium severity threshold.

### Observations Below Threshold

| ID | Class | File | Note |
|----|-------|------|------|
| OBS-1 | Loopback HTTP exception | `apps/shiplog/src/ingest/github.rs:1497` | VULN-002 fix from 2026-07-13 remains in place. `validate_https_api_base` permits HTTP only for genuine loopback addresses; the exception cannot be reached by remote attackers because `Url::parse` preserves the literal host string before DNS resolution. Carried forward. |
| OBS-2 | User-controlled regex | `apps/shiplog/src/main.rs:13703` | `RegexBuilder::new(pattern)` for `workstreams split --matching`. Self-DoS only; not exploitable by an external attacker. Carried forward. |
| OBS-3 | Markdown link escaping | `apps/shiplog/src/render/md/receipt.rs` | URLs from API responses or `manual_events.yaml` are interpolated unescaped into `[label](url)`. Output is a file (`packet.md`) opened locally; impact is limited to local renderer behavior. Carried forward. |
| OBS-4 | `dtolnay/rust-toolchain@master` | `.github/workflows/*.yml` (12 occurrences) | Mutable ref rather than a SHA pin. Mitigated by `rust-toolchain.toml` pinning the actual Rust version and the action's wide audit. Carried forward. |
| OBS-5 | `bundle/mod.rs::walk_files` follows symlinks | `apps/shiplog/src/bundle/mod.rs:162` | The bundle is only ever produced from output directories the same operator owns. Carried forward as low-severity defensive-hardening item. |
| OBS-6 | `transition.rs::lock_transitions` parses TOML per line | `xtask/src/tasks/transition.rs:469` | The line-oriented parser calls `line.parse::<toml::Table>()` per `+`/`-` line. The parsed value is thrown away after extracting the literal `name`/`version` string. Not a security issue, but allocates on every `Cargo.lock` diff. Note added for follow-up. |

## Appendix

### Threat Model

- Version: 2026-05-11 (unchanged this scan; still within the 90-day
  freshness window)
- Location: `.factory/threat-model.md`
- Status: **Current** (aged 77 days, well under the 90-day refresh
  threshold)
- Action taken: re-used as scan context; no regeneration required.
- The Information Disclosure category in the threat model already names
  "leak sensitive info (token, email, private repo names)" as a High
  concern. VULN-001 (transition receipt authority, 2026-07-27) is the
  code-path instance of the broader Tampering concern ("Modify cached
  API responses to alter output") that the threat model records as
  "Severity: Low" — the new transition surface raised it from Low to
  Medium for this specific path.

### Commits Scanned (strict 7-day window)

| SHA | Date (UTC) | Subject | Files |
|------|------------|---------|-------|
| `4a33d91` | 2026-07-25 | `deps: bump anyhow from 1.0.103 to 1.0.104 (#268)` | `Cargo.lock` |
| `01f7e4e` | 2026-07-25 | `feat(xtask): earn transition divergence authority from checked evidence (#277)` | `plans/shiplog-swarm/current-promotion.md`, `plans/shiplog-swarm/promotion-state.toml`, `xtask/src/tasks/mod.rs`, `xtask/src/tasks/promote.rs`, `xtask/src/tasks/promotion_state.rs`, `xtask/src/tasks/transition.rs` (VULN-001 introduced) |
| `6737b49` | 2026-07-25 | `feat(xtask): overlay source promotions in an owned worktree (#275)` | `xtask/src/tasks/promote.rs` |
| `1ca35e9` | 2026-07-25 | `chore: remove RTK from Shiplog proof contracts (#274)` | `.codex/goals/README.md`, `.codex/goals/active.toml`, `AGENTS.md`, 3 docs, `xtask/src/tasks/check_goals.rs`, `xtask/src/tasks/check_support_tiers.rs`, 2 test files |
| `92a6158` | 2026-07-24 | `feat(xtask): add read-only --verify-only post-merge mode to promote (#272)` | `docs/xtask.md`, `xtask/src/cli.rs`, `xtask/src/tasks/promote.rs` |
| `ab3bddf` | 2026-07-23 | `docs(governance): document automation-authority boundary and emergency path (#271)` | `docs/README.md`, `docs/governance/automation-authority.md`, `policy/README.md` |
| `9723910` | 2026-07-23 | `deps: bump tokio from 1.53.0 to 1.53.1 (#265)` | `Cargo.lock` (1.53.1 fixes a Windows MSRV regression in the signal handler and a timer race; no RustSec advisory applies) |
| `27cc2d6` | 2026-07-23 | `deps(swarm): align Tokio with source for transition (#269)` | `Cargo.lock`, `Cargo.toml` |
| `f9442ae` | 2026-07-22 | `chore(promotion): refresh pending swarm range state through #261 (#262)` | `plans/shiplog-swarm/current-promotion.md`, `plans/shiplog-swarm/promotion-state.toml` |
| `7371cbb` | 2026-07-22 | `fix(github_activity): validate run_ref segment shape (#261)` | `apps/shiplog/src/github_activity.rs` (closes a path-traversal observation from prior scans) |
| `55a0846` | 2026-07-21 | `fix(security): Security scan report - 2026-07-20 (0 findings) (#260)` | `.factory/security/reports/security-report-2026-07-20.md` |

### Surfaces Reviewed

| Surface | Purpose | Result |
|---------|---------|--------|
| `xtask/src/tasks/transition.rs` | Transition receipt authority derivation | VULN-001 found, patched |
| `xtask/src/tasks/promotion_state.rs` | Bounded `promotion-state.toml` validator + generator | VULN-001 hardening applied (path validator) |
| `xtask/src/tasks/promote.rs` | `cargo xtask promote` orchestrator | VULN-001 hardening applied (overlay path union) |
| `xtask/src/tasks/check_goals.rs` | `.codex/goals/active.toml` validator | PASS |
| `xtask/src/tasks/check_support_tiers.rs` | `docs/status/SUPPORT_TIERS.md` validator | PASS |
| `xtask/src/cli.rs` | `cargo xtask` clap surface | PASS |
| `apps/shiplog/src/github_activity.rs` | GitHub activity harvest orchestration (run_ref validation from #261) | PASS |
| `Cargo.lock`, `Cargo.toml` | Dependency manifest | PASS — Tokio 1.53.1 not affected by any current RustSec advisory; anyhow 1.0.104 is the patched version for RUSTSEC-2026-0190 (carried in the `>= 1.0.103` range from the 2026-06-29 fix) |
| `plans/shiplog-swarm/promotion-state.toml` | Current promotion manifest | PASS — manifest at HEAD is valid and free of the VULN-001 pattern (transition block absent) |
| `policy/source-only-paths.toml` | Permanent source-only path allowlist | PASS |
| `.factory/threat-model.md` | Living threat model | Current |

### STRIDE Threat Model Assessment

| STRIDE Category | Assessment |
|-----------------|------------|
| Spoofing | LOW RISK. Identities flow through HMAC-SHA256 aliasing (`apps/shiplog/src/redact/alias.rs`); bearer tokens are sourced from env vars or `--token`. The new `transition.rs` evidence checks (`check_merged_at`) refuse to let a record pretend to be a different merge commit: full 40-character SHA match is required, not a prefix. |
| Tampering | MEDIUM RISK. VULN-001 was a tampering vector (this report). Patched in commit `a5d9b5c`. All other ledger surfaces remain parameterized: `cache/sqlite.rs` continues to use `rusqlite::params!` in every query site, and the `promote` overlay worktree is the only place overlay objects are written. |
| Repudiation | LOW RISK. `ledger.events.jsonl` is append-only with SHA-256 `EventId`s (`shiplog::ids`). The new `promote` flow writes a machine-readable `promote-receipt.json` and a `promotion-body.md` next to every plan; receipts are deterministic for a given repository state. |
| Information Disclosure | LOW RISK. VULN-002 (`validate_https_api_base` in `apps/shiplog/src/ingest/github.rs`, wired in at `apps/shiplog/src/main.rs`) remains in place. The new `transition.rs` evidence path adds two `gh` calls per receipted PR (`pr view --json` and `pr diff --patch`) and one local `git merge-base --is-ancestor` call; none widen remote-attacker reachability. |
| Denial of Service | LOW RISK. The overlay worktree is claimed with a unique `pid+time` nonce and bounded to 64 retries; cleanup is `Drop`-driven on every exit path. The transition evidence fetches at most `(transitions x 1 source view + 1 source patch + chains x (1 swarm view + 1 swarm patch) + 1 source merge-base + chains x 1 swarm merge-base)` `gh`/`git` calls per `cargo xtask promote`; bounded by the size of the `transition` block. |
| Elevation of Privilege | LOW RISK. The workspace lint floor (`unsafe_code = "deny"`) is unchanged; the new code adds no `unsafe` and no `Command::new` calls with user-supplied argv. The transition evidence checks all happen before any `authority` field is set, so an unverified receipt can never widen the granted set. |

### Security Controls Verified

| Control | Status | Evidence |
|---------|--------|----------|
| Secrets Management | PASS | `.github/workflows/*.yml` reference `secrets.*` with branch / repo scoping; no plaintext tokens in repo. `main.rs::resolve_redaction_key` only reports `RedactionKeySource`. The new `transition.rs::check_merged_at` compares the recorded SHA against the forge-reported merge commit, so an attacker who tries to swap the recorded SHA fails closed. |
| SQL Injection | PASS | `cache/sqlite.rs` uses `rusqlite::params!` in all 9 query sites. |
| Command Injection | PASS | The new `transition.rs::system_patch_id` invokes `git patch-id --stable` with a hardcoded argv; the patch content is the only stdin. The `promote.rs` overlay worktree uses hardcoded argv vectors and environment-passed `GIT_OBJECT_DIRECTORY` / `GIT_ALTERNATE_OBJECT_DIRECTORIES`. |
| Unsafe Code | PASS | `[workspace.lints.rust] unsafe_code = "deny"` and the new code adds no `unsafe` blocks. |
| Unsafe Regex | PASS | `regex = "1.12.3"` (linear-time engine). The transition evidence path does not compile any user-supplied pattern. |
| Input Validation | PASS | `validate_transition_path` mirrors the `policy/source-only-paths.toml` validator. `validate_full_sha` already required 40 hex chars; the new check rejects empty paths. |
| Path Traversal (writes) | PASS | `bundle/mod.rs::write_zip` uses `path.strip_prefix(out_dir)` so the relative entry name cannot escape the run directory. The new overlay path union is normalized by the same validator and only ever passed to `git checkout <sha> -- <path>` and `git rm -r --force --ignore-unmatch -- <path>` against paths inside the worktree. |
| Path Traversal (reads) | N/A | All read paths come from operator-supplied CLI args or from the `transition.path` field, which the new validator now restricts to normalized repository-relative paths. |
| Redaction | PASS | Three profiles; deterministic HMAC-SHA256; alias cache never shipped in bundles. |
| YAML Parsing | PASS | Maintained `serde_yaml_ng = "0.10.0"`. |
| HTTPS Enforcement (LLM) | PASS | VULN-001 from 2026-06-29 retained. |
| HTTPS Enforcement (GitHub) | PASS | VULN-002 from 2026-07-13 retained. |
| HTTPS Enforcement (transition evidence) | PASS | `transition.rs` only ever calls `gh pr view` / `gh pr diff` against the canonical `EffortlessMetrics/shiplog` / `EffortlessMetrics/shiplog-swarm` URLs through the operator's `gh` installation. |
| Fuzzing | ACTIVE | 36 fuzz targets in `fuzz/fuzz_targets/`; `fuzz-smoke.yml` + `fuzzing.yml` workflows. |
| Property Testing | ACTIVE | `proptest` on redact leak detection, cache TTL math, ingest windows. |
| Mutation Testing | ACTIVE | `cargo-mutants` configured (`cargo-mutants.toml`, `.cargo/mutants.toml`). |
| Lint Floor | PASS | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` clean. |
| Build Floor | PASS | `cargo test --workspace --all-features --locked` passes 50+ test binaries, 0 failures. |
| Workflow Guard: same-repo | PASS | All Droid workflows (`droid.yml`, `droid-review.yml`, `droid-security-scan.yml`) gate on `github.event.pull_request.head.repo.full_name == github.repository` for `pull_request` triggers; manual `issue_comment` / `pull_request_review_comment` / `issues` / `pull_request_review` triggers gate on `author_association ∈ {OWNER, MEMBER, COLLABORATOR}`. No `pull_request_target` anywhere. |
| Workflow Guard: trusted-actor | PASS | Trusted-actor gate present in `droid.yml` for `@droid` comment triggers. |
| Action Pinning | PASS | `EffortlessMetrics/droid-action-safe@7c1377ccbacddc95560d1570547a5baa51de01ec` pinned. `actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0` (v7.0.0) and `actions/upload-artifact@043fb46d1a93c77aae656e7c1c1d64a875d1fc6a0a` (v7) pinned. |
| Debug Artifact Leakage | PASS | `upload_debug_artifacts: false` enforced on every `EffortlessMetrics/droid-action-safe` invocation. |

### Recommendations

1. **Apply the VULN-001 patch via the included PR** — The fix is
   already on this branch (`a5d9b5c`). Reviewers should confirm the
   receipt authority now requires the path to appear in the source PR's
   diff (`missing_in_swarm_requires_the_source_pr_to_touch_the_path` is
   the regression test), and the overlay restores transition-approved
   paths from source so the alignment check and the overlay content
   agree.
2. **Consider promoting the `path` field from `String` to a normalized
   newtype everywhere a receipt is read** — would make the validator a
   derive-time guarantee rather than a runtime check. Out of scope for
   this scan.
3. **Convert `@master` to SHA pins** — OBS-4. Carried forward as a
   follow-up.
4. **Defense-in-depth path-traversal sanitizers** — OBS-5 (prior scans).
   The current surfaces are below the medium threshold because they
   require the operator to already control the filesystem, but explicit
   `Path::components` checks would close the gap entirely.

### Validation Signals

- **Observed**: 1 commit in the past 7 days introduced
  `transition.rs` (`01f7e4ef`). The new code is small (~870 lines) and
  reachable only from `cargo xtask promote`, which requires an operator
  with local checkout and `gh` credentials. `git status` is clean
  after the patch; `cargo fmt` / `cargo clippy` / `cargo test` all pass
  locally. 4,000+ tests pass workspace-wide, 0 failed.
- **Reported**: Threat model file mtime is 2026-05-11 (current under
  the 90-day refresh rule); previous security report is
  `security-report-2026-07-20.md`.
- **Not verified**: No remote repository API call was performed;
  GitHub-side secret rotation / exposed token state cannot be checked
  from this checkout (and is governed by `EffortlessMetrics/shiplog`
  repo settings, not by code in this repo).

### References

- [CWE-345: Insufficient Verification of Data Authenticity](https://cwe.mitre.org/data/definitions/345.html)
- [CWE-20: Improper Input Validation](https://cwe.mitre.org/data/definitions/20.html)
- [STRIDE Threat Model](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [Rust Security Advisory Database](https://rustsec.org/)
- [RustSec advisory RUSTSEC-2026-0190 (anyhow `downcast_mut` unsound, fixed in 1.0.103)](https://rustsec.org/advisories/RUSTSEC-2026-0190.html)

---

*Report generated by Factory Droid (security-engineer plugin). The fix
for VULN-001 is included as commit `a5d9b5c` on branch
`droid/security-report-2026-07-27`.*
