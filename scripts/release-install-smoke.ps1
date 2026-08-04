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

function Get-Sha256Hex {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $getFileHashCommand = Get-Command Get-FileHash -ErrorAction SilentlyContinue
    if ($getFileHashCommand) {
        return (Get-FileHash $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    }

    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $sha256 = [System.Security.Cryptography.SHA256]::Create()
        try {
            return ([System.BitConverter]::ToString($sha256.ComputeHash($stream))).Replace("-", "").ToLowerInvariant()
        }
        finally {
            $sha256.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

if ($Version -eq "-h" -or $Version -eq "--help") {
    @"
usage: scripts/release-install-smoke.ps1 <version>

Downloads the Windows GitHub release binary, or consumes the exact
workflow-staged candidate when SHIPLOG_RELEASE_CANDIDATE_DIR is set, verifies
SHA256SUMS.txt, proves the no-token first-use path and runs the no-network
review rescue smoke path. This script is intended to work without Rust or
Cargo installed.

Set SHIPLOG_RELEASE_REPO=owner/repo to verify a fork.
Set SHIPLOG_RELEASE_SMOKE_DIR=path to override the scratch directory.
Set SHIPLOG_RELEASE_CANDIDATE_DIR=path to consume the retained workflow
candidate instead of downloading release assets.
Set SHIPLOG_RELEASE_SOURCE_SHA=sha to bind a staged candidate to its source.
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
$candidateDir = $env:SHIPLOG_RELEASE_CANDIDATE_DIR
$sourceSha = $env:SHIPLOG_RELEASE_SOURCE_SHA

$asset = "shiplog-x86_64-pc-windows-msvc.exe"
$binaryPath = Join-Path $downloadDir "shiplog.exe"
$sumsPath = Join-Path $downloadDir "SHA256SUMS.txt"

Remove-Item -Recurse -Force $workDir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $downloadDir | Out-Null
if ($candidateDir) {
    $candidateDir = (Resolve-Path -LiteralPath $candidateDir).Path
    $candidateAsset = Join-Path (Join-Path $candidateDir $asset) $asset
    if (-not (Test-Path -LiteralPath $candidateAsset)) {
        $candidateAsset = Join-Path $candidateDir $asset
    }
    $candidateManifest = Join-Path $candidateDir "RELEASE_CANDIDATE.txt"
    $candidateSums = Join-Path $candidateDir "SHA256SUMS.txt"
    foreach ($path in @($candidateAsset, $candidateManifest, $candidateSums)) {
        if (-not (Test-Path -LiteralPath $path)) {
            throw "missing staged candidate file: $path"
        }
    }
    if (-not $sourceSha) {
        throw "SHIPLOG_RELEASE_SOURCE_SHA is required for a staged candidate"
    }
    if (-not (Get-Content -LiteralPath $candidateManifest | Where-Object { $_ -eq "release_tag=$tag" })) {
        throw "staged candidate tag does not match $tag"
    }
    if (-not (Get-Content -LiteralPath $candidateManifest | Where-Object { $_ -eq "source_sha=$sourceSha" })) {
        throw "staged candidate source SHA does not match $sourceSha"
    }
    Invoke-Step "consuming exact staged candidate for $repo@$tag"
    Copy-Item -LiteralPath $candidateAsset -Destination $binaryPath
    Copy-Item -LiteralPath $candidateSums -Destination $sumsPath
}
else {
    $baseUrl = "https://github.com/$repo/releases/download/$tag"
    Invoke-Step "downloading $repo@$tag release asset for Windows"
    Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$asset" -OutFile $binaryPath
    Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/SHA256SUMS.txt" -OutFile $sumsPath
}

Invoke-Step "verifying SHA256SUMS.txt entry for $asset"
$sumLine = Get-Content $sumsPath | ForEach-Object {
    $fields = $_ -split "\s+"
    if ($fields.Count -ge 2 -and [System.IO.Path]::GetFileName($fields[1].TrimStart("*")) -eq $asset) {
        $_
        break
    }
} | Select-Object -First 1
if (-not $sumLine) {
    throw "no SHA256SUMS.txt entry found for $asset"
}
$expectedSha = ($sumLine -split "\s+")[0].ToLowerInvariant()
$actualSha = Get-Sha256Hex $binaryPath
if ($actualSha -ne $expectedSha) {
    throw "checksum mismatch for $asset`nexpected: $expectedSha`nactual:   $actualSha"
}

Invoke-Step "smoking downloaded binary"
$versionOutput = & $binaryPath --version
if ($LASTEXITCODE -ne 0 -or $versionOutput.Trim() -ne "shiplog $versionNumber") {
    throw "unexpected version output: $versionOutput"
}
Invoke-Shiplog $binaryPath @("--help") | Out-Null

Invoke-Step "proving the no-token first-use path"
$coldStartDir = Join-Path $workDir "cold-start"
Remove-Item -Recurse -Force $coldStartDir -ErrorAction SilentlyContinue
$ghConfigDir = Join-Path $coldStartDir "gh-config"
New-Item -ItemType Directory -Force $ghConfigDir | Out-Null
foreach ($name in @("GITHUB_TOKEN", "GH_TOKEN", "GITLAB_TOKEN", "JIRA_TOKEN", "LINEAR_API_KEY", "SHIPLOG_REDACT_KEY")) {
    Remove-Item "Env:$name" -ErrorAction SilentlyContinue
}
$env:GH_CONFIG_DIR = $ghConfigDir

Push-Location -LiteralPath $coldStartDir
try {
    & $binaryPath | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "shiplog command failed: $binaryPath"
    }
    Invoke-Shiplog $binaryPath @("intake") | Out-Null
    $openPath = & $binaryPath open --print-path
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace(($openPath -join "").Trim())) {
        throw "shiplog open --print-path did not return a packet path"
    }
    if (-not (Test-Path -LiteralPath ($openPath -join "").Trim())) {
        throw "shiplog open --print-path returned a missing path: $openPath"
    }
    $statusJson = & $binaryPath status --latest --json
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace(($statusJson -join "").Trim())) {
        throw "shiplog status --latest --json returned no JSON"
    }
    $statusJson | Set-Content -LiteralPath (Join-Path $coldStartDir "status.latest.json") -Encoding utf8
    $eventDate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-dd")
    Invoke-Shiplog $binaryPath @(
        "add",
        "Published binary cold-start proof",
        "--date",
        $eventDate,
        "--description",
        "Verified the release binary from an empty directory without provider credentials."
    ) | Out-Null
    Invoke-Shiplog $binaryPath @("update", "--no-open") | Out-Null
}
finally {
    Pop-Location
}

$latestRun = Get-ChildItem -LiteralPath (Join-Path $coldStartDir "out") -Directory |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
if (-not $latestRun) {
    throw "no cold-start run directory produced under $coldStartDir\out"
}
foreach ($artifact in @(
    "packet.md",
    "intake.report.json",
    "ledger.events.jsonl",
    "coverage.manifest.json",
    "bundle.manifest.json"
)) {
    $artifactPath = Join-Path $latestRun.FullName $artifact
    if (-not (Test-Path -LiteralPath $artifactPath)) {
        throw "missing cold-start artifact: $artifactPath"
    }
}

# Structurally validate the receipts, not merely their existence: the
# published binary must parse its own intake.report.json/packet.md/ledger/
# coverage/bundle receipts back into their canonical shapes.
Invoke-Step "structurally validating cold-start receipts"
$reportJson = Join-Path $latestRun.FullName "intake.report.json"
Invoke-Shiplog $binaryPath @("report", "validate", "--path", $reportJson, "--receipts") | Out-Null

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
