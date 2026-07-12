#!/usr/bin/env bash
set -euo pipefail

repo="${SHIPLOG_RELEASE_REPO:-EffortlessMetrics/shiplog}"
install_dir="${SHIPLOG_INSTALL_DIR:-$HOME/.local/bin}"

usage() {
  cat <<'USAGE'
Install the latest prebuilt shiplog binary.

Environment:
  SHIPLOG_RELEASE_REPO  GitHub owner/repo override
  SHIPLOG_INSTALL_DIR   User-local installation directory override
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi
if [[ "${1:-}" != "" ]]; then
  usage >&2
  exit 2
fi

download() {
  local url="$1"
  local destination="$2"
  if command -v curl >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -fsSL -o "$destination" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$destination" "$url"
  else
    echo "missing required command: curl or wget" >&2
    exit 2
  fi
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "missing required command: sha256sum or shasum" >&2
    exit 2
  fi
}

case "$(uname -s):$(uname -m)" in
  Linux:x86_64) asset="shiplog-x86_64-unknown-linux-gnu" ;;
  Darwin:x86_64) asset="shiplog-x86_64-apple-darwin" ;;
  Darwin:arm64|Darwin:aarch64) asset="shiplog-aarch64-apple-darwin" ;;
  *)
    echo "unsupported shiplog release platform: $(uname -s)/$(uname -m)" >&2
    exit 2
    ;;
esac

base_url="https://github.com/$repo/releases/latest/download"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/shiplog-install.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

echo "==> downloading latest shiplog for $asset"
download "$base_url/$asset" "$work_dir/$asset"
download "$base_url/SHA256SUMS.txt" "$work_dir/SHA256SUMS.txt"

expected_sha="$(awk -v asset="$asset" '$2 == asset || $2 ~ "/" asset "$" {print $1; found=1} END {if (!found) exit 1}' "$work_dir/SHA256SUMS.txt")"
actual_sha="$(sha256_file "$work_dir/$asset")"
if [[ "$actual_sha" != "$expected_sha" ]]; then
  echo "checksum mismatch for $asset" >&2
  echo "expected: $expected_sha" >&2
  echo "actual:   $actual_sha" >&2
  exit 1
fi

mkdir -p "$install_dir"
install -m 0755 "$work_dir/$asset" "$install_dir/shiplog"

echo "Installed shiplog to $install_dir/shiplog"
if [[ ":${PATH}:" != *":$install_dir:"* ]]; then
  echo "Add this directory to PATH: $install_dir"
fi
"$install_dir/shiplog" --version
