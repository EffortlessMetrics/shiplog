param(
    [string]$InstallDir
)

$ErrorActionPreference = "Stop"

if ($args.Count -gt 0 -and $args[0] -in @("-h", "--help")) {
    @"
Install the latest prebuilt shiplog binary.

Environment:
  SHIPLOG_RELEASE_REPO  GitHub owner/repo override
  SHIPLOG_INSTALL_DIR   User-local installation directory override
"@ | Write-Host
    exit 0
}
if ($args.Count -gt 0) {
    throw "unknown argument: $($args[0])"
}

$repo = if ($env:SHIPLOG_RELEASE_REPO) { $env:SHIPLOG_RELEASE_REPO } else { "EffortlessMetrics/shiplog" }
if (-not $InstallDir) {
    $InstallDir = if ($env:SHIPLOG_INSTALL_DIR) { $env:SHIPLOG_INSTALL_DIR } else { Join-Path $HOME ".local\bin" }
}

$asset = "shiplog-x86_64-pc-windows-msvc.exe"
$baseUrl = "https://github.com/$repo/releases/latest/download"
$workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("shiplog-install-" + [Guid]::NewGuid().ToString("N"))
$downloadPath = Join-Path $workDir $asset
$sumsPath = Join-Path $workDir "SHA256SUMS.txt"
$destination = Join-Path $InstallDir "shiplog.exe"

try {
    New-Item -ItemType Directory -Force $workDir | Out-Null
    Write-Host "==> downloading latest shiplog for $asset"
    Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$asset" -OutFile $downloadPath
    Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/SHA256SUMS.txt" -OutFile $sumsPath

    $escapedAsset = [Regex]::Escape($asset)
    $sumLine = Get-Content $sumsPath | Where-Object { $_ -match "(^|\s|/)$escapedAsset$" } | Select-Object -First 1
    if (-not $sumLine) {
        throw "no SHA256SUMS.txt entry found for $asset"
    }
    $expectedSha = ($sumLine -split "\s+")[0].ToLowerInvariant()
    $actualSha = (Get-FileHash $downloadPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualSha -ne $expectedSha) {
        throw "checksum mismatch for $asset`nexpected: $expectedSha`nactual:   $actualSha"
    }

    New-Item -ItemType Directory -Force $InstallDir | Out-Null
    Copy-Item -Force $downloadPath $destination
    Write-Host "Installed shiplog to $destination"
    $pathEntries = [Environment]::GetEnvironmentVariable("Path", "User") -split ";" | Where-Object { $_ }
    if ($pathEntries -notcontains $InstallDir) {
        Write-Host "Add this directory to your user PATH: $InstallDir"
    }
    & $destination --version
    if ($LASTEXITCODE -ne 0) {
        throw "installed shiplog failed --version"
    }
}
finally {
    Remove-Item -Recurse -Force $workDir -ErrorAction SilentlyContinue
}
