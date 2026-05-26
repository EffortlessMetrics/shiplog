#!/usr/bin/env bash
set -euo pipefail

from=""
if [[ "${1:-}" == "--from" ]]; then
  from="${2:-}"
  if [[ -z "$from" ]]; then
    echo "usage: $0 [--from <package>]" >&2
    exit 2
  fi
elif [[ "${1:-}" != "" ]]; then
  echo "usage: $0 [--from <package>]" >&2
  exit 2
fi

scripts/package-version-audit.sh
scripts/package-boundary-audit.sh

python_bin="${PYTHON:-}"
if [[ -z "$python_bin" ]]; then
  if command -v python3 >/dev/null 2>&1; then
    python_bin=python3
  elif command -v python >/dev/null 2>&1; then
    python_bin=python
  else
    echo "python3 or python is required for publish dry-run" >&2
    exit 127
  fi
fi

mapfile -t packages < <("$python_bin" - <<'PY' | tr -d '\r'
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

seen_from=false
for p in "${packages[@]}"; do
  if [[ -n "$from" && "$seen_from" == false ]]; then
    if [[ "$p" == "$from" ]]; then
      seen_from=true
    else
      continue
    fi
  fi

  echo "==> cargo publish -p $p --dry-run --locked"
  cargo publish -p "$p" --dry-run --locked
done

if [[ -n "$from" && "$seen_from" == false ]]; then
  echo "package not found in release matrix: $from" >&2
  exit 2
fi
