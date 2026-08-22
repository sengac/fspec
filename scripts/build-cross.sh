#!/usr/bin/env bash
# build-cross.sh — Cross-compile fspec for other platforms
#
# Cross-compiles Windows and Linux binaries from macOS/Linux.
# Uses cargo-xwin for Windows, cargo-zigbuild for Linux/macOS.
# No Docker required.
#
# Usage:
#   ./scripts/build-cross.sh                              # Build x86_64 Windows
#   ./scripts/build-cross.sh --target x86_64-pc-windows-msvc
#   ./scripts/build-cross.sh --target aarch64-unknown-linux-gnu
#   ./scripts/build-cross.sh --all                         # Build all targets

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CODELET_DIR="$REPO_ROOT/rust"
DIST_DIR="$REPO_ROOT/dist"
BUILD_PROFILE="${BUILD_PROFILE:-release-slim}"

# ── Color helpers ────────────────────────────────────────────────────────────
if [[ -t 1 ]]; then
  RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'
  CYAN='\033[0;36m'; MAGENTA='\033[0;35m'; NC='\033[0m'
else
  RED=''; GREEN=''; YELLOW=''; CYAN=''; MAGENTA=''; NC=''
fi

info()     { echo -e "${CYAN}INFO: $*${NC}" >&2; }
success()  { echo -e "${GREEN}✓ $*${NC}" >&2; }
warning()  { echo -e "${YELLOW}⚠ $*${NC}" >&2; }
error()    { echo -e "${RED}✗ $*${NC}" >&2; }
header()   { echo -e "${MAGENTA}$*${NC}" >&2; }

# ── Usage ─────────────────────────────────────────────────────────────────────
usage() {
  cat <<EOF
fspec Cross-Compilation Builder

Cross-compiles fspec for Windows and Linux from macOS/Linux.
Uses cargo-xwin for Windows, cargo-zigbuild for Linux/macOS.
No Docker required.

Usage: build-cross.sh [options]

Options:
  --target <triple>  Target triple (default: x86_64-pc-windows-msvc)
  --profile <name>   Cargo build profile (default: release-slim)
  --all              Build all supported targets
  --help             Show this help message

Environment variables:
  BUILD_PROFILE      Cargo build profile (overrides --profile)

Supported targets:
  Windows:
    x86_64-pc-windows-msvc
    aarch64-pc-windows-msvc

  Linux:
    x86_64-unknown-linux-gnu
    aarch64-unknown-linux-gnu

  macOS:
    x86_64-apple-darwin
    aarch64-apple-darwin
    universal2-apple-darwin

Examples:
  ./scripts/build-cross.sh
  ./scripts/build-cross.sh --target aarch64-unknown-linux-gnu
  ./scripts/build-cross.sh --all

Prerequisites:
  - Rust (cargo) installed
  - Homebrew (macOS) or apt (Linux)
  - All other tools are auto-installed by the script
EOF
  exit 0
}

# ── Parse arguments ──────────────────────────────────────────────────────────
BUILD_ALL=false
TARGET="${TARGET:-x86_64-pc-windows-msvc}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)   TARGET="$2"; shift 2 ;;
    --profile)  BUILD_PROFILE="$2"; shift 2 ;;
    --all)      BUILD_ALL=true; shift ;;
    --help)     usage ;;
    *)          error "Unknown option: $1"; exit 1 ;;
  esac
done

# ── Prerequisites ─────────────────────────────────────────────────────────────
check_prereq() {
  local cmd="$1" msg="$2"
  if ! command -v "$cmd" &>/dev/null; then
    error "$msg"
    exit 1
  fi
}

header "fspec Cross-Compilation Builder"
echo ""

info "Checking prerequisites..."
check_prereq cargo   "Rust (cargo) is required. Install via https://rustup.rs/"

# ── Install cargo-xwin (Windows cross-compilation) ────────────────────────────
if ! command -v cargo-xwin &>/dev/null; then
  info "cargo-xwin not found — installing..."
  cargo install --locked cargo-xwin
  success "cargo-xwin installed"
else
  info "cargo-xwin found: $(cargo-xwin --version 2>&1 || echo 'unknown version')"
fi

# ── Install cargo-zigbuild (Linux/macOS cross-compilation) ────────────────────
if ! command -v cargo-zigbuild &>/dev/null; then
  info "cargo-zigbuild not found — installing..."
  cargo install --locked cargo-zigbuild
  success "cargo-zigbuild installed"
else
  info "cargo-zigbuild found"
fi

# ── Install Zig (required by cargo-zigbuild) ──────────────────────────────────
if ! command -v zig &>/dev/null; then
  info "zig not found — installing via Homebrew..."
  if command -v brew &>/dev/null; then
    brew install zig
  else
    info "Homebrew not found — trying pip..."
    pip3 install ziglang
  fi
  success "zig installed"
fi

# ── Install full LLVM (required by cargo-xwin) ────────────────────────────────
if ! command -v llvm-lib &>/dev/null; then
  if [[ "$(uname)" == "Darwin" ]]; then
    info "llvm-lib not found — installing LLVM via Homebrew..."
    if ! command -v brew &>/dev/null; then
      error "Homebrew is required. Install via https://brew.sh/"
      exit 1
    fi
    brew install llvm
    success "LLVM installed"
  else
    info "llvm-lib not found — installing via apt..."
    sudo apt install -y llvm
    success "LLVM installed"
  fi
fi

# ── Install lld (required linker for MSVC cross-compilation) ─────────────────
if ! command -v lld-link &>/dev/null; then
  info "lld-link not found — installing lld via Homebrew..."
  if command -v brew &>/dev/null; then
    brew install lld
  else
    sudo apt install -y lld
  fi
  success "lld installed"
fi

# ── Add LLVM and lld to PATH (keg-only on macOS) ──────────────────────────────
LLVM_BIN="$(dirname "$(which llvm-lib 2>/dev/null || echo '/opt/homebrew/opt/llvm/bin/llvm-lib')")"
if [[ -d "$LLVM_BIN" ]]; then
  export PATH="$LLVM_BIN:$PATH"
  info "Added LLVM to PATH: $LLVM_BIN"
fi

LLD_BIN="$(dirname "$(which lld-link 2>/dev/null || echo '/opt/homebrew/opt/lld/bin/lld-link')")"
if [[ -d "$LLD_BIN" ]]; then
  export PATH="$LLD_BIN:$PATH"
  info "Added lld to PATH: $LLD_BIN"
fi

# ── Add llvm-tools rustup component ───────────────────────────────────────────
if ! rustup component list --installed 2>/dev/null | grep -q "llvm-tools"; then
  info "llvm-tools component not found — adding..."
  rustup component add llvm-tools
  success "llvm-tools installed"
fi

# ── Determine build tool based on target ──────────────────────────────────────
get_build_tool() {
  local target="$1"
  if [[ "$target" == *"windows"* ]]; then
    echo "cargo-xwin"
  else
    echo "cargo-zigbuild"
  fi
}

# ── Build function ────────────────────────────────────────────────────────────
build_target() {
  local target_triple="$1"
  local build_tool
  build_tool="$(get_build_tool "$target_triple")"

  local binary_name="fspec"
  [[ "$target_triple" == *"windows"* ]] && binary_name="fspec.exe"

  echo ""
  header "Building for ${target_triple}..."

  # Add target if not already installed.
  # NOTE: resolve the active toolchain from within $CODELET_DIR so that
  # rust/rust-toolchain.toml is respected. Running this from the repo root
  # would resolve the user's default toolchain, which may not be the one
  # cargo actually uses for the build (toolchain mismatch → E0463).
  ACTIVE_TOOLCHAIN="$(cd "$CODELET_DIR" && rustup show active-toolchain | head -1 | sed 's/ .*//')"
  INSTALLED_TARGETS="$(rustup target list --installed --toolchain "$ACTIVE_TOOLCHAIN" 2>/dev/null)"

  if ! echo "$INSTALLED_TARGETS" | grep -q "$target_triple"; then
    info "Adding target $target_triple to toolchain $ACTIVE_TOOLCHAIN..."
    rustup target add "$target_triple" --toolchain "$ACTIVE_TOOLCHAIN"
    success "Target $target_triple installed"
  fi

  # Build
  info "Building with $build_tool..."
  if [[ "$build_tool" == "cargo-xwin" ]]; then
    (
      cd "$CODELET_DIR"
      cargo xwin build --profile "$BUILD_PROFILE" --target "$target_triple" -p codelet-fspec
    )
  else
    # For x86_64 Linux targets, override CC and CFLAGS to use zig with AVX-512 support
    # This is required because lance-linalg's build.rs uses -march=native which
    # targets the host CPU (ARM64 on macOS), not the target (x86_64 Linux).
    # zig cc can cross-compile AVX-512 code from ARM64, but needs explicit CPU flags.
    # For aarch64 Linux, no AVX-512 flags needed (NEON handles SIMD).
    #
    # TARGET_GLIBC_VERSION=2.17 pins the glibc floor so the binary runs on any
    # x86_64/aarch64 Linux distro with glibc >= 2.17 (CentOS 7+). Without this,
    # zig links against the newest glibc symbols it knows about (e.g. 2.39),
    # which breaks on older distros.
    #
    # RUSTFLAGS="-C codegen-units=1" reduces the object-file count at link time.
    # On macOS this is required to avoid ProcessFdQuotaExceeded / UnableToSpawnSelf
    # linker failures with the default 16 codegen units.
    local zig_cc="zig cc"
    local zig_cflags=""
    if [[ "$target_triple" == "x86_64"* ]]; then
      zig_cflags="-mcpu=sapphirerapids -mavx512bw"
    fi
    (
      cd "$CODELET_DIR"
      TARGET_CC="$zig_cc" TARGET_CFLAGS="$zig_cflags" TARGET_GLIBC_VERSION=2.17 \
        RUSTFLAGS="-C codegen-units=1" \
        cargo zigbuild --profile "$BUILD_PROFILE" --target "$target_triple" -p codelet-fspec
    )
  fi

  # Package
  local binary_path="$CODELET_DIR/target/$target_triple/$BUILD_PROFILE/$binary_name"
  if [[ ! -f "$binary_path" ]]; then
    error "Build artifact not found at $binary_path"
    exit 1
  fi

  mkdir -p "$DIST_DIR"

  local archive_name="fspec-${target_triple}"
  local archive_path

  if [[ "$target_triple" == *"windows"* ]]; then
    archive_path="$DIST_DIR/${archive_name}.zip"
    info "Packaging: ${archive_name}.zip"
    (
      cd "$CODELET_DIR/target/$target_triple/$BUILD_PROFILE"
      zip -j "$archive_path" "$binary_name"
    )
  else
    archive_path="$DIST_DIR/${archive_name}.tar.gz"
    info "Packaging: ${archive_name}.tar.gz"
    (
      cd "$CODELET_DIR/target/$target_triple/$BUILD_PROFILE"
      tar -czf "$archive_path" "$binary_name"
    )
  fi

  success "Built and packaged: $archive_path"
}

# ── Main ──────────────────────────────────────────────────────────────────────
if [[ "$BUILD_ALL" == "true" ]]; then
  build_target "x86_64-pc-windows-msvc"
  build_target "aarch64-unknown-linux-gnu"
  build_target "x86_64-unknown-linux-gnu"
  build_target "aarch64-apple-darwin"
  build_target "x86_64-apple-darwin"
else
  build_target "$TARGET"
fi

echo ""
success "Cross-compilation complete!"
info "Binaries are in $DIST_DIR/"
ls -lh "$DIST_DIR/"
