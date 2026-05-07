#!/usr/bin/env bash
set -euo pipefail

mkdir -p target
cargo metadata --format-version 1 --no-deps > target/package-boundary-metadata.json

python - <<'PY'
import json
import sys
from pathlib import Path

metadata_path = Path("target/package-boundary-metadata.json")
metadata = json.loads(metadata_path.read_text())

published = {
    "shiplog",
    "shiplog-ids",
    "shiplog-schema",
    "shiplog-ports",
    "shiplog-coverage",
    "shiplog-cache",
    "shiplog-redact",
    "shiplog-bundle",
    "shiplog-workstreams",
    "shiplog-merge",
    "shiplog-render-md",
    "shiplog-render-json",
    "shiplog-ingest-json",
    "shiplog-ingest-manual",
    "shiplog-ingest-git",
    "shiplog-ingest-github",
    "shiplog-ingest-gitlab",
    "shiplog-ingest-jira",
    "shiplog-ingest-linear",
    "shiplog-cluster-llm",
    "shiplog-team",
    "shiplog-engine",
}

dev_only = {
    "shiplog-testkit",
}

workspace_ids = set(metadata["workspace_members"])
packages = {
    package["id"]: package
    for package in metadata["packages"]
    if package["id"] in workspace_ids
}
workspace_names = {package["name"] for package in packages.values()}

errors = []

overlap = published & dev_only
if overlap:
    errors.append(
        "package boundary audit has overlapping published/dev-only entries: "
        + ", ".join(sorted(overlap))
    )

for name in sorted((published | dev_only) - workspace_names):
    errors.append(f"classified package is not a workspace member: {name}")

for name in sorted(workspace_names - (published | dev_only)):
    errors.append(f"workspace package is not classified as published or dev-only: {name}")

for package in packages.values():
    name = package["name"]
    publish_value = package.get("publish")
    publish_false = publish_value == [] or publish_value is False

    if name in dev_only and not publish_false:
        errors.append(f"dev-only package must set publish = false: {name}")

    if name in published and publish_false:
        errors.append(f"published package must not set publish = false: {name}")

    if name in published:
        for dep in package.get("dependencies", []):
            if dep.get("kind") is not None:
                continue
            dep_name = dep["name"]
            if dep_name in dev_only:
                errors.append(
                    f"published package {name} has a normal dependency on dev-only {dep_name}"
                )

if errors:
    print("package boundary audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)

print(
    "package boundary audit passed: "
    f"{len(published)} published packages, {len(dev_only)} dev-only packages"
)
PY
