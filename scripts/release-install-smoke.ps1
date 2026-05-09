param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Version
)

$ErrorActionPreference = "Stop"

function Invoke-Step {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Message
    )
    Write-Host "==> $Message"
}

function Invoke-Shiplog {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Binary,

        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    & $Binary @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "shiplog command failed: $Binary $($Arguments -join ' ')"
    }
}

if ($Version -eq "-h" -or $Version -eq "--help") {
    @"
usage: scripts/release-install-smoke.ps1 <version>

Downloads the Windows GitHub release binary, verifies SHA256SUMS.txt, and runs
the no-network review rescue smoke path. This script is intended to work without
Rust or Cargo installed.

Set SHIPLOG_RELEASE_REPO=owner/repo to verify a fork.
Set SHIPLOG_RELEASE_SMOKE_DIR=path to override the scratch directory.
"@ | Write-Error
    exit 2
}

$versionNumber = $Version.TrimStart("v")
$tag = "v$versionNumber"
$repo = if ($env:SHIPLOG_RELEASE_REPO) { $env:SHIPLOG_RELEASE_REPO } else { "EffortlessMetrics/shiplog" }

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir
$workDir = if ($env:SHIPLOG_RELEASE_SMOKE_DIR) {
    $env:SHIPLOG_RELEASE_SMOKE_DIR
}
else {
    Join-Path $repoRoot "target\release-install-smoke\$tag"
}
$downloadDir = Join-Path $workDir "download"
$demoOut = Join-Path $workDir "demo-out"

$asset = "shiplog-x86_64-pc-windows-msvc.exe"
$baseUrl = "https://github.com/$repo/releases/download/$tag"
$binaryPath = Join-Path $downloadDir "shiplog.exe"
$sumsPath = Join-Path $downloadDir "SHA256SUMS.txt"

Invoke-Step "downloading $repo@$tag release asset for Windows"
Remove-Item -Recurse -Force $workDir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $downloadDir | Out-Null
Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$asset" -OutFile $binaryPath
Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/SHA256SUMS.txt" -OutFile $sumsPath

Invoke-Step "verifying SHA256SUMS.txt entry for $asset"
$sumLine = Get-Content $sumsPath | Where-Object { $_ -match "/$([Regex]::Escape($asset))$" } | Select-Object -First 1
if (-not $sumLine) {
    throw "no SHA256SUMS.txt entry found for $asset"
}
$expectedSha = ($sumLine -split "\s+")[0].ToLowerInvariant()
$actualSha = (Get-FileHash $binaryPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualSha -ne $expectedSha) {
    throw "checksum mismatch for $asset`nexpected: $expectedSha`nactual:   $actualSha"
}

Invoke-Step "smoking downloaded binary"
$versionOutput = & $binaryPath --version
if ($LASTEXITCODE -ne 0 -or $versionOutput.Trim() -ne "shiplog $versionNumber") {
    throw "unexpected version output: $versionOutput"
}
Invoke-Shiplog $binaryPath @("init", "--dry-run") | Out-Null
Invoke-Shiplog $binaryPath @("intake", "--help") | Out-Null
Invoke-Shiplog $binaryPath @("share", "verify", "public", "--help") | Out-Null

Invoke-Step "running no-network review rescue fixture"
Remove-Item -Recurse -Force $demoOut -ErrorAction SilentlyContinue
& (Join-Path $scriptDir "demo-review-rescue.ps1") -ShiplogBin $binaryPath -Out $demoOut |
    Out-File -FilePath (Join-Path $workDir "demo-review-rescue.stdout") -Encoding utf8

if (-not (Get-ChildItem -Path $demoOut -Recurse -Filter "intake.report.md" | Select-Object -First 1)) {
    throw "no intake.report.md produced under $demoOut"
}
if (-not (Get-ChildItem -Path $demoOut -Recurse -Filter "packet.md" | Select-Object -First 1)) {
    throw "no packet.md produced under $demoOut"
}

Write-Host "release install smoke passed for $repo@$tag"
