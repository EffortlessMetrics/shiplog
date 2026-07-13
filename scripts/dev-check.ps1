$ErrorActionPreference = "Stop"

$repoRoot = (& git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw "dev-check.ps1 must run inside a Git checkout"
}

Set-Location -LiteralPath $repoRoot
cargo xtask ci-small
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
