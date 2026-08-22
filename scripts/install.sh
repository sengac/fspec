#!/usr/bin/env bash
# fspec Rust Installer (macOS / Linux)
# Builds the Rust binary from source and installs it.
#
# Usage:
#   ./scripts/install.sh                     # Run from repo root
#   curl ... | bash                           # Piped (auto-clones repo)
#   ./scripts/install.sh --dir /usr/local/bin
#   BUILD_PROFILE=release ./scripts/install.sh

set -euo pipefail

# ── Defaults ────────────────────────────────────────────────────────────────
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
BUILD_PROFILE="${BUILD_PROFILE:-release-slim}"
# Cap parallel rustc processes to bound peak memory. This mirrors the 2-4 vCPU
# ceiling of GitHub's runners (where this build succeeds without OOM). On a
# 20-core machine the default -j 20 spawns 20 concurrent rustc processes on
# heavy crates (lance, datafusion, arrow, tantivy) and OOMs.
# Override with BUILD_JOBS for faster builds on machines with headroom.
BUILD_JOBS="${BUILD_JOBS:-4}"

# Detect whether we're running from a file or piped
if [[ -n "${BASH_SOURCE[0]:-}" && -f "${BASH_SOURCE[0]}" ]]; then
  # Running from a file — derive repo root from script location
  REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
else
  # Piped or sourced — try to find repo root from cwd
  if [[ -d "$(pwd)/rust" ]]; then
    REPO_ROOT="$(pwd)"
  else
    # Not in a repo — clone it
    REPO_ROOT=""
  fi
fi

CODELET_DIR="$REPO_ROOT/rust"

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
fspec Rust Installer (macOS / Linux)

Builds the fspec Rust binary from source and installs it.

Usage: install.sh [options]

Options:
  --dir <path>       Installation directory (default: ~/.local/bin)
  --profile <name>   Cargo build profile (default: release-slim)
  --help             Show this help message

Environment variables:
  INSTALL_DIR        Installation directory (overrides --dir)
  BUILD_PROFILE      Cargo build profile (overrides --profile)
  BUILD_JOBS         Max parallel rustc processes (default: 4, bounds peak memory)

Examples:
  ./scripts/install.sh
  ./scripts/install.sh --dir /usr/local/bin
  BUILD_PROFILE=release ./scripts/install.sh
EOF
  exit 0
}

# ── Parse arguments ──────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dir)    INSTALL_DIR="$2"; shift 2 ;;
    --profile) BUILD_PROFILE="$2"; shift 2 ;;
    --help)   usage ;;
    *)        error "Unknown option: $1"; exit 1 ;;
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

header "fspec Rust Installer"
echo ""

info "Checking prerequisites..."
check_prereq cargo "Rust (cargo) is required. Install via https://rustup.rs/"
check_prereq git   "git is required"

# Check protoc (required by build.rs for protobuf compilation)
if ! command -v protoc &>/dev/null; then
  warning "protoc not found — the build may fail without it."
  warning "Install via: brew install protobuf (macOS) or apt install protobuf-compiler (Linux)"
fi

# ── Build ─────────────────────────────────────────────────────────────────────
echo ""
info "Building fspec (profile: $BUILD_PROFILE)..."

# If repo not found (piped without being in the repo), clone it
if [[ -z "$REPO_ROOT" ]]; then
  info "Cloning fspec repository..."
  REPO_ROOT="$(mktemp -d)"
  git clone --depth 1 https://github.com/sengac/fspec.git "$REPO_ROOT" >/dev/null 2>&1
  CODELET_DIR="$REPO_ROOT/rust"
fi

if [[ ! -d "$CODELET_DIR" ]]; then
  error "rust/ directory not found at $CODELET_DIR"
  error "This script must be run from within the fspec repository."
  exit 1
fi

(
  cd "$CODELET_DIR"
  cargo build --profile "$BUILD_PROFILE" -p codelet-fspec -j "$BUILD_JOBS"
)

BINARY="$CODELET_DIR/target/$BUILD_PROFILE/fspec"

if [[ ! -f "$BINARY" ]]; then
  error "Build artifact not found at $BINARY"
  error "The build may have failed. Check the output above."
  exit 1
fi

success "Build complete: $BINARY"

# ── Install ───────────────────────────────────────────────────────────────────
echo ""
info "Installing to $INSTALL_DIR..."

if [[ ! -d "$INSTALL_DIR" ]]; then
  info "Creating directory: $INSTALL_DIR"
  mkdir -p "$INSTALL_DIR"
fi

cp "$BINARY" "$INSTALL_DIR/fspec"
chmod +x "$INSTALL_DIR/fspec"

success "Installed to $INSTALL_DIR/fspec"

# ── PATH check ────────────────────────────────────────────────────────────────
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
  echo ""
  warning "Installation directory is not in PATH"
  echo ""
  info "Add the following to your shell config (~/.zshrc or ~/.bashrc):"
  echo ""
  echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
  echo ""
  info "Then run: source ~/.zshrc  (or ~/.bashrc)"
  echo ""
fi

# ── Verify ────────────────────────────────────────────────────────────────────
echo ""
if "$INSTALL_DIR/fspec" --version &>/dev/null; then
  VERSION=$("$INSTALL_DIR/fspec" --version 2>&1)
  success "Version check passed: $VERSION"
else
  warning "Could not verify installation, but binary is in place"
fi

echo ""
success "Installation complete!"
info "Run 'fspec' to start the factory."
