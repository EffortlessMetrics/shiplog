#!/usr/bin/env bash
set -euo pipefail

mkdir -p target
metadata_path="${PACKAGE_BOUNDARY_METADATA_PATH:-target/package-boundary-metadata.json}"
policy_path="${PACKAGE_BOUNDARY_POLICY_PATH:-policy/publish-allowlist.toml}"

if [[ -z "${PACKAGE_BOUNDARY_METADATA_PATH:-}" ]]; then
  mkdir -p "$(dirname "$metadata_path")"
  cargo metadata --format-version 1 --no-deps > "$metadata_path"
fi

export PACKAGE_BOUNDARY_METADATA_PATH="$metadata_path"
export PACKAGE_BOUNDARY_POLICY_PATH="$policy_path"

python_bin="${PYTHON:-}"
if [[ -z "$python_bin" ]]; then
  if command -v python3 >/dev/null 2>&1; then
    python_bin=python3
  elif command -v python >/dev/null 2>&1; then
    python_bin=python
  else
    echo "python3 or python is required for package boundary audit" >&2
    exit 127
  fi
fi

"$python_bin" - <<'PY'
import json
import os
import sys
from pathlib import Path

metadata_path = Path(os.environ["PACKAGE_BOUNDARY_METADATA_PATH"])
policy_path = Path(os.environ["PACKAGE_BOUNDARY_POLICY_PATH"])
metadata = json.loads(metadata_path.read_text())

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python <3.11 is not expected in CI.
    print("Python 3.11+ is required for tomllib", file=sys.stderr)
    sys.exit(2)

policy = tomllib.loads(policy_path.read_text())
publish_order = policy.get("publish", {}).get("default_order", [])
transitional_exceptions = set(
    policy.get("publish", {}).get("transitional_exceptions", [])
)
package_entries = policy.get("package", [])
historical_entries = policy.get("historical", [])
packages_by_name = {entry.get("name"): entry for entry in package_entries}
historical_by_name = {entry.get("name"): entry for entry in historical_entries}

workspace_ids = set(metadata["workspace_members"])
packages = {
    package["id"]: package
    for package in metadata["packages"]
    if package["id"] in workspace_ids
}
workspace_names = {package["name"] for package in packages.values()}

errors = []
allowed_tiers = {
    "public-supported",
    "public-transitional",
    "internal-module",
    "dev-only",
}

if len(packages_by_name) != len(package_entries):
    names = [entry.get("name") for entry in package_entries]
    duplicates = sorted({name for name in names if names.count(name) > 1})
    errors.append(
        "publish allowlist has duplicate package entries: "
        + ", ".join(name for name in duplicates if name)
    )

if len(historical_by_name) != len(historical_entries):
    names = [entry.get("name") for entry in historical_entries]
    duplicates = sorted({name for name in names if names.count(name) > 1})
    errors.append(
        "publish allowlist has duplicate historical entries: "
        + ", ".join(name for name in duplicates if name)
    )

for name in sorted(set(packages_by_name) & set(historical_by_name)):
    errors.append(f"package cannot be both current and historical: {name}")

if len(publish_order) != len(set(publish_order)):
    errors.append("publish.default_order contains duplicate package names")

for name in sorted(workspace_names - set(packages_by_name)):
    errors.append(f"workspace package is not classified in publish allowlist: {name}")

for name in sorted(set(packages_by_name) - workspace_names):
    errors.append(f"publish allowlist package is not a workspace member: {name}")

for name in publish_order:
    entry = packages_by_name.get(name)
    if entry is None:
        if name in historical_by_name:
            errors.append(f"historical package must not enter publish.default_order: {name}")
        else:
            errors.append(f"publish.default_order package is not classified: {name}")
        continue
    if entry.get("publish") is not True:
        errors.append(f"publish.default_order package must set publish = true: {name}")

for entry in historical_entries:
    name = entry.get("name")
    tier = entry.get("tier")
    reason = str(entry.get("reason", "")).strip()

    if not name:
        errors.append("historical entry is missing a package name")
    if tier != "historical-0.6":
        errors.append(f"{name}: historical entries must use tier 'historical-0.6'")
    if not reason:
        errors.append(f"{name}: historical entry reason is empty")
    if name in publish_order:
        errors.append(f"{name}: historical package must not enter publish.default_order")

for package in packages.values():
    name = package["name"]
    entry = packages_by_name.get(name)
    if not entry:
        continue

    tier = entry.get("tier")
    publish_enabled = entry.get("publish")
    reason = str(entry.get("reason", "")).strip()
    publish_value = package.get("publish")
    publish_false = publish_value == [] or publish_value is False

    if tier not in allowed_tiers:
        errors.append(f"{name}: unknown support tier {tier!r}")

    if not reason:
        errors.append(f"{name}: publish allowlist reason is empty")

    if publish_enabled is not True and publish_enabled is not False:
        errors.append(f"{name}: publish must be true or false in publish allowlist")

    if tier == "public-supported" and publish_enabled is not True:
        errors.append(f"{name}: public-supported packages must be publish enabled")

    if tier == "public-supported" and name not in publish_order:
        errors.append(f"{name}: public-supported package is missing from publish.default_order")

    if tier == "public-transitional" and publish_enabled is True:
        if name not in transitional_exceptions:
            errors.append(
                f"{name}: public-transitional publish requires a named exception"
            )
        if name not in publish_order:
            errors.append(
                f"{name}: public-transitional publish exception is missing from publish.default_order"
            )

    if tier in {"internal-module", "dev-only"} and publish_enabled is True:
        errors.append(f"{name}: {tier} package must not be publish enabled")

    if publish_enabled is False and not publish_false:
        errors.append(f"non-published package must set Cargo publish = false: {name}")

    if publish_enabled is True and publish_false:
        errors.append(f"publish-enabled package must not set Cargo publish = false: {name}")

    if publish_enabled is True:
        for dep in package.get("dependencies", []):
            if dep.get("kind") is not None:
                continue
            dep_name = dep["name"]
            if dep_name in historical_by_name:
                errors.append(
                    f"published package {name} has a normal dependency on "
                    f"historical 0.6 package {dep_name}"
                )
            dep_entry = packages_by_name.get(dep_name)
            if dep_entry and dep_entry.get("publish") is not True:
                errors.append(
                    f"published package {name} has a normal dependency on "
                    f"non-publishable workspace package {dep_name} "
                    f"({dep_entry.get('tier')})"
                )
            elif dep_name in workspace_names and dep_entry is None:
                errors.append(
                    f"published package {name} has a normal dependency on "
                    f"unclassified workspace package {dep_name}"
                )

enabled_names = {
    name for name, entry in packages_by_name.items() if entry.get("publish") is True
}
if set(publish_order) != enabled_names:
    errors.append(
        "publish.default_order must exactly match publish-enabled packages: "
        f"order={publish_order}, enabled={sorted(enabled_names)}"
    )

if errors:
    print("package boundary audit failed:", file=sys.stderr)
    for error in errors:
        print(f"- {error}", file=sys.stderr)
    sys.exit(1)

print(
    "package boundary audit passed: "
    f"{len(publish_order)} publish-allowed package(s), "
    f"{sum(1 for e in package_entries if e.get('tier') == 'public-transitional')} transitional, "
    f"{sum(1 for e in package_entries if e.get('tier') == 'internal-module')} internal, "
    f"{sum(1 for e in package_entries if e.get('tier') == 'dev-only')} dev-only, "
    f"{len(historical_entries)} historical"
)
PY
