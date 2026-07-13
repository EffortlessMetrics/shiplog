#!/usr/bin/env bash
# Run the same small gate CI enforces, locally, in one command.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

cargo xtask ci-small
