# Tooling Substrate Standard

Shiplog standardizes on a small upstream tooling substrate, then exposes the
repo contract through `xtask` and policy ledgers. Upstream tools are the engine
room; `cargo xtask ...` remains the public control surface for humans, agents,
and CI.

## Control-plane rule

```text
Do not make upstream tools the repo's public control surface.
Make xtask the repo surface.
Make upstream tools the engine room.
```

This keeps policy encoded in one repo-shaped interface instead of scattering
command semantics across workflow YAML, shell scripts, and agent prompts.

## Standard substrate

| Plane | Upstream substrate | Shiplog-facing role |
| --- | --- | --- |
| Repo orchestration | `xtask` | Stable public command surface and policy aggregation |
| Source exceptions | `cargo-allow` | Exception receipts and allowlist evidence |
| Syntax / codemod | `ast-grep`; rust-analyzer crates for Rust authority | Structural candidates, codemods, and Rust-specific source identity checks |
| Workspace graph | `cargo_metadata`; `guppy` | Package inventory, reverse-dependency closure, feature/risk routing |
| Test execution | `cargo-nextest`; `cargo test --doc` | PR tests, partitioning, JUnit, doctests |
| Coverage | `cargo-llvm-cov` | Execution-surface measurement and coverage artifacts |
| Mutation | `ripr`; `cargo-mutants` | PR-time static mutation-exposure and targeted/nightly runtime mutation backstop |
| Unsafe / UB | `unsafe-review`; Miri | Unsafe-contract reviewability and targeted concrete UB witnesses |
| Dependency policy | `cargo-deny`; `cargo-vet`; RustSec / `cargo-audit`; `cargo-auditable` | Dependency trust, advisories, release binary auditability |
| Public API / release | `cargo-semver-checks`; rustdoc JSON | Semver compatibility and custom API-surface inventories |
| Workflow policy | `actionlint`; `zizmor` | GitHub Actions correctness and security posture |
| Text/config hygiene | `taplo`; `typos`; Markdown link/style tooling | TOML, spelling, Markdown, and link hygiene |
| Workspace hygiene | `cargo-udeps` scheduled; `cargo-hakari` only when justified | Unused-dependency checks and large-workspace duplicate-build relief |
| CI cache | `Swatinem/rust-cache`; `sccache` only when justified | Default Rust cache and optional large-workspace remote cache |

## Candidate versus authority

`ast-grep` is the default syntax-aware scanner for fast structural discovery,
workflow/source pattern checks, codemod candidates, non-Rust policy probes, and
agent worklists. It should not be the final authority when a policy needs Rust
semantic identity.

```text
ast-grep finds candidates.
Rust-aware tooling decides authority.
```

Use `cargo_metadata` for basic Cargo workspace metadata and `guppy` when policy
needs graph queries such as changed-crate reverse dependency closure,
risk-pack expansion, publish ordering, feature routing, or CI lane selection.
Use rust-analyzer crates when source selectors must survive formatting and
line-number churn.

## PR, nightly, and release lane posture

The substrate is intentionally tiered by cost:

- **Default PR lanes** should prefer cheap, bounded proof: `cargo xtask`
  ledger checks, `cargo-nextest` for selected tests, `cargo test --doc`, `ripr`
  for static mutation-exposure, and workflow/config linting where installed.
- **Targeted PR lanes** should expand when a risk pack, label, or finding routes
  extra proof: targeted `cargo-mutants`, targeted Miri witnesses, coverage, BDD,
  property, fuzz, or broader package tests.
- **Nightly and release lanes** should run the expensive backstops: broader
  mutation, broader Miri, `cargo-udeps`, release coverage snapshots,
  semver checks, supply-chain audits, and release-binary auditability checks.

Coverage is execution-surface evidence, not a correctness claim. `ripr` shifts
mutation-shaped signal left by identifying static mutation-exposure and weak
oracle candidates; `cargo-mutants` remains the runtime mutation backstop. The
unsafe stack has the same split: `unsafe-review` checks whether an unsafe seam
has a reviewable contract, guard, test reach, and witness route; Miri checks
concrete executions for categories of undefined behavior.

## Stable wrapper surface

Shiplog should grow toward these repo-facing wrappers, even when the engine-room
implementation changes:

```bash
cargo xtask check-pr
cargo xtask fix-pr
cargo xtask pr-summary

cargo xtask allow-check
cargo xtask allow-diff
cargo xtask ripr-pr
cargo xtask unsafe-review-pr

cargo xtask test-pr
cargo xtask coverage
cargo xtask mutation-targeted
cargo xtask miri-targeted

cargo xtask check-deps
cargo xtask check-supply-chain
cargo xtask semver-check
cargo xtask check-workflows
cargo xtask check-toml
cargo xtask policy-report
```

These names are the desired repo contract, not a claim that every wrapper is
implemented today. Existing implemented commands are documented in
[`docs/xtask.md`](../xtask.md).

## Install baseline

Local developer and CI images should prefer locked installs or pinned release
binaries for the baseline tools:

```bash
cargo install cargo-allow --locked
cargo install ripr --locked
cargo install unsafe-review --locked
cargo install cargo-nextest --locked
cargo install cargo-deny --locked
cargo install cargo-llvm-cov --locked
cargo install cargo-semver-checks --locked
cargo install cargo-mutants --locked
cargo install cargo-audit --locked
cargo install taplo-cli --locked
cargo install typos-cli --locked
```

External binaries should be provisioned by pinned release installers or package
manager locks where available:

```text
ast-grep
actionlint
zizmor
markdownlint-cli2
lychee or markdown-link-check
```

Nightly-only and scheduled tools stay out of the ordinary PR tax:

```bash
cargo +nightly install cargo-udeps --locked
rustup +nightly component add miri
```

## Non-goals

- Do not standardize Semgrep as the repo control-plane scanner; use `ast-grep`
  plus Rust-aware checks for local policy.
- Do not require Nix or Docker for default CI unless a repo-specific product or
  environment contract justifies it.
- Do not run full-workspace mutation or full Miri on ordinary PRs.
- Do not introduce `cargo-hakari` or `sccache` until measured workspace scale or
  cache economics justify the extra surface.

## Contract summary

```text
ast-grep finds syntactic candidates.
cargo_metadata/guppy understand the workspace.
cargo-nextest runs the tests.
cargo-llvm-cov measures execution.
cargo-allow owns exception receipts.
ripr shifts mutation signal left.
unsafe-review makes unsafe reviewable.
cargo-mutants and Miri provide runtime backstops.
cargo-deny/vet/audit own dependency trust.
cargo-semver-checks owns release compatibility.
xtask ties it all into one repo-shaped control plane.
```
