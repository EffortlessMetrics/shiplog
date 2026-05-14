#!/usr/bin/env bash
set -euo pipefail

mkdir -p target
cargo metadata --format-version 1 --no-deps > target/package-proof-metadata.json
scripts/package-boundary-audit.sh
scripts/package-version-audit.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --exclude shiplog-testkit -- --test-threads=4
cargo test -p shiplog --no-default-features -- --test-threads=4
cargo check --manifest-path fuzz/Cargo.toml --bins
cargo deny check
git diff --check

mapfile -t packages < <(python - <<'PY' | tr -d '\r'
from pathlib import Path
import sys

try:
    import tomllib
except ModuleNotFoundError:
    print("Python 3.11+ is required for tomllib", file=sys.stderr)
    sys.exit(2)

policy = tomllib.loads(Path("policy/publish-allowlist.toml").read_text())
for package in policy.get("publish", {}).get("default_order", []):
    print(package)
PY
)

if [[ "${#packages[@]}" -eq 0 ]]; then
  echo "publish allowlist is empty" >&2
  exit 1
fi

for p in "${packages[@]}"; do
  echo "==> cargo package -p $p --list"
  cargo package -p "$p" --list
done
