#!/usr/bin/env bash
# Run the same fmt/clippy/test gate CI enforces, locally, in one command.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

echo "==> cargo fmt --all -- --check"
cargo fmt --all -- --check

echo "==> cargo clippy --workspace --all-targets --all-features -- -D warnings"
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "==> cargo test --workspace"
cargo test --workspace

echo "All checks passed."
