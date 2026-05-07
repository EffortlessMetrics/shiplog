#!/usr/bin/env bash
set -euo pipefail

mkdir -p target
cargo metadata --format-version 1 --no-deps > target/package-version-metadata.json

python - <<'PY'
import json
import sys
from pathlib import Path

metadata = json.loads(Path("target/package-version-metadata.json").read_text())
workspace_ids = set(metadata["workspace_members"])
packages = {
    package["id"]: package
    for package in metadata["packages"]
    if package["id"] in workspace_ids
}
by_name = {package["name"]: package for package in packages.values()}

target = by_name.get("shiplog")
if target is None:
    print("package version audit failed:", file=sys.stderr)
    print("- workspace package `shiplog` not found", file=sys.stderr)
    sys.exit(1)

target_version = target["version"]
workspace_names = set(by_name)
errors = []

for name, package in sorted(by_name.items()):
    if package["version"] != target_version:
        errors.append(
            f"workspace package {name} is {package['version']}, expected {target_version}"
        )

for name, package in sorted(by_name.items()):
    for dep in package.get("dependencies", []):
        if dep["name"] not in workspace_names:
            continue
        if dep.get("kind") is not None:
            continue
        req = dep.get("req", "")
        expected = f"^{target_version}"
        if req != expected:
            errors.append(
                f"normal workspace dependency {name} -> {dep['name']} "
                f"uses requirement {req!r}, expected {expected!r}"
            )

if errors:
    print("package version audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)

print(
    "package version audit passed: "
    f"{len(by_name)} workspace packages at {target_version}"
)
PY
