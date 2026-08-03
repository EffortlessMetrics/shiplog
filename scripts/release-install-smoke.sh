#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: scripts/release-install-smoke.sh <version>

Verifies the current-platform Shiplog release candidate, proves the no-token
first-use path, and runs the no-network review rescue smoke path. By default the
script downloads public GitHub Release assets. During staged release proof,
point it at the exact workflow candidate bundle instead.

Set SHIPLOG_RELEASE_CANDIDATE_DIR=path to use a local staged candidate bundle.
Set SHIPLOG_RELEASE_SOURCE_SHA=<full-sha> with candidate mode to bind the bundle
to the exact tagged source commit.
Set SHIPLOG_RELEASE_REPO=owner/repo to verify a public release or fork.
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
candidate_dir="${SHIPLOG_RELEASE_CANDIDATE_DIR:-}"
expected_source_sha="${SHIPLOG_RELEASE_SOURCE_SHA:-}"

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

lower_hex() {
  printf '%s' "$1" | tr '[:upper:]' '[:lower:]'
}

find_unique_candidate_file() {
  local root="$1"
  local name="$2"
  local match=""
  local candidate
  local count=0

  # Do not follow symlinked files/directories or cross filesystem boundaries.
  # NUL-delimited parsing keeps candidate identity unambiguous even when a
  # local scratch path contains whitespace or a newline.
  while IFS= read -r -d '' candidate; do
    count=$((count + 1))
    match="$candidate"
  done < <(find "$root" -xdev -type f -name "$name" -print0)

  if [[ "$count" != "1" ]]; then
    echo "candidate bundle must contain exactly one $name; found $count under $root" >&2
    exit 1
  fi
  printf '%s\n' "$match"
}

validate_candidate_manifest() {
  local root="$1"
  local manifest="$2"
  local sums="$3"
  local line key value required_asset
  local schema_version="" release_tag="" source_sha="" repository=""
  local workflow_run_id="" workflow_run_attempt="" asset_count=""
  local expected_sums_sha="" actual_sums_sha
  local seen_schema=0 seen_tag=0 seen_source=0 seen_repository=0
  local seen_run=0 seen_attempt=0 seen_count=0 seen_sums=0

  if [[ "$expected_source_sha" == "" ]]; then
    echo "SHIPLOG_RELEASE_SOURCE_SHA is required with SHIPLOG_RELEASE_CANDIDATE_DIR" >&2
    exit 2
  fi
  if [[ ! "$expected_source_sha" =~ ^[0-9a-fA-F]{40}$ ]]; then
    echo "SHIPLOG_RELEASE_SOURCE_SHA must be a full 40-character commit SHA" >&2
    exit 2
  fi

  # Parse one exact, closed schema. Duplicate, unknown, empty, or control-byte
  # fields are rejected before any manifest value reaches logs or commands.
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -n "$line" ]] || continue
    if [[ "$line" != *=* ]]; then
      echo "malformed candidate manifest field" >&2
      exit 1
    fi
    key="${line%%=*}"
    value="${line#*=}"
    if [[ -z "$key" || -z "$value" || "$key" =~ [[:cntrl:]] || "$value" =~ [[:cntrl:]] ]]; then
      echo "candidate manifest contains an empty or unsafe field" >&2
      exit 1
    fi

    case "$key" in
      schema_version)
        (( seen_schema == 0 )) || { echo "duplicate candidate manifest field: schema_version" >&2; exit 1; }
        seen_schema=1; schema_version="$value" ;;
      release_tag)
        (( seen_tag == 0 )) || { echo "duplicate candidate manifest field: release_tag" >&2; exit 1; }
        seen_tag=1; release_tag="$value" ;;
      source_sha)
        (( seen_source == 0 )) || { echo "duplicate candidate manifest field: source_sha" >&2; exit 1; }
        seen_source=1; source_sha="$value" ;;
      repository)
        (( seen_repository == 0 )) || { echo "duplicate candidate manifest field: repository" >&2; exit 1; }
        seen_repository=1; repository="$value" ;;
      workflow_run_id)
        (( seen_run == 0 )) || { echo "duplicate candidate manifest field: workflow_run_id" >&2; exit 1; }
        seen_run=1; workflow_run_id="$value" ;;
      workflow_run_attempt)
        (( seen_attempt == 0 )) || { echo "duplicate candidate manifest field: workflow_run_attempt" >&2; exit 1; }
        seen_attempt=1; workflow_run_attempt="$value" ;;
      asset_count)
        (( seen_count == 0 )) || { echo "duplicate candidate manifest field: asset_count" >&2; exit 1; }
        seen_count=1; asset_count="$value" ;;
      checksum_manifest_sha256)
        (( seen_sums == 0 )) || { echo "duplicate candidate manifest field: checksum_manifest_sha256" >&2; exit 1; }
        seen_sums=1; expected_sums_sha="$value" ;;
      *)
        echo "unknown candidate manifest field: $key" >&2
        exit 1
        ;;
    esac
  done < "$manifest"

  if (( seen_schema + seen_tag + seen_source + seen_repository + seen_run + seen_attempt + seen_count + seen_sums != 8 )); then
    echo "candidate manifest is missing required fields" >&2
    exit 1
  fi
  [[ "$schema_version" == "1" ]] || { echo "candidate manifest has unsupported schema" >&2; exit 1; }
  [[ "$release_tag" == "$tag" ]] || { echo "candidate manifest is not bound to $tag" >&2; exit 1; }
  [[ "$source_sha" == "$expected_source_sha" ]] || {
    echo "candidate manifest is not bound to source commit $expected_source_sha" >&2
    exit 1
  }
  [[ "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || {
    echo "candidate manifest has an invalid repository identity" >&2
    exit 1
  }
  [[ "$workflow_run_id" =~ ^[0-9]+$ && "$workflow_run_attempt" =~ ^[0-9]+$ ]] || {
    echo "candidate manifest has an invalid workflow run identity" >&2
    exit 1
  }
  [[ "$asset_count" == "4" ]] || {
    echo "candidate manifest does not record the four supported binaries" >&2
    exit 1
  }
  [[ "$expected_sums_sha" =~ ^[0-9a-fA-F]{64}$ ]] || {
    echo "candidate manifest has no valid checksum manifest digest" >&2
    exit 1
  }

  for required_asset in \
    shiplog-x86_64-unknown-linux-gnu \
    shiplog-x86_64-apple-darwin \
    shiplog-aarch64-apple-darwin \
    shiplog-x86_64-pc-windows-msvc.exe; do
    find_unique_candidate_file "$root" "$required_asset" >/dev/null
  done

  actual_sums_sha="$(sha256_file "$sums")"
  if [[ "$(lower_hex "$actual_sums_sha")" != "$(lower_hex "$expected_sums_sha")" ]]; then
    echo "candidate SHA256SUMS.txt digest mismatch" >&2
    echo "expected: $expected_sums_sha" >&2
    echo "actual:   $actual_sums_sha" >&2
    exit 1
  fi
}

asset="$(host_asset)"
binary_path="$download_dir/shiplog"
if [[ "$asset" == *.exe ]]; then
  binary_path="$download_dir/shiplog.exe"
fi

rm -rf "$work_dir"
mkdir -p "$download_dir"
work_dir="$(cd -P -- "$work_dir" && pwd)"
download_dir="$(cd -P -- "$download_dir" && pwd)"
demo_out="$work_dir/demo-out"
binary_path="$download_dir/$(basename -- "$binary_path")"

if [[ "$candidate_dir" != "" ]]; then
  candidate_dir="$(cd -P -- "$candidate_dir" && pwd)"
  candidate_asset="$(find_unique_candidate_file "$candidate_dir" "$asset")"
  candidate_sums="$(find_unique_candidate_file "$candidate_dir" SHA256SUMS.txt)"
  candidate_manifest="$(find_unique_candidate_file "$candidate_dir" RELEASE_CANDIDATE.txt)"

  echo "==> loading staged candidate $tag for this platform"
  cp "$candidate_asset" "$binary_path"
  cp "$candidate_sums" "$download_dir/SHA256SUMS.txt"
  cp "$candidate_manifest" "$download_dir/RELEASE_CANDIDATE.txt"
  validate_candidate_manifest \
    "$candidate_dir" \
    "$download_dir/RELEASE_CANDIDATE.txt" \
    "$download_dir/SHA256SUMS.txt"
else
  base_url="https://github.com/$repo/releases/download/$tag"
  echo "==> downloading $repo@$tag release asset for this platform"
  download "$base_url/$asset" "$binary_path"
  download "$base_url/SHA256SUMS.txt" "$download_dir/SHA256SUMS.txt"
fi

echo "==> verifying SHA256SUMS.txt entry for $asset"
expected_sha="$(
  awk -v asset="$asset" '
    {
      path = $2
      sub(/^.*\//, "", path)
      if (path == asset) {
        print $1
        found = 1
        exit
      }
    }
    END { if (!found) exit 1 }
  ' \
    "$download_dir/SHA256SUMS.txt"
)"
actual_sha="$(sha256_file "$binary_path")"
if [[ "$(lower_hex "$actual_sha")" != "$(lower_hex "$expected_sha")" ]]; then
  echo "checksum mismatch for $asset" >&2
  echo "expected: $expected_sha" >&2
  echo "actual:   $actual_sha" >&2
  exit 1
fi

chmod +x "$binary_path" 2>/dev/null || true

echo "==> smoking candidate binary"
if ! version_output="$("$binary_path" --version)"; then
  echo "candidate binary failed --version" >&2
  exit 1
fi
if [[ "$version_output" != "shiplog $version" ]]; then
  echo "unexpected version output: $version_output" >&2
  exit 1
fi
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

# The intake report records its path relative to the original cold-start
# working directory. Validate from that same directory so the self-reference is
# resolved once rather than joined onto the run directory a second time.
echo "==> structurally validating cold-start receipts"
latest_run_name="$(basename "$latest_run")"
(
  cd "$cold_start_dir"
  "$binary_path" report validate \
    --path "./out/$latest_run_name/intake.report.json" \
    --receipts
)

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
if [[ "$candidate_dir" != "" ]]; then
  echo "staged release candidate smoke passed for $tag at $expected_source_sha"
fi
