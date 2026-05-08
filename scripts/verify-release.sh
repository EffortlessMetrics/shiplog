#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: scripts/verify-release.sh <version>

Verifies a shipped release from public surfaces:
  - GitHub release exists and is not draft/prerelease
  - expected binary assets and SHA256SUMS.txt are present
  - downloaded assets verify against SHA256SUMS.txt
  - cargo install shiplog --version <version> --locked works
  - installed binary and current-platform release asset pass smoke commands

Set SHIPLOG_RELEASE_REPO=owner/repo to verify a fork.
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
download_dir="target/release-verify/$tag"
install_root="$download_dir/cargo-install"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 2
  fi
}

require_cmd cargo
require_cmd gh
require_cmd sha256sum

case "$download_dir" in
  target/release-verify/*) ;;
  *)
    echo "refusing unsafe download directory: $download_dir" >&2
    exit 2
    ;;
esac

echo "==> verifying GitHub release $repo@$tag"
release_tag="$(gh api "repos/$repo/releases/tags/$tag" --jq '.tag_name')"
is_draft="$(gh api "repos/$repo/releases/tags/$tag" --jq '.draft')"
is_prerelease="$(gh api "repos/$repo/releases/tags/$tag" --jq '.prerelease')"

if [[ "$release_tag" != "$tag" ]]; then
  echo "release tag mismatch: expected $tag, got $release_tag" >&2
  exit 1
fi
if [[ "$is_draft" != "false" ]]; then
  echo "$tag is still a draft release" >&2
  exit 1
fi
if [[ "$is_prerelease" != "false" ]]; then
  echo "$tag is marked prerelease" >&2
  exit 1
fi

expected_assets=(
  shiplog-x86_64-unknown-linux-gnu
  shiplog-x86_64-apple-darwin
  shiplog-aarch64-apple-darwin
  shiplog-x86_64-pc-windows-msvc.exe
  SHA256SUMS.txt
)

asset_names="$(gh api "repos/$repo/releases/tags/$tag" --jq '.assets[].name')"
for asset in "${expected_assets[@]}"; do
  if ! grep -Fxq "$asset" <<<"$asset_names"; then
    echo "missing release asset: $asset" >&2
    exit 1
  fi
done

echo "==> downloading release assets"
rm -rf "$download_dir"
mkdir -p "$download_dir"
gh release download "$tag" --repo "$repo" --dir "$download_dir"

asset_checksum_dir() {
  local asset="$1"
  if [[ "$asset" == *.exe ]]; then
    echo "${asset%.exe}"
  else
    echo "$asset"
  fi
}

asset_checksum_path() {
  local asset="$1"
  echo "$download_dir/$(asset_checksum_dir "$asset")/$asset"
}

echo "==> preparing checksum layout"
for asset in "${expected_assets[@]}"; do
  if [[ "$asset" == "SHA256SUMS.txt" ]]; then
    continue
  fi
  flat_asset="$download_dir/$asset"
  nested_asset="$(asset_checksum_path "$asset")"
  if [[ -f "$flat_asset" && ! -f "$nested_asset" ]]; then
    tmp_asset="$download_dir/.flat-$asset"
    mv "$flat_asset" "$tmp_asset"
    mkdir -p "$(dirname "$nested_asset")"
    mv "$tmp_asset" "$nested_asset"
  fi
done

echo "==> verifying SHA256SUMS.txt"
(
  cd "$download_dir"
  sha256sum -c SHA256SUMS.txt
)

release_asset_for_host() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os:$arch" in
    Linux:x86_64) echo "shiplog-x86_64-unknown-linux-gnu" ;;
    Darwin:x86_64) echo "shiplog-x86_64-apple-darwin" ;;
    Darwin:arm64) echo "shiplog-aarch64-apple-darwin" ;;
    MINGW*:x86_64|MSYS*:x86_64|CYGWIN*:x86_64) echo "shiplog-x86_64-pc-windows-msvc.exe" ;;
    *) echo "" ;;
  esac
}

smoke_binary() {
  local bin="$1"
  "$bin" --version | grep -Fxq "shiplog $version"
  "$bin" --help >/dev/null
  "$bin" init --dry-run >/dev/null
  "$bin" collect --help >/dev/null
  "$bin" collect multi --help >/dev/null
  "$bin" render --help >/dev/null
}

echo "==> installing shiplog $version from crates.io"
cargo install shiplog --version "$version" --locked --root "$install_root" --force

installed_bin="$install_root/bin/shiplog"
if [[ -x "$install_root/bin/shiplog.exe" ]]; then
  installed_bin="$install_root/bin/shiplog.exe"
fi
if [[ ! -x "$installed_bin" ]]; then
  echo "installed binary not found under $install_root/bin" >&2
  exit 1
fi

echo "==> smoking crates.io install"
smoke_binary "$installed_bin"

host_asset="$(release_asset_for_host)"
if [[ -n "$host_asset" ]]; then
  release_bin="$(asset_checksum_path "$host_asset")"
  chmod +x "$release_bin" 2>/dev/null || true
  echo "==> smoking downloaded $host_asset"
  smoke_binary "$release_bin"
else
  echo "==> no current-platform release asset smoke for $(uname -s)/$(uname -m); checksum verification already covered all assets"
fi

echo "release $tag verified"
