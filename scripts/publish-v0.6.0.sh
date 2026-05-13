#!/usr/bin/env bash
set -euo pipefail

# Publish shiplog v0.6.0 to crates.io in dependency order.
# Usage: CARGO_REGISTRY_TOKEN=<token> bash scripts/publish-v0.6.0.sh

if [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
    echo "error: CARGO_REGISTRY_TOKEN not set"
    echo "usage: CARGO_REGISTRY_TOKEN=<token> bash $0"
    exit 1
fi

crates=(
    shiplog-ids
    shiplog-schema
    shiplog-ports
    shiplog-merge
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
    shiplog-engine
    shiplog-team
    shiplog
)

for crate in "${crates[@]}"; do
    echo "==> Publishing $crate..."
    cargo publish -p "$crate"
    echo "ok: $crate published"
    sleep 2
done

echo ""
echo "ok: all 0.6.0 crates published"
echo ""
echo "Next steps:"
echo "1. Verify releases at https://crates.io/crates/shiplog/versions"
echo "2. Push tag v0.6.0 if it was not pushed before publishing"
echo "3. Run release-install smoke tests against v0.6.0 artifacts"
