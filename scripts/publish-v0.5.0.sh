#!/usr/bin/env bash
set -euo pipefail

# Publish shiplog v0.5.0 to crates.io in dependency order
# Usage: CARGO_REGISTRY_TOKEN=<token> ./scripts/publish-v0.5.0.sh

if [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
    echo "error: CARGO_REGISTRY_TOKEN not set"
    echo "usage: CARGO_REGISTRY_TOKEN=<token> $0"
    exit 1
fi

crates=(
    shiplog-ids
    shiplog-schema
    shiplog-ports
    shiplog-coverage
    shiplog-cache
    shiplog-redact
    shiplog-bundle
    shiplog-workstreams
    shiplog-render-md
    shiplog-render-json
    shiplog-cluster-llm
    shiplog-ingest-github
    shiplog-ingest-git
    shiplog-ingest-json
    shiplog-ingest-manual
    shiplog-ingest-gitlab
    shiplog-ingest-jira
    shiplog-ingest-linear
    shiplog-team
    shiplog-merge
    shiplog-engine
    shiplog
)

for crate in "${crates[@]}"; do
    echo "==> Publishing $crate..."
    cargo publish -p "$crate"
    echo "✅ $crate published"
    sleep 2  # rate-limit API calls
done

echo ""
echo "✅ All 23 crates published successfully"
echo ""
echo "Next steps:"
echo "1. Verify releases at https://crates.io/crates/shiplog/versions"
echo "2. Announce the release on GitHub"
echo "3. Update any dependent projects"
