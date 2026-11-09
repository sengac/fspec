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

# BUG-564: install fspec.exe while fspec runs. Windows cannot overwrite or
# delete a running .exe, but it can rename one. So the new binary is copied to
# a temporary file in the install directory, the current fspec.exe is renamed
# aside to fspec.exe.<id>.old, and the temporary file is renamed to fspec.exe.
# Running fspec processes keep the old file until they exit; each install
# removes the fspec.exe.*.old files no process holds any more. On failure the
# temporary file is removed and fspec.exe is left as it was. This function is
# identical in build-install.ps1 and install.ps1 (a test checks it).
function Install-FspecBinary {
  param(
    [string]$Source,
    [string]$Target
  )

  $installDir = Split-Path -Parent $Target
  $name = Split-Path -Leaf $Target
  # Old binaries still held by a running fspec stay until a later install.
  Get-ChildItem -LiteralPath $installDir -Filter "$name.*.old" -File -ErrorAction SilentlyContinue |
    ForEach-Object { Remove-Item -LiteralPath $_.FullName -Force -ErrorAction SilentlyContinue }

  $id = [guid]::NewGuid().ToString("N")
  $temp = Join-Path $installDir ".fspec-install-$id.tmp"
  $old = "$Target.$id.old"
  try {
    Copy-Item -LiteralPath $Source -Destination $temp -Force -ErrorAction Stop
    if (Test-Path -LiteralPath $Target) {
      Move-Item -LiteralPath $Target -Destination $old -ErrorAction Stop
    }
    Move-Item -LiteralPath $temp -Destination $Target -ErrorAction Stop
  }
  catch {
    if (Test-Path -LiteralPath $temp) {
      Remove-Item -LiteralPath $temp -Force -ErrorAction SilentlyContinue
    }
    if (-not (Test-Path -LiteralPath $Target) -and (Test-Path -LiteralPath $old)) {
      Move-Item -LiteralPath $old -Destination $Target -ErrorAction SilentlyContinue
    }
    throw
  }
  Write-Host "Running fspec processes keep the old version until restarted."
}

# -- Install ------------------------------------------------------------------
Write-Host ""
Write-Info "Installing to $INSTALL_DIR..."
if (-not (Test-Path $INSTALL_DIR)) {
    Write-Info "Creating directory: $INSTALL_DIR"
    New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null
}
$installed = Join-Path $INSTALL_DIR "fspec.exe"
try {
    Install-FspecBinary -Source $BINARY -Target $installed
}
catch {
    Write-Error2 "Failed to install $installed"
    Write-Error2 "Error: $_"
    exit 1
}
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
