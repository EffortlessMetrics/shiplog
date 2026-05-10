# No-Panic Policy

This document describes how shiplog tracks panic-family shapes (`unwrap`,
`expect`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`, indexing slices,
`assert!` outside tests, and similar) so they cannot grow silently.

It is part of the [Rust 1.95 / 0.5.0 quality
rollout](ci/rust-1.95-rollout.md). The `policy/no-panic-baseline.toml` and
`policy/no-panic-allowlist.toml` skeletons land in PR #141; the classifier
and gate land in PR #151. Everything below describes the intended steady
state.

## Intent

shiplog's product loop produces evidence other people rely on (intake reports,
share manifests, redaction-key gated bundles). A panic in production is an
evidence-loss event. We want:

- New panic-family shapes never enter the codebase silently.
- Existing shapes are inventoried, not pretended to be absent.
- The inventory matches by **exact counted finding identity**, so an allowlist
  entry cannot accidentally cover an unrelated `unwrap` later added in the same
  file.

## Mode

```toml
# policy/no-panic-baseline.toml
[policy]
mode = "no-new-debt"
```

`no-new-debt` means: the baseline records the panic-family findings that exist
today; PRs cannot introduce new findings; PRs can remove findings, which
shrinks the baseline; resetting the baseline requires a dedicated PR (PR #151
for the initial baseline; future resets are themselves single-purpose PRs).

This is not "all panics are forbidden". It is "today's panics are receipted
and cannot grow without an explicit decision."

## Exact-identity matching

A baseline entry captures the full identity of the finding, not a coarse
shape:

```toml
[[entry]]
path = "crates/shiplog-engine/src/intake.rs"
family = "unwrap"
selector_kind = "method-call"
selector_callee = "Option::unwrap"
snippet = "config.report_dir.unwrap()"
count = 1
```

The combination `(path, family, selector_kind, selector_callee, snippet,
count)` is the entry's identity. If a new `config.report_dir.unwrap()` is added
in the same file, the count goes from 1 to 2, the baseline check fails, and the
PR is responsible for either:

- removing the new occurrence,
- bumping the entry to `count = 2` in a dedicated baseline-update PR with a
  recorded reason, or
- moving the addition to `policy/no-panic-allowlist.toml` with owner / reason
  / expiry (rare, requires reviewer sign-off).

A coarse `(path, family)` baseline would let an unrelated `unwrap` in the same
file slip in unnoticed. We explicitly do not do that.

## Allowlist vs baseline

| | `policy/no-panic-baseline.toml` | `policy/no-panic-allowlist.toml` |
| --- | --- | --- |
| Purpose | Snapshot of existing debt | Permanent, justified exceptions |
| Entry shape | exact identity + count | exact identity + owner + reason + expiry |
| Growth | Shrinks over time as debt is paid down | Grows only with reviewer approval |
| Review weight | low (snapshot) | high (codified exception) |

Most existing findings belong in the baseline. The allowlist is reserved for
panics we have decided are correct (e.g. an `unreachable!` in an exhaustive
match that the type system cannot prove, with a clear comment).

## Production vs test scope

The policy applies to production code paths:

- `apps/shiplog/src/**/*.rs`
- `crates/shiplog-*/src/**/*.rs`
- `xtask/src/**/*.rs`
- `crates/shiplog-*/build.rs`

Test code paths are out of scope by default:

- `crates/shiplog-*/tests/**/*.rs`
- `crates/shiplog-*/src/**/*.rs` inside `#[cfg(test)]` modules
- `crates/shiplog-testkit/**/*.rs`
- `fuzz/**/*.rs`

This is because tests are allowed to assert; the no-panic policy is about not
losing evidence in production. However, a panic that is theoretically
test-only but reachable from a `pub fn` should still be receipted — the
classifier flags those.

## Files

```text
policy/no-panic-baseline.toml      Exact-identity inventory of existing shapes
policy/no-panic-allowlist.toml     Permanent justified exceptions
.gitattributes                     Marks baseline as generated, eol=lf
docs/NO_PANIC_POLICY.md            This file
xtask/src/tasks/no_panic.rs        Classifier and baseline updater
```

The baseline is marked `linguist-generated=true` so review tools collapse it
by default.

## Commands

```bash
cargo xtask no-panic baseline           # show diff vs baseline
cargo xtask no-panic baseline --reset   # rewrite baseline (dedicated PR only)
cargo xtask check-no-panic-family       # CI gate
cargo xtask policy-report               # summary across all policy ledgers
```

`check-no-panic-family` is the gate that fails CI when a PR adds a new
finding without a corresponding baseline or allowlist entry.

## When to reset the baseline

The baseline is reset only in a dedicated PR. Trigger conditions:

- A burndown effort removed enough findings that the file should be rewritten
  so reviewers can see the smaller surface, not a long list of removed
  entries. (Burndown in v0.5.0 is opportunistic: small dents during 1.95
  ratchets in PR #152, broader sweeps as follow-up.)
- A refactor changed file paths, breaking entry identity. Always prefer to
  preserve identities by rebasing the path field; reset only when the change
  is structural (crate split, large rename).

A reset PR contains only the baseline change and an explanation. It must not
also change product code.

## See also

- [`CLIPPY_POLICY.md`](CLIPPY_POLICY.md) — companion policy for Clippy lints.
- [`POLICY_ALLOWLISTS.md`](POLICY_ALLOWLISTS.md) — common schema for owner /
  reason / expiry entries.
- [`ci/rust-1.95-rollout.md`](ci/rust-1.95-rollout.md) — the rollout map and
  PR ladder.
