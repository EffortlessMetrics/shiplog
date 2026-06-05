# Install shiplog

Use the GitHub release binary when you need shiplog quickly and do not already
have Rust installed. Use `cargo install` when you already have a Rust toolchain
or want optional Cargo features.

Latest shipped release assets for `v0.8.0`:

```text
shiplog-x86_64-unknown-linux-gnu
shiplog-x86_64-apple-darwin
shiplog-aarch64-apple-darwin
shiplog-x86_64-pc-windows-msvc.exe
SHA256SUMS.txt
```

## GitHub release binary

### Windows PowerShell

```powershell
$version = "v0.8.0"
$asset = "shiplog-x86_64-pc-windows-msvc.exe"
$base = "https://github.com/EffortlessMetrics/shiplog/releases/download/$version"
$bin = "$HOME\bin"

New-Item -ItemType Directory -Force $bin | Out-Null
Invoke-WebRequest "$base/$asset" -OutFile "$bin\shiplog.exe"
Invoke-WebRequest "$base/SHA256SUMS.txt" -OutFile "$bin\SHA256SUMS.txt"

$expected = (Select-String -Path "$bin\SHA256SUMS.txt" -Pattern $asset).Line.Split(" ")[0]
$actual = (Get-FileHash "$bin\shiplog.exe" -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actual -ne $expected) {
  throw "checksum mismatch for $asset"
}

$env:Path = "$bin;$env:Path"
shiplog --version
shiplog intake --help
```

Add `$HOME\bin` to your user `PATH` if you want `shiplog` to be available in
new shells.

### macOS

Pick the asset for your CPU:

```bash
# Apple Silicon
asset=shiplog-aarch64-apple-darwin

# Intel
asset=shiplog-x86_64-apple-darwin
```

Then download, verify, and install:

```bash
version=v0.8.0
base="https://github.com/EffortlessMetrics/shiplog/releases/download/$version"

curl -fsSLo shiplog "$base/$asset"
curl -fsSLo SHA256SUMS.txt "$base/SHA256SUMS.txt"

expected="$(grep "$asset$" SHA256SUMS.txt | awk '{print $1}')"
actual="$(shasum -a 256 shiplog | awk '{print $1}')"
test "$actual" = "$expected"

chmod +x shiplog
mkdir -p "$HOME/bin"
mv shiplog "$HOME/bin/shiplog"
"$HOME/bin/shiplog" --version
"$HOME/bin/shiplog" intake --help
```

### Linux x86_64

```bash
version=v0.8.0
asset=shiplog-x86_64-unknown-linux-gnu
base="https://github.com/EffortlessMetrics/shiplog/releases/download/$version"

curl -fsSLo shiplog "$base/$asset"
curl -fsSLo SHA256SUMS.txt "$base/SHA256SUMS.txt"

expected="$(grep "$asset$" SHA256SUMS.txt | awk '{print $1}')"
actual="$(sha256sum shiplog | awk '{print $1}')"
test "$actual" = "$expected"

chmod +x shiplog
mkdir -p "$HOME/bin"
mv shiplog "$HOME/bin/shiplog"
"$HOME/bin/shiplog" --version
"$HOME/bin/shiplog" intake --help
```

Add `$HOME/bin` to your shell `PATH` if needed.

## Cargo install

Use this when Rust is already installed:

```bash
cargo install shiplog --locked
shiplog --version
shiplog intake --help
```

Install the optional LLM-assisted workstream clustering feature explicitly:

```bash
cargo install shiplog --locked --features llm
```

## From source

```bash
git clone https://github.com/EffortlessMetrics/shiplog.git
cd shiplog
cargo install --path apps/shiplog
```

Developers working inside the repository can run:

```bash
cargo run -p shiplog -- <subcommand>
```

## Package-manager status

These channels are not the primary install path yet:

- `cargo-binstall`: planned tracking item; use GitHub release binaries or
  `cargo install` for now.
- Homebrew: planned tap work; no official tap is documented here yet.
- Scoop and winget: planned Windows distribution work; use the GitHub release
  binary until those manifests exist.

Do not install a third-party package unless you trust its publisher and version.

## Smoke test

After any install method:

```bash
shiplog --version
shiplog init --dry-run
shiplog doctor --setup --help
shiplog status --help
shiplog intake --help
shiplog share verify manager --help
```

Then start the review loop:

```bash
shiplog init --guided
shiplog doctor --setup
shiplog status --latest
shiplog intake --last-6-months --explain
shiplog status --latest
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog runs diff --latest
shiplog share explain manager --latest
```

Use `open intake-report --latest` and `open packet --latest` when you want to
inspect the generated files after any intake run. Use the read-first repair
handoff before writing manual evidence: `repair plan` chooses the safe repair
ID, `journal add --from-repair` writes local manual evidence only, and the
diff/share commands read receipts before any explicit share rendering.

When a future 0.9 release is explicitly approved, the release smoke should
include this setup/status path before intake. Do not run a 0.9 release-install
smoke while the 0.9 hold is active.

## Release binary smoke

Developers verifying a shipped release from a repository checkout can use the
binary-only smoke scripts. They download the current-platform GitHub release
asset, verify `SHA256SUMS.txt`, run the first-run help checks, and run the
fixture-backed review rescue demo without provider tokens or Rust installed.

Linux and macOS:

```bash
scripts/release-install-smoke.sh v0.8.0
```

Windows PowerShell:

```powershell
pwsh -File .\scripts\release-install-smoke.ps1 v0.8.0
```

The smoke path runs:

```bash
shiplog --version
shiplog init --dry-run
shiplog doctor --setup --help
shiplog status --help
shiplog intake --help
shiplog share verify public --help
scripts/demo-review-rescue.sh --out ./out/demo-review-rescue
```

On Windows, use `pwsh -File .\scripts\demo-review-rescue.ps1` for the same
no-network fixture path. The demo uses
`examples/configs/local-git-json-manual.toml` so it can exercise local git,
JSON, and manual evidence without live provider credentials.
