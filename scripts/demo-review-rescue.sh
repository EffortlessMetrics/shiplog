#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: scripts/demo-review-rescue.sh [--out path] [--shiplog-bin path] [--config path]

Runs the no-network review rescue demo against checked-in local fixtures.
The demo exercises intake, intake-report reopening, commands-only fixups, and
manager share verification without requiring provider tokens.

Defaults:
  --out ./out/demo-review-rescue
  --shiplog-bin shiplog
  --config examples/configs/local-git-json-manual.toml
USAGE
}

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"
out="./out/demo-review-rescue"
shiplog_bin="shiplog"
config="examples/configs/local-git-json-manual.toml"

absolute_path() {
  local path="$1"
  case "$path" in
    /*) echo "$path" ;;
    [A-Za-z]:*) echo "$path" ;;
    *) echo "$(pwd)/$path" ;;
  esac
}

while [[ "${1:-}" != "" ]]; do
  case "$1" in
    --out)
      out="${2:-}"
      shift 2
      ;;
    --shiplog-bin)
      shiplog_bin="${2:-}"
      shift 2
      ;;
    --config)
      config="${2:-}"
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$out" || -z "$shiplog_bin" || -z "$config" ]]; then
  usage
  exit 2
fi

out="$(absolute_path "$out")"
case "$shiplog_bin" in
  */* | *\\*) shiplog_bin="$(absolute_path "$shiplog_bin")" ;;
esac
if [[ "$config" == /* || "$config" == [A-Za-z]:* ]]; then
  config_arg="$config"
else
  config_arg="$repo_root/$config"
fi

without_provider_tokens() {
  env \
    -u GITHUB_TOKEN \
    -u GITLAB_TOKEN \
    -u JIRA_TOKEN \
    -u LINEAR_API_KEY \
    "$@"
}

echo "==> running review rescue demo"
echo "out: $out"
echo "config: $config_arg"

mkdir -p "$out"
(
  cd "$repo_root"
  without_provider_tokens "$shiplog_bin" \
    intake \
    --out "$out" \
    --config "$config_arg" \
    --no-open \
    --explain

  echo
  echo "==> intake report"
  without_provider_tokens "$shiplog_bin" open intake-report --out "$out" --latest --print-path

  echo
  echo "==> commands-only fixups"
  without_provider_tokens "$shiplog_bin" review fixups --out "$out" --latest --commands-only

  echo
  echo "==> manager share preflight"
  without_provider_tokens "$shiplog_bin" share verify manager --out "$out" --latest --redact-key fixture-key
)

if ! find "$out" -name intake.report.md -type f -print -quit | grep -q .; then
  echo "no intake.report.md produced under $out" >&2
  exit 1
fi
if ! find "$out" -name packet.md -type f -print -quit | grep -q .; then
  echo "no packet.md produced under $out" >&2
  exit 1
fi

echo
echo "review rescue demo passed"
