#!/usr/bin/env bash
set -euo pipefail

tag="${1:-}"
if [[ -z "$tag" && "${GITHUB_REF:-}" == refs/tags/* ]]; then
  tag="${GITHUB_REF#refs/tags/}"
fi
if [[ -z "$tag" && -n "${GITHUB_REF_NAME:-}" ]]; then
  tag="$GITHUB_REF_NAME"
fi

if [[ -z "$tag" ]]; then
  echo "release hold guard requires an explicit release tag like v0.8.1" >&2
  exit 2
fi

if [[ ! "$tag" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "release hold guard requires a semver release tag like v0.8.1; got '$tag'" >&2
  exit 2
fi

hold_file="docs/release/0.9.0-release-hold.md"

case "$tag" in
  v0.9.0 | 0.9.0)
    if [[ -f "$hold_file" ]]; then
      echo "release hold blocks $tag while $hold_file exists" >&2
      echo "Do not tag, publish, or create a GitHub release for v0.9.0 until the hold is explicitly lifted." >&2
      exit 1
    fi
    ;;
esac

echo "release hold check passed for $tag"
