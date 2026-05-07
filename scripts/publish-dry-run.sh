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
  shiplog-ingest-gitlab
  shiplog-ingest-jira
  shiplog-ingest-linear
  shiplog-cluster-llm
  shiplog-team
  shiplog-engine
  shiplog
)

seen_from=false
for p in "${packages[@]}"; do
  if [[ -n "$from" && "$seen_from" == false ]]; then
    if [[ "$p" == "$from" ]]; then
      seen_from=true
    else
      continue
    fi
  fi

  echo "==> cargo publish -p $p --dry-run"
  cargo publish -p "$p" --dry-run
done

if [[ -n "$from" && "$seen_from" == false ]]; then
  echo "package not found in release matrix: $from" >&2
  exit 2
fi
