# build-install.ps1 -- fspec Source Build Installer (Windows)
#
# Builds the Rust binary from source and installs it.
# PowerShell mirror of scripts/build-install.sh (macOS / Linux).
#
# For a prebuilt binary (no Rust toolchain required), use scripts\install.ps1:
#   powershell -ExecutionPolicy Bypass -File scripts\install.ps1
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts\build-install.ps1
#   powershell -ExecutionPolicy Bypass -File scripts\build-install.ps1 -InstallDir C:\bin
#
# NOTE: This file is intentionally pure ASCII (see the note in build.ps1).

[CmdletBinding()]
param(
    # Installation directory (default: %USERPROFILE%\.local\bin)
    [string]$InstallDir = "",
    # Cargo build profile (default: release-slim; env BUILD_PROFILE used when -Profile absent)
    [string]$Profile = "",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if (-not $InstallDir) { $InstallDir = Join-Path $env:USERPROFILE ".local\bin" }
if (-not $Profile) { $Profile = $env:BUILD_PROFILE }
$BUILD_PROFILE = if ($Profile) { $Profile } else { "release-slim" }

function Write-Info { param([string]$msg) Write-Host "INFO: $msg" -ForegroundColor Cyan }
function Write-Success { param([string]$msg) Write-Host "[OK] $msg" -ForegroundColor Green }
function Write-Warning2 { param([string]$msg) Write-Host "[WARN] $msg" -ForegroundColor Yellow }
function Write-Error2 { param([string]$msg) Write-Host "[FAIL] $msg" -ForegroundColor Red }
function Write-Header { param([string]$msg) Write-Host $msg -ForegroundColor Magenta }

function Show-Usage {
    Write-Host @"
fspec Source Build Installer (Windows)

Builds the fspec Rust binary from source and installs it.
Requires a Rust toolchain (cargo via rustup), MSVC build tools, and git.

For a prebuilt binary instead, use: scripts\install.ps1

Usage: build-install.ps1 [options]

Options:
  -InstallDir <path>  Installation directory (default: %USERPROFILE%\.local\bin)
  -Profile <name>     Cargo build profile (default: release-slim)
  -Help               Show this help message

Environment variables:
  INSTALL_DIR        Installation directory (overrides -InstallDir)
  BUILD_PROFILE      Cargo build profile (overrides -Profile)

Examples:
  powershell -ExecutionPolicy Bypass -File scripts\build-install.ps1
  powershell -ExecutionPolicy Bypass -File scripts\build-install.ps1 -InstallDir C:\bin
"@
    exit 0
}

if ($Help) { Show-Usage }
if (-not $InstallDir) { $InstallDir = $env:INSTALL_DIR }
$INSTALL_DIR = if ($InstallDir) { $InstallDir } else { Join-Path $env:USERPROFILE ".local\bin" }

# -- Locate the repo (derive from script location; clone as fallback) --------
$REPO_ROOT = Split-Path -Parent $PSScriptRoot
Write-Header "fspec Source Build Installer (Windows)"
Write-Host ""
Write-Info "Checking prerequisites..."

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error2 "Rust (cargo) is required. Install via https://rustup.rs/"
    exit 1
}
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Error2 "git is required"
    exit 1
}
if (-not (Get-Command protoc -ErrorAction SilentlyContinue)) {
    Write-Warning2 "protoc not found on PATH -- the build may fail without it."
    Write-Warning2 "Download from https://github.com/protocolbuffers/protobuf/releases"
}

$CODELET_DIR = Join-Path $REPO_ROOT "rust"
if (-not (Test-Path (Join-Path $CODELET_DIR "Cargo.toml"))) {
    Write-Info "This repo does not look like the fspec source tree."
    Write-Info "Cloning fspec into a temp directory..."
    $REPO_ROOT = Join-Path ([System.IO.Path]::GetTempPath()) ("fspec-src-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
    git clone --depth 1 https://github.com/sengac/fspec.git $REPO_ROOT 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Error2 "git clone failed"
        exit 1
    }
    $CODELET_DIR = Join-Path $REPO_ROOT "rust"
}
if (-not (Test-Path (Join-Path $CODELET_DIR "Cargo.toml"))) {
    Write-Error2 "rust/Cargo.toml not found at $CODELET_DIR"
    Write-Error2 "This script must be run from within the fspec repository."
    exit 1
}

# -- Build --------------------------------------------------------------------
Write-Host ""
Write-Info "Building fspec (profile: $BUILD_PROFILE)..."

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

$BINARY = Join-Path $CODELET_DIR "target\$BUILD_PROFILE\fspec.exe"
if (-not (Test-Path $BINARY)) {
    Write-Error2 "Build artifact not found at $BINARY"
    Write-Error2 "The build may have failed. Check the output above."
    exit 1
}
Write-Success "Build complete: $BINARY"

# -- Install ------------------------------------------------------------------
Write-Host ""
Write-Info "Installing to $INSTALL_DIR..."
if (-not (Test-Path $INSTALL_DIR)) {
    Write-Info "Creating directory: $INSTALL_DIR"
    New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null
}
$installed = Join-Path $INSTALL_DIR "fspec.exe"
Copy-Item $BINARY $installed -Force
Write-Success "Installed to $installed"

# -- PATH check ---------------------------------------------------------------
if (($env:PATH -split ';') -notcontains $INSTALL_DIR) {
    Write-Host ""
    Write-Warning2 "Installation directory is not in your user PATH"
    Write-Host ""
    Write-Info "Add it with:"
    Write-Host ""
    Write-Host "  [Environment]::SetEnvironmentVariable('Path', `$env:Path + ';$INSTALL_DIR', 'User')"
    Write-Host ""
    Write-Info "Then open a new terminal, or run in the current one:"
    Write-Host ""
    Write-Host "  `$env:Path += ';$INSTALL_DIR'"
    Write-Host ""
}

# -- Verify -------------------------------------------------------------------
Write-Host ""
$prevEap = $ErrorActionPreference
$ErrorActionPreference = "Continue"
& (Join-Path $INSTALL_DIR "fspec.exe") --version
$verifyExit = $LASTEXITCODE
$ErrorActionPreference = $prevEap
if ($verifyExit -eq 0) {
    Write-Success "Version check passed"
} else {
    Write-Warning2 "Could not verify installation (exit code $verifyExit), but binary is in place"
}

Write-Host ""
Write-Success "Installation complete!"
Write-Info "Run 'fspec' to start the factory."
