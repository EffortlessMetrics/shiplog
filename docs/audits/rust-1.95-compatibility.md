# Rust 1.95 Compatibility Probe

Audit of shiplog `main` (post-PR #143, commit `a11ee72`) running under
Rust 1.95.0. Performed as PR #144 of the v0.5.0 ladder; no MSRV /
toolchain change in this PR. The actual bump to `rust-version = "1.95"`
+ `rust-toolchain.toml` channel `1.95.0` lands in PR #145, which must
address the fallout below.

## Probe environment

| | |
|---|---|
| Date | 2026-05-09 |
| Probe machine | Windows 11 Pro (x86_64-pc-windows-msvc) |
| Toolchain probed | `rustc 1.95.0` (installed via rustup) |
| Active workspace pin | `rustc 1.92.0` (unchanged) |
| Probe method | `cargo +1.95.0 <subcmd>` per command (no `rustup override`, no `rust-toolchain.toml` change) |

## Gates run

| Gate | Result | Notes |
|------|--------|-------|
| `cargo +1.95.0 fmt --all -- --check` | ✅ pass | No formatting drift |
| `cargo +1.95.0 check --workspace --all-targets --all-features --locked` | ✅ pass | All 23 workspace crates compile |
| `cargo +1.95.0 clippy --workspace --all-targets --all-features --locked` | ⚠️ 1 lint, 2 sites | See [Findings](#findings) |
| `cargo +1.95.0 clippy --workspace --all-targets --all-features --locked -- -D warnings` | ❌ fails | Blocked by the same lint above |
| `cargo +1.95.0 test --workspace --all-features --locked` | ✅ pass | All tests + doctests pass |
| `cargo +1.95.0 doc --workspace --no-deps` | ✅ pass | Generated cleanly |
| `bash scripts/package-boundary-audit.sh` | ✅ pass | 22 published + 2 dev-only (toolchain-independent) |
| `bash scripts/package-version-audit.sh` | ✅ pass | 24 packages at 0.4.0 (toolchain-independent) |

## Findings

### `clippy::unnecessary_sort_by` (×2 sites)

A new Clippy lint added in Rust 1.95 ([reference](https://rust-lang.github.io/rust-clippy/rust-1.95.0/index.html#unnecessary_sort_by)).
It flags `Vec::sort_by(|a, b| b.X.cmp(&a.X))` and suggests
`Vec::sort_by_key(|b| std::cmp::Reverse(b.X))`. The lint is `warn`
by default; under shiplog's `-D warnings` policy it becomes an error.

**Sites:**

1. `crates/shiplog-ingest-git/src/lib.rs:267`

   ```rust
   events.sort_by(|a, b| b.occurred_at.cmp(&a.occurred_at));
   ```

   Fix:

   ```rust
   events.sort_by_key(|b| std::cmp::Reverse(b.occurred_at));
   ```

2. `apps/shiplog/src/main.rs:7608`

   ```rust
   candidates.sort_by(|left, right| right.0.cmp(&left.0));
   ```

   Fix:

   ```rust
   candidates.sort_by_key(|right| std::cmp::Reverse(right.0));
   ```

**Impact:** Both sites compile and run correctly on 1.95. The fallout is
purely Clippy strictness, not a runtime or type-system change. Tests
continue to pass.

**Recommended action in PR #145** (the mechanical MSRV bump):

- Apply the two `sort_by_key(|b| std::cmp::Reverse(b.X))` fixes inline.
  This counts as 1.95 compatibility fallout (without it, the toolchain
  bump itself would break CI's `cargo clippy -- -D warnings` gate). It
  is **not** "modernization" or "policy activation" — it is the minimal
  diff required to keep CI green after the toolchain pin moves.
- Alternatively, add an `[[allow]]` entry in `[workspace.lints.clippy]`
  for `unnecessary_sort_by` and a matching debt entry in
  `policy/clippy-debt.toml`. The fix is mechanical enough that the
  inline fix is preferred over taking on debt.

The lint should also be added to `policy/clippy-lints.toml` as
`[[planned]]` with `activate_when_msrv = "1.95"` so the ledger reflects
that the lint becomes active at the bumped MSRV (PR #150 ledger
checker enforces this).

## What was NOT a finding

- **`rustfmt 1.95` formatting**: identical to 1.92 output for the
  current shiplog tree.
- **`cargo check`**: clean across all 23 crates including all features
  (LLM gate enabled).
- **`cargo test`**: every workspace test plus every doctest passes.
- **`cargo doc --no-deps`**: builds without warnings.
- **Existing per-crate Clippy debt** (`needless_pass_by_value`,
  `cloned_instead_of_copied`): unchanged behavior under 1.95.
- **MSRV / Cargo features**: no syntax or std-API issue surfaced by
  `cargo check --all-features`.

## What this probe does NOT cover

The probe was run on Windows only. Linux + macOS parity should be
verified in CI on the PR #145 branch. The expectation:

- Linux Ubuntu 24.04: identical clippy fallout (it's a lint-table
  difference, not platform-specific).
- macOS: same expectation; `lib.rs` and `apps/shiplog` build cleanly on
  1.95 across platforms today (1.92 already verified there).

The probe did not test Rust 1.95 against:

- Sanitizer builds (none configured for shiplog).
- Cross-compilation targets (release pipeline uses `--target` per
  platform; build-binary lane on 1.95 should be smoke-tested as part
  of the PR #145 release-readiness check).

## Conclusion

shiplog is **compatible with Rust 1.95**. The only blocking item for the
MSRV bump (PR #145) is the two `sort_by_key`-style fixes for the new
`clippy::unnecessary_sort_by` lint. That is mechanical and non-behavior
changing. No semver-significant API surface change, no test regression,
no doc regression.

PR #145 acceptance:

- [ ] Bump `Cargo.toml` `rust-version = "1.95"` and `rust-toolchain.toml`
      `channel = "1.95.0"`.
- [ ] Bump `.github/workflows/ci.yml` and `.github/workflows/release.yml`
      toolchain pins from `1.92` → `1.95.0` (4 + 5 sites).
- [ ] Rename `MSRV (1.92)` job to `MSRV (1.95)` (single string change in
      `ci.yml`); update branch protection in the same merge per
      [`required-check-migration.md`](../ci/required-check-migration.md).
- [ ] Add `clippy.toml` with `msrv = "1.95"` and the
      `cognitive-complexity-threshold` / `too-many-arguments-threshold` /
      `type-complexity-threshold` numbers per
      [`docs/CLIPPY_POLICY.md`](../CLIPPY_POLICY.md).
- [ ] Apply the two `sort_by_key(|b| std::cmp::Reverse(b.X))` fixes
      flagged above (1.95 compatibility fallout).
- [ ] Add `clippy::unnecessary_sort_by` to `policy/clippy-lints.toml`
      `[[planned]]` with `activate_when_msrv = "1.95"` (so the lint
      ledger records that it becomes active at the bumped MSRV).
- [ ] Sweep `README.md`, `CLAUDE.md`, and any docs that reference
      `1.92` for stale MSRV strings (per
      [`required-check-migration.md`](../ci/required-check-migration.md)).
- [ ] Verify all workspace gates green on Ubuntu + Windows.

No API cleanup. No policy activation beyond what is listed above. The
1.95 lint floor and ratchets land in PR #152 (separate slice).
