# build.ps1 -- Build fspec for Windows (native MSVC build)
#
# Builds the fspec Rust binary natively on Windows and optionally packages it.
# PowerShell mirror of scripts/build.sh (macOS / Linux native build).
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts\build.ps1
#   powershell -ExecutionPolicy Bypass -File scripts\build.ps1 -Package
#   powershell -ExecutionPolicy Bypass -File scripts\build.ps1 -Profile release
#
# Requirements:
#   - Rust via rustup (cargo on PATH) -- https://rustup.rs/
#   - MSVC build tools (link.exe) -- Visual Studio 2022 or Build Tools
#   - protoc on PATH (build-time only, required by nanograph/Lance)
#     Download: https://github.com/protocolbuffers/protobuf/releases
#
# NOTE: This file is intentionally pure ASCII. Windows PowerShell 5.1
# (powershell.exe) decodes BOM-less .ps1 files using the system ANSI code
# page (CP1252 on en-US systems); UTF-8 non-ASCII bytes in string literals
# (e.g. the checkmark U+2713 contains byte 0x93, a CP1252 double-quote)
# corrupt the parse. Keep any future edits ASCII-only for that reason.

[CmdletBinding()]
param(
    # Cargo build profile (default: release-slim; env BUILD_PROFILE is used
    # when -Profile is not given)
    [string]$Profile = "",
    # Build and package the zip for distribution into dist/
    [switch]$Package,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

$REPO_ROOT = Split-Path -Parent $PSScriptRoot
$CODELET_DIR = Join-Path $REPO_ROOT "rust"
$DIST_DIR = Join-Path $REPO_ROOT "dist"

# -- Color helpers ------------------------------------------------------------
function Write-Info { param([string]$msg) Write-Host "INFO: $msg" -ForegroundColor Cyan }
function Write-Success { param([string]$msg) Write-Host "[OK] $msg" -ForegroundColor Green }
function Write-Warning2 { param([string]$msg) Write-Host "[WARN] $msg" -ForegroundColor Yellow }
function Write-Error2 { param([string]$msg) Write-Host "[FAIL] $msg" -ForegroundColor Red }
function Write-Header { param([string]$msg) Write-Host $msg -ForegroundColor Magenta }

# -- Usage ---------------------------------------------------------------------
function Show-Usage {
    Write-Host @"
fspec Native Builder (Windows)

Builds the fspec Rust binary for the current Windows platform (native MSVC).

Usage: build.ps1 [options]

Options:
  -Profile <name>    Cargo build profile (default: release-slim)
  -Package            Build and package for distribution (zip in dist/)
  -Help               Show this help message

Environment variables:
  BUILD_PROFILE      Cargo build profile (used when -Profile is not given)

Examples:
  powershell -ExecutionPolicy Bypass -File scripts\build.ps1
  powershell -ExecutionPolicy Bypass -File scripts\build.ps1 -Package
  powershell -ExecutionPolicy Bypass -File scripts\build.ps1 -Profile release
"@
    exit 0
}

if ($Help) { Show-Usage }

# -- Resolve build profile (CLI flag wins over env, default release-slim) -----
if (-not $Profile) { $Profile = $env:BUILD_PROFILE }
$BUILD_PROFILE = if ($Profile) { $Profile } else { "release-slim" }

# -- Prerequisites -------------------------------------------------------------
Write-Header "fspec Native Builder (Windows)"
Write-Host ""
Write-Info "Checking prerequisites..."

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error2 "Rust (cargo) is required. Install via https://rustup.rs/"
    exit 1
}

if (-not (Get-Command protoc -ErrorAction SilentlyContinue)) {
    Write-Warning2 "protoc not found on PATH -- the build may fail without it."
    Write-Warning2 "Download from https://github.com/protocolbuffers/protobuf/releases"
    Write-Warning2 "(protoc-<version>-win64.zip) and add its bin\ dir to PATH."
}

# MSVC linker check (link.exe under Hostx64\x64) -- warn only: rustup can pick
# up Build Tools installs in non-default locations.
$linkExe = $null
foreach ($vsRoot in @("C:\Program Files\Microsoft Visual Studio", "C:\Program Files (x86)\Microsoft Visual Studio")) {
    if (Test-Path $vsRoot) {
        $linkExe = Get-ChildItem -Path $vsRoot -Recurse -Filter link.exe -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match "Hostx64\\x64" } |
            Select-Object -First 1
        if ($linkExe) { break }
    }
}
if (-not $linkExe -and -not (Get-Command link -ErrorAction SilentlyContinue)) {
    Write-Warning2 "MSVC link.exe not found under the default Visual Studio paths."
    Write-Warning2 "Install Visual Studio 2022 or the Build Tools (Desktop development with C++)."
}

# -- Detect platform -----------------------------------------------------------
switch ($env:PROCESSOR_ARCHITECTURE) {
    "AMD64"  { $ARCH_NAME = "x86_64"; $TARGET_TRIPLE = "x86_64-pc-windows-msvc" }
    "ARM64"  { $ARCH_NAME = "aarch64"; $TARGET_TRIPLE = "aarch64-pc-windows-msvc" }
    default  {
        Write-Error2 "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE"
        exit 1
    }
}

Write-Info "Platform: windows ($ARCH_NAME, $TARGET_TRIPLE)"
Write-Info "Build profile: $BUILD_PROFILE"

# -- Build ---------------------------------------------------------------------
Write-Host ""
Write-Info "Building fspec..."

$sw = [Diagnostics.Stopwatch]::StartNew()
# Cargo writes its progress to stderr; drop Stop-ActionPreference around the
# native call so stderr output is not treated as a terminating error, then
# check $LASTEXITCODE explicitly.
$prevEap = $ErrorActionPreference
$ErrorActionPreference = "Continue"
Push-Location $CODELET_DIR
try {
    cargo build --profile $BUILD_PROFILE -p codelet-fspec
    $buildExit = $LASTEXITCODE
} finally {
    Pop-Location
    $ErrorActionPreference = $prevEap
}
if ($buildExit -ne 0) {
    Write-Error2 "cargo build failed with exit code $buildExit"
    exit 1
}
$sw.Stop()
Write-Info ("Build took {0:N1} seconds" -f $sw.Elapsed.TotalSeconds)

$BINARY = Join-Path $CODELET_DIR "target\$BUILD_PROFILE\fspec.exe"

if (-not (Test-Path $BINARY)) {
    Write-Error2 "Build artifact not found at $BINARY"
    Write-Error2 "The build may have failed. Check the output above."
    exit 1
}

$sizeMB = [math]::Round((Get-Item $BINARY).Length / 1MB, 1)
Write-Success "Build complete: $BINARY"
Write-Info "Binary size: ${sizeMB} MB"

# -- Package (optional) --------------------------------------------------------
if ($Package) {
    Write-Host ""
    Write-Info "Packaging for distribution..."

    # Asset name is a contract with the UPD-002 self-updater:
    # fspec-<target-triple>.zip containing only the fspec binary.
    $archiveName = "fspec-$TARGET_TRIPLE.zip"
    $archivePath = Join-Path $DIST_DIR $archiveName
    if (Test-Path $archivePath) { Remove-Item $archivePath }

    New-Item -ItemType Directory -Force -Path $DIST_DIR | Out-Null
    # Compress-Archive with a single file path stores the file at the zip root
    # (no wrapper directory) -- matches the release.yml packaging contract.
    Compress-Archive -Path $BINARY -DestinationPath $archivePath

    $archiveMB = [math]::Round((Get-Item $archivePath).Length / 1MB, 1)
    Write-Success "Packaged: $archivePath"
    Write-Info "Archive size: ${archiveMB} MB"
}

# -- Verify --------------------------------------------------------------------
Write-Host ""
$prevEap = $ErrorActionPreference
$ErrorActionPreference = "Continue"
& $BINARY --version
$verifyExit = $LASTEXITCODE
$ErrorActionPreference = $prevEap
if ($verifyExit -eq 0) {
    Write-Success "Version check passed"
} else {
    Write-Warning2 "Could not verify build (exit code $verifyExit), but binary is in place"
}

Write-Host ""
Write-Success "Build complete!"
Write-Info "Binary: $BINARY"
Write-Info "To install locally: copy $BINARY to `$env:USERPROFILE\.local\bin\fspec.exe"
