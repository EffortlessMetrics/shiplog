# Security Scan Report

**Generated:** 2026-07-27
**Scan Type:** Weekly Scheduled
**Repository:** EffortlessMetrics/shiplog
**Branch:** `droid/security-report-2026-07-27`
**Severity Threshold:** medium
**Scan Window:** 2026-07-20 to 2026-07-27 (7 days)

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

No high-confidence security vulnerabilities at or above the medium threshold
were found in the last seven days of changes. The single scoped commit removes
the `rtk` output-filtering wrapper from live proof-command contracts, updates
two validators to enforce the direct-command form, and adds regression tests.
The changed validators parse and render command strings but do not execute
them, and the newly direct commands are fixed repository inspection and proof
commands with no embedded credentials or untrusted interpolation.

## Scan Results Overview

The scan window contained one commit:
`d88d59a8afd7eee445c3214c5f89ca7ccd50e4de`. The initial shallow checkout made
this commit appear to add the full 825-file tree. The parent commit
`a94bfaf6880c6873d4dd0fce7bf9733fb1b61734` was fetched to recover the exact
change set. The resulting security scope is 13 files, with 261 insertions and
117 deletions.

### Commits Scanned

| SHA | Date | Subject | Files |
|-----|------|---------|-------|
| `d88d59a` | 2026-07-24 | sync: remove RTK from Shiplog proof contracts (#666) | 13 |

### Files Scanned

| File | Change | Security Assessment |
|------|--------|---------------------|
| `.codex/goals/README.md` | Direct-command guidance | No executable behavior; no unsafe command added. |
| `.codex/goals/active.toml` | Twelve proof commands changed from wrapped to direct form | Commands are fixed `git`, `gh`, and `cargo xtask` invocations; no secrets, shell interpolation, destructive flags, or remote mutation. |
| `AGENTS.md` | Development setup commands changed to direct form | Fixed fetch and new-branch commands only; no credential output or destructive operation introduced. |
| `docs/specs/SHIPLOG-SPEC-0010-source-of-truth-stack.md` | Proof command examples updated | Documentation-only command prefix removal. |
| `docs/specs/SHIPLOG-SPEC-0011-shiplog-swarm-cutover-contract.md` | Proof command examples updated | Documentation-only command prefix removal. |
| `docs/status/SUPPORT_TIERS.md` | Proof commands changed to direct `cargo xtask` form | Commands target known local validators and bounded `target/` outputs. |
| `docs/templates/plan-item.md` | Template proof commands changed to direct form | Placeholders only; no executable interpolation. |
| `docs/xtask.md` | Validator contract and examples updated | Documentation matches changed parser behavior. |
| `plans/shiplog-swarm/promotion-runbook.md` | Promotion proof commands changed to direct form | Read-only inspection and local proof commands; release-authority boundaries remain explicit. |
| `xtask/src/tasks/check_goals.rs` | Reject `rtk` as the first command token | String validation only; no subprocess or shell execution added. Archived historical commands remain exempt by design. |
| `xtask/src/tasks/check_support_tiers.rs` | Require `cargo xtask` rather than `rtk cargo xtask` | String parsing only; known subcommand and safe output-path checks remain in place. |
| `xtask/tests/check_goals_direct_commands.rs` | New direct-command regression tests | Test-only temporary files and subprocess invocation of the local `xtask` binary. |
| `xtask/tests/cli.rs` | Fixtures and expectations updated | Test-only direct-command fixtures; no production execution path. |

### STRIDE Assessment

| STRIDE Category | Assessment |
|-----------------|------------|
| Spoofing | No identity, authentication, or credential-resolution behavior changed. |
| Tampering | No production file mutation was added. Generator proof commands remain constrained to `target/` where applicable. |
| Repudiation | Goal, support-tier, and promotion receipts remain explicit; command provenance is still stored in repository-controlled files. |
| Information Disclosure | No secrets or credential values were added. Direct `gh` commands query named public project repositories and do not print tokens. |
| Denial of Service | Validators perform bounded string parsing over repository files; no new loops, network requests, or attacker-sized allocations were introduced. |
| Elevation of Privilege | Changed code does not execute proof-command strings. It only validates or renders them, so no new command-execution boundary is reachable. |

### Candidate Validation

One candidate was explicitly evaluated and rejected as a false positive:

- **Candidate:** Removing the `rtk` wrapper could permit command injection
  through `.codex/goals/active.toml`.
- **Reachability:** The changed validators read command strings, compare tokens,
  and pass the strings to Markdown/TOML report renderers. They do not invoke a
  shell or `std::process::Command` with those values.
- **Exploitability:** No runtime execution sink exists in the changed paths.
  Executing a documented proof command remains an explicit operator or agent
  action against repository-reviewed content.
- **Impact:** No privilege boundary is crossed by the code change.
- **Disposition:** Not a vulnerability. No patch required.

## Critical Findings

None.

## High Findings

None.

## Medium Findings

None.

## Low Findings

None reported. Findings below medium are outside the configured reporting
threshold.

## Patches

No confirmed findings were eligible for automatic patching, so no security fix
commit was created.

## Appendix

### Threat Model

- Version: 2026-05-11
- Location: `.factory/threat-model.md`
- Status: Current, 77 days old on 2026-07-27 and within the 90-day refresh window
- Action: Reused as scan context; regeneration was not required

The existing model identifies external APIs, local files, configuration, and
manual event inputs as trust boundaries. None of those runtime data flows were
changed by the scoped commit.

### Scan Metadata

- Commits Scanned: 1
- Files Scanned: 13
- Diff Size: 261 insertions, 117 deletions
- Scan Duration: 17m 15s
- Skills/Procedures Used: threat-model context, commit security scan, STRIDE analysis, vulnerability validation; security patch generation was not required
- Parent Used for Exact Diff: `a94bfaf6880c6873d4dd0fce7bf9733fb1b61734`
- Head Scanned: `d88d59a8afd7eee445c3214c5f89ca7ccd50e4de`

### Validation Signals

**Observed:**

- `cargo fmt --all -- --check` passed.
- `cargo test -p xtask --locked` passed: 201 unit tests, 3 direct-command integration tests, and 18 CLI integration tests.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` passed.
- `cargo test --workspace --all-features --locked` passed, including workspace unit, integration, and documentation tests.
- `cargo run --quiet -p xtask -- check-support-tiers` passed with 11 linked claims.
- Secret-pattern scanning found no production credentials in the scoped changes.
- The exact 13-file parent-to-head diff was reviewed, including every added command and both changed validator implementations.

**Reported but not attributed to this commit:**

- `cargo run --quiet -p xtask -- check-goals` reports two existing repository
  contract findings for `promotion-cadence`: the referenced current-promotion
  file lacks the expected work-item heading and is not ledgered as a plan
  artifact. The relevant plan reference and ledger state were unchanged by
  `d88d59a`, so this is not a security regression in the weekly scope.

**Not verified:**

- No live provider API calls, credential transmission, release operation, or
  remote mutation was needed for this documentation and validator-only diff.

### References

- [CWE Database](https://cwe.mitre.org/)
- [STRIDE Threat Model](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Rust Security Advisory Database](https://rustsec.org/)

---

*Report generated by Factory Droid. No findings met the configured medium
severity threshold.*
