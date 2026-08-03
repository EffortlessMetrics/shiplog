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

function Find-UniqueCandidateFile {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Root,

        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    $candidateMatches = @(
        Get-ChildItem -LiteralPath $Root -Recurse -File -Force |
            Where-Object {
                $_.Name -eq $Name -and
                -not ($_.Attributes -band [System.IO.FileAttributes]::ReparsePoint)
            }
    )
    if ($candidateMatches.Count -ne 1) {
        throw "candidate bundle must contain exactly one $Name; found $($candidateMatches.Count) under $Root"
    }
    return $candidateMatches[0].FullName
}

function Assert-CandidateManifest {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CandidateRoot,

        [Parameter(Mandatory = $true)]
        [string]$ManifestPath,

        [Parameter(Mandatory = $true)]
        [string]$SumsPath,

        [Parameter(Mandatory = $true)]
        [string]$ExpectedTag,

        [Parameter(Mandatory = $true)]
        [string]$ExpectedSourceSha
    )

    if ($ExpectedSourceSha -notmatch '^[0-9a-fA-F]{40}$') {
        throw "SHIPLOG_RELEASE_SOURCE_SHA must be a full 40-character commit SHA"
    }

    $allowedKeys = @(
        'schema_version',
        'release_tag',
        'source_sha',
        'repository',
        'workflow_run_id',
        'workflow_run_attempt',
        'asset_count',
        'checksum_manifest_sha256'
    )
    $entries = @{}
    foreach ($line in Get-Content -LiteralPath $ManifestPath) {
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }
        if ($line -match '[\x00-\x1F\x7F]') {
            throw "candidate manifest contains a control character"
        }
        $parts = $line -split '=', 2
        if ($parts.Count -ne 2 -or
            [string]::IsNullOrEmpty($parts[0]) -or
            [string]::IsNullOrEmpty($parts[1])) {
            throw "malformed candidate manifest field"
        }
        $key = $parts[0]
        $value = $parts[1]
        if ($key -notin $allowedKeys) {
            throw "unknown candidate manifest field: $key"
        }
        if ($entries.ContainsKey($key)) {
            throw "duplicate candidate manifest field: $key"
        }
        $entries[$key] = $value
    }

    if ($entries.Count -ne $allowedKeys.Count) {
        throw "candidate manifest is missing required fields"
    }
    if ($entries['schema_version'] -ne '1') {
        throw "candidate manifest has unsupported schema"
    }
    if ($entries['release_tag'] -ne $ExpectedTag) {
        throw "candidate manifest is not bound to $ExpectedTag"
    }
    if ($entries['source_sha'] -ne $ExpectedSourceSha) {
        throw "candidate manifest is not bound to source commit $ExpectedSourceSha"
    }
    if ($entries['repository'] -notmatch '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$') {
        throw "candidate manifest has an invalid repository identity"
    }
    if ($entries['workflow_run_id'] -notmatch '^[0-9]+$' -or
        $entries['workflow_run_attempt'] -notmatch '^[0-9]+$') {
        throw "candidate manifest has an invalid workflow run identity"
    }
    if ($entries['asset_count'] -ne '4') {
        throw "candidate manifest does not record the four supported binaries"
    }

    foreach ($requiredAsset in @(
        'shiplog-x86_64-unknown-linux-gnu',
        'shiplog-x86_64-apple-darwin',
        'shiplog-aarch64-apple-darwin',
        'shiplog-x86_64-pc-windows-msvc.exe'
    )) {
        $null = Find-UniqueCandidateFile -Root $CandidateRoot -Name $requiredAsset
    }

    if ($entries['checksum_manifest_sha256'] -notmatch '^[0-9a-fA-F]{64}$') {
        throw "candidate manifest has no valid checksum manifest digest"
    }

    $actualSumsSha = Get-Sha256Hex $SumsPath
    if ($actualSumsSha -ne $entries['checksum_manifest_sha256'].ToLowerInvariant()) {
        throw "candidate SHA256SUMS.txt digest mismatch`nexpected: $($entries['checksum_manifest_sha256'])`nactual:   $actualSumsSha"
    }
}

if ($Version -eq "-h" -or $Version -eq "--help") {
    @"
usage: scripts/release-install-smoke.ps1 <version>

Verifies the Windows Shiplog release candidate, proves the no-token first-use
path, and runs the no-network review rescue smoke path. By default the script
downloads public GitHub Release assets. During staged release proof, point it at
the exact workflow candidate bundle instead.

Set SHIPLOG_RELEASE_CANDIDATE_DIR=path to use a local staged candidate bundle.
Set SHIPLOG_RELEASE_SOURCE_SHA=<full-sha> with candidate mode to bind the bundle
to the exact tagged source commit.
Set SHIPLOG_RELEASE_REPO=owner/repo to verify a public release or fork.
Set SHIPLOG_RELEASE_SMOKE_DIR=path to override the scratch directory.
"@ | Write-Error
    exit 2
}

$versionNumber = $Version.TrimStart("v")
$tag = "v$versionNumber"
$repo = if ($env:SHIPLOG_RELEASE_REPO) { $env:SHIPLOG_RELEASE_REPO } else { "EffortlessMetrics/shiplog" }
$candidateDir = $env:SHIPLOG_RELEASE_CANDIDATE_DIR
$expectedSourceSha = $env:SHIPLOG_RELEASE_SOURCE_SHA

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
$binaryPath = Join-Path $downloadDir "shiplog.exe"
$sumsPath = Join-Path $downloadDir "SHA256SUMS.txt"
$manifestPath = Join-Path $downloadDir "RELEASE_CANDIDATE.txt"

Remove-Item -Recurse -Force $workDir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $downloadDir | Out-Null
$workDir = (Resolve-Path -LiteralPath $workDir).ProviderPath
$downloadDir = (Resolve-Path -LiteralPath $downloadDir).ProviderPath
$demoOut = Join-Path $workDir "demo-out"
$binaryPath = Join-Path $downloadDir "shiplog.exe"
$sumsPath = Join-Path $downloadDir "SHA256SUMS.txt"
$manifestPath = Join-Path $downloadDir "RELEASE_CANDIDATE.txt"

if ($candidateDir) {
    if (-not $expectedSourceSha) {
        throw "SHIPLOG_RELEASE_SOURCE_SHA is required with SHIPLOG_RELEASE_CANDIDATE_DIR"
    }
    $candidateRoot = (Resolve-Path -LiteralPath $candidateDir).ProviderPath
    $candidateAsset = Find-UniqueCandidateFile -Root $candidateRoot -Name $asset
    $candidateSums = Find-UniqueCandidateFile -Root $candidateRoot -Name "SHA256SUMS.txt"
    $candidateManifest = Find-UniqueCandidateFile -Root $candidateRoot -Name "RELEASE_CANDIDATE.txt"

    Invoke-Step "loading staged candidate $tag for Windows"
    Copy-Item -LiteralPath $candidateAsset -Destination $binaryPath
    Copy-Item -LiteralPath $candidateSums -Destination $sumsPath
    Copy-Item -LiteralPath $candidateManifest -Destination $manifestPath
    Assert-CandidateManifest `
        -CandidateRoot $candidateRoot `
        -ManifestPath $manifestPath `
        -SumsPath $sumsPath `
        -ExpectedTag $tag `
        -ExpectedSourceSha $expectedSourceSha
}
else {
    $baseUrl = "https://github.com/$repo/releases/download/$tag"
    Invoke-Step "downloading $repo@$tag release asset for Windows"
    Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$asset" -OutFile $binaryPath
    Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/SHA256SUMS.txt" -OutFile $sumsPath
}

Invoke-Step "verifying SHA256SUMS.txt entry for $asset"
$sumLine = Get-Content $sumsPath | Where-Object {
    $parts = $_ -split "\s+"
    $parts.Count -ge 2 -and [System.IO.Path]::GetFileName($parts[-1]) -eq $asset
} | Select-Object -First 1
if (-not $sumLine) {
    throw "no SHA256SUMS.txt entry found for $asset"
}
$expectedSha = ($sumLine -split "\s+")[0].ToLowerInvariant()
$actualSha = Get-Sha256Hex $binaryPath
if ($actualSha -ne $expectedSha) {
    throw "checksum mismatch for $asset`nexpected: $expectedSha`nactual:   $actualSha"
}

Invoke-Step "smoking candidate binary"
try {
    $versionOutput = @(& $binaryPath --version)
    if ($LASTEXITCODE -ne 0) {
        throw "candidate binary failed --version"
    }
}
catch {
    throw "candidate binary failed --version: $($_.Exception.Message)"
}
$versionText = ($versionOutput -join [Environment]::NewLine).Trim()
if ($versionText -ne "shiplog $versionNumber") {
    throw "unexpected version output: $versionText"
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

# The intake report records its path relative to the original cold-start
# working directory. Validate from that same directory so the self-reference is
# resolved once rather than joined onto the run directory a second time.
Invoke-Step "structurally validating cold-start receipts"
Push-Location -LiteralPath $coldStartDir
try {
    $relativeReport = Join-Path "." (Join-Path "out" (Join-Path $latestRun.Name "intake.report.json"))
    Invoke-Shiplog $binaryPath @("report", "validate", "--path", $relativeReport, "--receipts") | Out-Null
}
finally {
    Pop-Location
}

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
if ($candidateDir) {
    Write-Host "staged release candidate smoke passed for $tag at $expectedSourceSha"
}
