#!/usr/bin/env bash
# build.sh — Build fspec for the current platform (macOS or Linux)
#
# Builds the fspec Rust binary natively and optionally packages it.
#
# Usage:
#   ./scripts/build.sh                    # Build for current platform
#   ./scripts/build.sh --package          # Build and package for distribution
#   ./scripts/build.sh --profile release  # Use release profile (with debug info)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CODELET_DIR="$REPO_ROOT/rust"
DIST_DIR="$REPO_ROOT/dist"
BUILD_PROFILE="${BUILD_PROFILE:-release-slim}"
DO_PACKAGE=false

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
fspec Native Builder (macOS / Linux)

Builds the fspec Rust binary for the current platform.

Usage: build.sh [options]

Options:
  --profile <name>   Cargo build profile (default: release-slim)
  --package          Build and package for distribution
  --help             Show this help message

Environment variables:
  BUILD_PROFILE      Cargo build profile (overrides --profile)

Examples:
  ./scripts/build.sh
  ./scripts/build.sh --package
  ./scripts/build.sh --profile release
EOF
  exit 0
}

# ── Parse arguments ──────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)  BUILD_PROFILE="$2"; shift 2 ;;
    --package)  DO_PACKAGE=true; shift ;;
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

header "fspec Native Builder"
echo ""

info "Checking prerequisites..."
check_prereq cargo "Rust (cargo) is required. Install via https://rustup.rs/"

# Check protoc (required by build.rs for protobuf compilation)
if ! command -v protoc &>/dev/null; then
  warning "protoc not found — the build may fail without it."
  warning "Install via: brew install protobuf (macOS) or apt install protobuf-compiler (Linux)"
fi

# ── Detect platform ──────────────────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)   PLATFORM="macos" ;;
  Linux)    PLATFORM="linux" ;;
  *)        error "Unsupported platform: $OS"; exit 1 ;;
esac

case "$ARCH" in
  arm64|aarch64)  ARCH_NAME="aarch64" ;;
  x86_64)         ARCH_NAME="x86_64" ;;
  *)              error "Unsupported architecture: $ARCH"; exit 1 ;;
esac

info "Platform: $PLATFORM ($ARCH_NAME)"
info "Build profile: $BUILD_PROFILE"

# ── Build ─────────────────────────────────────────────────────────────────────
echo ""
info "Building fspec..."

(
  cd "$CODELET_DIR"
  cargo build --profile "$BUILD_PROFILE" -p codelet-fspec
)

BINARY="$CODELET_DIR/target/$BUILD_PROFILE/fspec"

if [[ ! -f "$BINARY" ]]; then
  error "Build artifact not found at $BINARY"
  error "The build may have failed. Check the output above."
  exit 1
fi

success "Build complete: $BINARY"

# Show binary size
BINARY_SIZE=$(du -h "$BINARY" | cut -f1)
info "Binary size: $BINARY_SIZE"

# ── Package (optional) ────────────────────────────────────────────────────────
if [[ "$DO_PACKAGE" == "true" ]]; then
  echo ""
  info "Packaging for distribution..."

  # Determine target triple
  case "$OS" in
    Darwin)  TARGET_TRIPLE="${ARCH_NAME}-apple-darwin" ;;
    Linux)   TARGET_TRIPLE="${ARCH_NAME}-unknown-linux-gnu" ;;
  esac

  ARCHIVE_NAME="fspec-${TARGET_TRIPLE}.tar.gz"
  ARCHIVE_PATH="$DIST_DIR/$ARCHIVE_NAME"

  mkdir -p "$DIST_DIR"

  # Create tarball with the binary
  (
    cd "$CODELET_DIR/target/$BUILD_PROFILE"
    tar -czf "$ARCHIVE_PATH" fspec
  )

  success "Packaged: $ARCHIVE_PATH"
  ARCHIVE_SIZE=$(du -h "$ARCHIVE_PATH" | cut -f1)
  info "Archive size: $ARCHIVE_SIZE"
fi

# ── Verify ────────────────────────────────────────────────────────────────────
echo ""
if "$BINARY" --version &>/dev/null; then
  VERSION=$("$BINARY" --version 2>&1)
  success "Version check passed: $VERSION"
else
  warning "Could not verify build, but binary is in place"
fi

echo ""
success "Build complete!"
info "Binary: $BINARY"
info "To install locally: cp $BINARY ~/.local/bin/fspec"
