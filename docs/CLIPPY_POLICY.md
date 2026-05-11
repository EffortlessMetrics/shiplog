# Clippy Policy

This document describes how shiplog manages Clippy lints. It exists so a
reader can answer: which lints are active, which are planned, why are some
allowed, and how do we change that?

It is part of the [Rust 1.95 / 0.5.0 quality
rollout](ci/rust-1.95-rollout.md). Today shiplog has only a small workspace
lints block; the TOML skeletons land in PR #141, the `xtask` runner in
PR #143, the `clippy.toml` in PR #145 (alongside the MSRV bump), and the
ledger checker in PR #150. Everything below describes the intended steady
state once those PRs have merged.

## Intent

- Clippy enforces our taste, but only after we have written down the rule and
  the reason. Adding a lint without a ledger entry is not allowed.
- A lint is either **active** (enforced) or **planned** (recorded in the
  ledger and gated on a future MSRV / future PR). It is never silently dropped.
- Suppressions are reviewable. We do not allow bare `#[allow(clippy::...)]`
  without a citation.

## Files

```text
clippy.toml                    Clippy MSRV + thresholds
policy/clippy-lints.toml       Active and planned lints, with reasons
policy/clippy-debt.toml        Broad allows that exist as receipted debt
policy/clippy-exceptions.toml  Per-occurrence suppressions with owner+reason+expiry
```

The `xtask` runner (PR #143) provides:

```bash
cargo xtask check-lint-policy        # workspace lints match policy
cargo xtask check-clippy-exceptions  # exceptions ledger is well-formed
cargo xtask policy-report            # human summary across all policy ledgers
```

`check-lint-policy` verifies:

- `workspace.package.rust-version` matches `policy.msrv`.
- `clippy.toml`'s `msrv` matches `policy.msrv`.
- No test carveouts (e.g. `#[cfg(test)]`-scoped allows that hide debt).
- Every `[[active]]` lint in the policy is reflected in
  `[workspace.lints]` of the root `Cargo.toml`.
- Every `[[planned]]` lint is either activated or has an explicit
  `activate_when_msrv` deferral.
- Every broad allow currently in `[workspace.lints]` is receipted in
  `policy/clippy-debt.toml`.
- Every exception entry has an owner, a reason, and an expiry.

## Suppression style

Use `#[expect(..., reason = "...")]`, not `#[allow(...)]`. `expect` fails the
build when the lint stops firing, so dead suppressions are caught
automatically.

```rust
// Good — reason cited, automatically caught when no longer needed.
#[expect(clippy::needless_pass_by_value, reason = "kept until shiplog-engine cleanup; tracked in policy/clippy-debt.toml")]
fn render_packet(events: Vec<Event>) -> Packet { /* ... */ }

// Not allowed.
#[allow(clippy::needless_pass_by_value)]
fn render_packet(events: Vec<Event>) -> Packet { /* ... */ }
```

Bare `#[allow(clippy::...)]` is rejected by `check-clippy-exceptions`.

## No test carveouts

Tests must obey the same lint policy as production code. Patterns like

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests { /* ... */ }
```

are not allowed. If a lint produces noise in tests, the fix is one of:

- write a fallible test helper (e.g. `fn fixture() -> Result<Fixture>`),
- demote the lint to `warn` workspace-wide and document the demotion in
  `policy/clippy-debt.toml`,
- or scope a single `#[expect(..., reason = "...")]` per call site with a
  citation.

Test code paths inside `#[cfg(test)]` modules and `tests/` directories all
count.

## Lint MSRV gating

Many of the lints we want to add are introduced in newer compilers. The ledger
encodes this with `activate_when_msrv = "..."`:

```toml
[[planned]]
name = "clippy::manual_checked_ops"
level = "warn"
activate_when_msrv = "1.95"
reason = "Prefer checked arithmetic over manual divide-by-zero guards."
```

When `policy.msrv` reaches the gate, the lint moves from `[[planned]]` to
`[[active]]` in a dedicated PR (PR #152, the Rust+Clippy 1.95 ratchets PR).
Bumping MSRV without activating gated lints is allowed, but
`check-lint-policy` will report the lints as ready-to-activate.

## Current debt (pre-ledger)

Until PR #150 lands the ledger checker, the following workspace allows exist
in `Cargo.toml`:

```toml
[workspace.lints.clippy]
needless_pass_by_value = "allow"   # ~30+ hits across crates
cloned_instead_of_copied = "allow" # ~40+ hits across crates
```

These are intentional debt and will become entries in
`policy/clippy-debt.toml` with owner, reason, and expiry once the ledger
lands (skeletons in PR #141, checker in PR #150). Burndown is opportunistic —
a small dent happens when 1.95 ratchets activate in PR #152, and the broader
sweep is a follow-up release.

## See also

- [`POLICY_ALLOWLISTS.md`](POLICY_ALLOWLISTS.md) — common schema for ledger
  entries (owner / reason / expiry).
- [`NO_PANIC_POLICY.md`](NO_PANIC_POLICY.md) — companion policy for panic-family
  shapes.
- [`FILE_POLICY.md`](FILE_POLICY.md) — companion policy for non-Rust files.
- [`CLIPPY_PROTECTED_FIELDS.md`](CLIPPY_PROTECTED_FIELDS.md) — the six
  protected field classes that `clippy::disallowed_fields` will eventually
  enforce, and the staged activation ladder.
- [`ci/rust-1.95-rollout.md`](ci/rust-1.95-rollout.md) — the rollout map and
  PR ladder that introduces this policy.
