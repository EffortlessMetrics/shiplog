# Install shiplog

Use a prebuilt binary when you want shiplog without installing Rust. The
versionless installers select the latest GitHub release, verify its SHA-256
checksum, and install to a user-local directory.

Current shipped release: `v0.10.0`.

## Prebuilt binary

### Linux and macOS

```bash
curl --proto '=https' --tlsv1.2 -fsSL \
  https://raw.githubusercontent.com/EffortlessMetrics/shiplog/main/scripts/install.sh \
  | bash
```

The installer supports Linux x86_64, macOS Intel, and macOS Apple Silicon.
Set `SHIPLOG_INSTALL_DIR` to choose another user-local directory or
`SHIPLOG_RELEASE_REPO=owner/repo` to use a fork.

The release asset names are:

```text
shiplog-x86_64-unknown-linux-gnu
shiplog-x86_64-apple-darwin
shiplog-aarch64-apple-darwin
shiplog-x86_64-pc-windows-msvc.exe
SHA256SUMS.txt
```

### Windows PowerShell

```powershell
& ([scriptblock]::Create((Invoke-WebRequest -UseBasicParsing `
  https://raw.githubusercontent.com/EffortlessMetrics/shiplog/main/scripts/install.ps1).Content))
```

Or run the checked-out script directly:

```powershell
.\scripts\install.ps1
```

Set `SHIPLOG_INSTALL_DIR` or pass `-InstallDir` to choose the destination.
The installer does not modify PATH automatically; it prints the directory to
add when the destination is not already on PATH.

The installers download only over HTTPS and refuse a checksum mismatch. They
do not require Rust, Cargo, provider tokens, or a Shiplog service. On Unix,
the installer uses `sha256sum shiplog` or `shasum -a 256 shiplog`; on Windows
it uses `Get-FileHash`.

## Cargo installation

With Rust already installed:

```bash
cargo install shiplog --locked
```

For a prebuilt Cargo installation without compiling from source, install
[`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall) and run:

```bash
cargo binstall shiplog --no-confirm
```

Shiplog publishes raw target-named release binaries for Linux x86_64, macOS
Intel, macOS Apple Silicon, and Windows x86_64. The package metadata disables
source compilation fallback so a failed prebuilt lookup is explicit.

## First use

From a work directory:

```bash
shiplog
```

This is read-only when no packet exists and points to the first useful action.
To collect the first packet explicitly:

```bash
shiplog intake
```

Local Git, manual evidence, and imported JSON work without provider credentials.
When an authenticated `gh` session is available, Shiplog can reuse it without
storing the credential. Setup and sharing diagnostics remain available through
`shiplog doctor`, `shiplog sources`, and `shiplog auth`. These commands do not
write intake evidence or render share artifacts automatically.

After installation, verify the binary and inspect the primary workflows:

```bash
shiplog --version
shiplog --help
shiplog intake --help
```

## Release binary smoke

To verify a specific published release from a repository checkout, use the
binary-only smoke scripts. They download the current-platform asset, verify
`SHA256SUMS.txt`, and run the no-network review-rescue fixture without Rust or
provider tokens.

Linux and macOS:

```bash
scripts/release-install-smoke.sh v0.10.0
```

Windows PowerShell:

```powershell
pwsh -File .\scripts\release-install-smoke.ps1 v0.10.0
```

These commands intentionally name the release under test. Ordinary install
instructions above are versionless.

## Package-manager status

The supported binary channels are the versionless installers, GitHub release
assets, cargo-binstall, and `cargo install`. Homebrew is not an official tap,
and Scoop and winget manifests are not official channels yet; do not treat
third-party packages as Shiplog releases.

## Safety boundaries

Shiplog is local-first and account-optional. Installation does not create a
cloud account, enable telemetry, or store provider credentials. Evidence
collection and manager/public sharing are separate operations; missing sharing
configuration should not prevent a local first packet.
