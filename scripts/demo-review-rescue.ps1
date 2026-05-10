param(
    [string]$Out = ".\out\demo-review-rescue",
    [string]$ShiplogBin = (Join-Path $PSScriptRoot "..\target\release\shiplog.exe"),
    [string]$Config = "examples/configs/local-git-json-manual.toml"
)

$ErrorActionPreference = "Stop"

function Resolve-DemoPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return (Join-Path (Get-Location) $Path)
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

function Invoke-WithoutProviderTokens {
    param(
        [Parameter(Mandatory = $true)]
        [scriptblock]$Script
    )

    $names = @("GITHUB_TOKEN", "GITLAB_TOKEN", "JIRA_TOKEN", "LINEAR_API_KEY")
    $oldValues = @{}
    foreach ($name in $names) {
        $oldValues[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
        [Environment]::SetEnvironmentVariable($name, $null, "Process")
    }

    try {
        & $Script
    }
    finally {
        foreach ($name in $names) {
            [Environment]::SetEnvironmentVariable($name, $oldValues[$name], "Process")
        }
    }
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir
$outPath = Resolve-DemoPath $Out
$shiplogPath = if ($ShiplogBin -match "[\\/]") {
    Resolve-DemoPath $ShiplogBin
}
else {
    $ShiplogBin
}
$configPath = if ([System.IO.Path]::IsPathRooted($Config)) {
    $Config
}
else {
    Join-Path $repoRoot $Config
}

Write-Host "==> running review rescue demo"
Write-Host "out: $outPath"
Write-Host "config: $configPath"

New-Item -ItemType Directory -Force $outPath | Out-Null
Push-Location $repoRoot
try {
    Invoke-WithoutProviderTokens {
        Invoke-Shiplog $shiplogPath @(
            "intake",
            "--out",
            $outPath,
            "--config",
            $configPath,
            "--no-open",
            "--explain"
        )

        Write-Host ""
        Write-Host "==> intake report"
        Invoke-Shiplog $shiplogPath @("open", "intake-report", "--out", $outPath, "--latest", "--print-path")

        Write-Host ""
        Write-Host "==> commands-only fixups"
        Invoke-Shiplog $shiplogPath @("review", "fixups", "--out", $outPath, "--latest", "--commands-only")

        Write-Host ""
        Write-Host "==> manager share preflight"
        Invoke-Shiplog $shiplogPath @("share", "verify", "manager", "--out", $outPath, "--latest", "--redact-key", "fixture-key")
    }
}
finally {
    Pop-Location
}

if (-not (Get-ChildItem -Path $outPath -Recurse -Filter "intake.report.md" | Select-Object -First 1)) {
    throw "no intake.report.md produced under $outPath"
}
if (-not (Get-ChildItem -Path $outPath -Recurse -Filter "packet.md" | Select-Object -First 1)) {
    throw "no packet.md produced under $outPath"
}

Write-Host ""
Write-Host "review rescue demo passed"
