#!/usr/bin/env bash
set -euo pipefail

cargo metadata --format-version 1 --no-deps
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --exclude shiplog-testkit -- --test-threads=4
cargo test -p shiplog --no-default-features -- --test-threads=4
cargo check --manifest-path fuzz/Cargo.toml --bins
cargo deny check
git diff --check

packages=(
  shiplog-ids
  shiplog-schema
  shiplog-ports
  shiplog-coverage
  shiplog-cache
  shiplog-redact
  shiplog-bundle
  shiplog-workstreams
  shiplog-merge
  shiplog-render-md
  shiplog-render-json
  shiplog-ingest-json
  shiplog-ingest-manual
  shiplog-ingest-git
  shiplog-ingest-github
  shiplog-cluster-llm
  shiplog-engine
  shiplog
)

for p in "${packages[@]}"; do
  echo "==> cargo package -p $p --list"
  cargo package -p "$p" --list
done
