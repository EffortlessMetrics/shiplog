#!/usr/bin/env bash
# Install a git pre-commit hook that runs fmt + clippy before each commit,
# so CI failures on formatting/lint show up locally first.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
hooks_dir="$repo_root/.git/hooks"
hook_path="$hooks_dir/pre-commit"

mkdir -p "$hooks_dir"

cat > "$hook_path" <<'HOOK'
#!/usr/bin/env bash
# Installed by scripts/install-hooks.sh. Skip with SHIPLOG_SKIP_HOOKS=1 or `git commit --no-verify`.
set -euo pipefail

if [[ -n "${SHIPLOG_SKIP_HOOKS:-}" ]]; then
  exit 0
fi

echo "pre-commit: cargo fmt --all -- --check"
if ! cargo fmt --all -- --check; then
  echo "pre-commit: formatting check failed; run 'cargo fmt --all' and re-stage" >&2
  exit 1
fi

echo "pre-commit: cargo clippy --workspace --all-targets --all-features -- -D warnings"
if ! cargo clippy --workspace --all-targets --all-features -- -D warnings; then
  echo "pre-commit: clippy failed; fix the warnings above" >&2
  exit 1
fi
HOOK

chmod +x "$hook_path"
echo "Installed pre-commit hook at $hook_path"
echo "Skip a single commit with 'git commit --no-verify', or set SHIPLOG_SKIP_HOOKS=1."
