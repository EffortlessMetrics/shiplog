#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: scripts/release-install-smoke.sh <version>

Downloads the current-platform GitHub release binary, verifies SHA256SUMS.txt,
proves the no-token first-use path and runs the no-network review rescue smoke
path. This script is intended to work without Rust or Cargo installed.

Set SHIPLOG_RELEASE_REPO=owner/repo to verify a fork.
Set SHIPLOG_RELEASE_SMOKE_DIR=path to override the scratch directory.
USAGE
}

if [[ "${1:-}" == "" || "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 2
fi
if [[ "${2:-}" != "" ]]; then
  usage
  exit 2
fi

version="${1#v}"
tag="v$version"
repo="${SHIPLOG_RELEASE_REPO:-EffortlessMetrics/shiplog}"

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"
work_dir="${SHIPLOG_RELEASE_SMOKE_DIR:-$repo_root/target/release-install-smoke/$tag}"
download_dir="$work_dir/download"
demo_out="$work_dir/demo-out"

download() {
  local url="$1"
  local out="$2"

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$out" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$out" "$url"
  else
    echo "missing required command: curl or wget" >&2
    exit 2
  fi
}

host_asset() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os:$arch" in
    Linux:x86_64) echo "shiplog-x86_64-unknown-linux-gnu" ;;
    Darwin:x86_64) echo "shiplog-x86_64-apple-darwin" ;;
    Darwin:arm64 | Darwin:aarch64) echo "shiplog-aarch64-apple-darwin" ;;
    MINGW*:x86_64 | MSYS*:x86_64 | CYGWIN*:x86_64) echo "shiplog-x86_64-pc-windows-msvc.exe" ;;
    *)
      echo "unsupported release smoke platform: $os/$arch" >&2
      exit 2
      ;;
  esac
}

sha256_file() {
  local path="$1"

  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{ print $1 }'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path" | awk '{ print $1 }'
  else
    echo "missing required command: sha256sum or shasum" >&2
    exit 2
  fi
}

asset="$(host_asset)"
base_url="https://github.com/$repo/releases/download/$tag"
binary_path="$download_dir/shiplog"
if [[ "$asset" == *.exe ]]; then
  binary_path="$download_dir/shiplog.exe"
fi

echo "==> downloading $repo@$tag release asset for this platform"
rm -rf "$work_dir"
mkdir -p "$download_dir"
download "$base_url/$asset" "$binary_path"
download "$base_url/SHA256SUMS.txt" "$download_dir/SHA256SUMS.txt"

echo "==> verifying SHA256SUMS.txt entry for $asset"
expected_sha="$(
  awk -v asset="$asset" '$2 ~ "/" asset "$" { print $1; found=1 } END { if (!found) exit 1 }' \
    "$download_dir/SHA256SUMS.txt"
)"
actual_sha="$(sha256_file "$binary_path")"
if [[ "$actual_sha" != "$expected_sha" ]]; then
  echo "checksum mismatch for $asset" >&2
  echo "expected: $expected_sha" >&2
  echo "actual:   $actual_sha" >&2
  exit 1
fi

chmod +x "$binary_path" 2>/dev/null || true

echo "==> smoking downloaded binary"
"$binary_path" --version | grep -Fxq "shiplog $version"
"$binary_path" --help >/dev/null

echo "==> proving the no-token first-use path"
cold_start_dir="$work_dir/cold-start"
rm -rf "$cold_start_dir"
mkdir -p "$cold_start_dir/gh-config"
unset GITHUB_TOKEN GH_TOKEN GITLAB_TOKEN JIRA_TOKEN LINEAR_API_KEY SHIPLOG_REDACT_KEY || true
export GH_CONFIG_DIR="$cold_start_dir/gh-config"

(
  cd "$cold_start_dir"
  "$binary_path" >/dev/null
  "$binary_path" intake >/dev/null
  open_path="$("$binary_path" open --print-path)"
  [[ -n "$open_path" && -f "$open_path" ]]
  "$binary_path" status --latest --json > "$cold_start_dir/status.latest.json"
  [[ -s "$cold_start_dir/status.latest.json" ]]
  event_date="$(date -u +%F)"
  "$binary_path" add "Published binary cold-start proof" \
    --date "$event_date" \
    --description "Verified the release binary from an empty directory without provider credentials." \
    >/dev/null
  "$binary_path" update --no-open >/dev/null
)

latest_run="$(find "$cold_start_dir/out" -mindepth 1 -maxdepth 1 -type d -name 'merge_*' -print | sort | tail -n 1)"
if [[ "$latest_run" == "" ]]; then
  echo "no cold-start run directory produced under $cold_start_dir/out" >&2
  exit 1
fi
for artifact in \
  packet.md \
  intake.report.json \
  ledger.events.jsonl \
  coverage.manifest.json \
  bundle.manifest.json; do
  if [[ ! -f "$latest_run/$artifact" ]]; then
    echo "missing cold-start artifact: $latest_run/$artifact" >&2
    exit 1
  fi
done

echo "==> running no-network review rescue fixture"
rm -rf "$demo_out"
"$script_dir/demo-review-rescue.sh" \
  --shiplog-bin "$binary_path" \
  --out "$demo_out" \
  >"$work_dir/demo-review-rescue.stdout"

if ! find "$demo_out" -name intake.report.md -type f -print -quit | grep -q .; then
  echo "no intake.report.md produced under $demo_out" >&2
  exit 1
fi
if ! find "$demo_out" -name packet.md -type f -print -quit | grep -q .; then
  echo "no packet.md produced under $demo_out" >&2
  exit 1
fi

echo "release install smoke passed for $repo@$tag"
