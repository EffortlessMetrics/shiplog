# File Policy

This document describes how shiplog manages the non-Rust surface of the
repository: shell scripts, workflow YAML, fixtures, dictionaries, dotfiles,
config examples, fuzz corpora, and so on.

It is part of the [Rust 1.95 / 0.5.0 quality
rollout](ci/rust-1.95-rollout.md). The `policy/non-rust-allowlist.toml` and
its companion ledger skeletons land in PR #141; the checkers land in
PR #149. Everything below describes the intended steady state.

## Intent

A repo's non-Rust surface is where sneaky risk lives:

- A new shell script bypasses release proof.
- A new GitHub Actions workflow grants secrets to a third party.
- A generated file becomes the source of truth without anyone noticing.
- A dependency creeps in via a workflow change rather than `Cargo.toml`.

Each category should be **explicitly receipted**: present + owner + reason +
expiry.

## Ledgers

```text
policy/non-rust-allowlist.toml          All non-Rust files in repo
policy/generated-allowlist.toml         Files marked linguist-generated=true
policy/executable-allowlist.toml        Files with the executable bit
policy/dependency-surface-allowlist.toml Anything declaring third-party deps (Cargo.toml, package-lock-like, action versions)
policy/workflow-allowlist.toml          GitHub Actions workflow files and their permission scopes
policy/process-allowlist.toml           Scripts that spawn external processes
policy/network-allowlist.toml           Code/scripts that make network calls
policy/publish-allowlist.toml           0.7 public package publish order
```

All ledgers share the schema documented in
[`POLICY_ALLOWLISTS.md`](POLICY_ALLOWLISTS.md).

## Companion docs

- [`CLIPPY_POLICY.md`](CLIPPY_POLICY.md) — Clippy lint ledgers.
- [`NO_PANIC_POLICY.md`](NO_PANIC_POLICY.md) — panic-family baseline.
- [`POLICY_ALLOWLISTS.md`](POLICY_ALLOWLISTS.md) — common schema and
  suppression style.

## Commands

```bash
cargo xtask non-rust inventory         # report all non-Rust files
cargo xtask non-rust propose           # write a starter allowlist for review
cargo allow check --compat --kind non-rust --mode no-new
                                       # CI gate for the non-Rust master ledger
cargo xtask check-generated            # generated files allowlist
cargo xtask check-executable-files     # executable bit allowlist
cargo xtask check-dependency-surfaces  # third-party dep declarations
cargo xtask check-workflows            # workflow permission/secret scopes
cargo xtask check-process-policy       # process-spawning surfaces
cargo xtask check-network-policy       # network-calling surfaces
cargo xtask policy-report              # combined human summary
```

PR #149 starts these in `--mode advisory`. Promotion to blocking happens
after ledger cleanup, in a follow-up release.

## Surfaces shiplog needs to receipt

Existing surfaces that the initial allowlist will cover:

```text
.github/workflows/*.yml
  bdd-testing.yml, ci.yml, coverage.yml, droid.yml, droid-review.yml,
  droid-security-scan.yml, fuzzing.yml, mutation-testing.yml,
  property-testing.yml, release.yml, security.yml

scripts/*.{sh,ps1}
  package-proof.sh, package-boundary-audit.sh, package-version-audit.sh,
  publish-dry-run.sh, verify-release.sh,
  release-install-smoke.{sh,ps1}, demo-review-rescue.{sh,ps1}

release policy
  policy/publish-allowlist.toml

config/baseline files
  deny.toml, codecov.yml, rust-toolchain.toml,
  Cargo.toml, Cargo.lock, .gitattributes, .gitignore

docs/
  README.md, CHANGELOG.md, RELEASE_HANDOFF_*.md, all docs/**/*.md

fixtures and examples
  contracts/schemas/*.json, examples/**, tests/**/*.{json,yaml,jsonl}

fuzz/
  fuzz/Cargo.toml, fuzz/fuzz_targets/*, fuzz/dictionaries/*.dict,
  fuzz/corpus/** (gitignored), fuzz/artifacts/** (gitignored)
```

Each gets a category and a reason in the appropriate ledger.

## Shell-script-as-wrapper rule

shiplog has shell scripts that perform real release-proof work today
(`package-proof.sh`, `publish-dry-run.sh`, `package-boundary-audit.sh`,
`package-version-audit.sh`, `verify-release.sh`). These will move into
`xtask` over time (PR #143 onward), but the policy is:

- The shell entry points keep working as wrappers (so existing CI / docs / muscle
  memory continues to function) until `xtask` parity is proven for every step
  the script performs.
- A PR cannot delete a release-proof script and replace it with `xtask` in the
  same PR. Add the `xtask` task first, prove it produces the same artifact,
  then in a separate follow-up shrink the script to a one-line wrapper.
- A PR can never remove a release-proof obligation without replacing it with
  an equivalent obligation in another lane.

This rule is enforced by review, not by code. The `xtask policy-report`
output makes it easy to see whether a PR has added a check + removed an
equivalent script (allowed), or just removed a check (disallowed).

## Workflow file scope

Workflow YAML lives in `policy/workflow-allowlist.toml` with extra fields:

```toml
[[entry]]
path = ".github/workflows/release.yml"
owner = "release"
reason = "Tag-triggered release pipeline; multi-platform binary build + crates.io readiness."
expiry = "permanent"
permissions = ["contents:write", "id-token:write"]
secrets_used = ["GITHUB_TOKEN"]
external_actions = [
  "actions/checkout@v6.0.2",
  "dtolnay/rust-toolchain@master",
  "actions/upload-artifact@v7",
  "actions/download-artifact@v8",
  "actions/cache@v5",
  "softprops/action-gh-release@v3",
  "Swatinem/rust-cache@v2",
  "taiki-e/install-action@v2",
]
```

`check-workflows` verifies the declared list matches the file. A new
`uses:` line in any workflow is a policy event, not just a code change.

## Network and process scopes

Two narrower ledgers:

- `policy/process-allowlist.toml` — anywhere we shell out (`std::process::Command`,
  shell `exec`, `cargo install` invocations, etc.) — captures binary,
  reachable arguments shape, and network-or-not boolean.
- `policy/network-allowlist.toml` — anywhere we make a network call (`reqwest`,
  `curl`, `cargo install`, GitHub Actions network reach) — captures
  destination, auth requirement, and lane (build / test / release).

These exist primarily to make supply-chain review easier. A new outbound
endpoint should be a deliberate addition with an owner.

## See also

- [`CLIPPY_POLICY.md`](CLIPPY_POLICY.md)
- [`NO_PANIC_POLICY.md`](NO_PANIC_POLICY.md)
- [`POLICY_ALLOWLISTS.md`](POLICY_ALLOWLISTS.md)
- [`ci/rust-1.95-rollout.md`](ci/rust-1.95-rollout.md)
