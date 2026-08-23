#!/usr/bin/env bash
# fspec Native Installer (macOS / Linux)
# Downloads and installs the latest fspec binary from GitHub Releases.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.sh | bash
#   ./scripts/install.sh                     # Run from repo root
#   ./scripts/install.sh --dir /usr/local/bin
#   INSTALL_DIR=/usr/local/bin ./scripts/install.sh
#
# Requires only: curl, tar, and a sha256 tool (sha256sum/shasum/sha256).
# To build from source instead, use scripts/build-install.sh.

set -euo pipefail

# ── Configuration ────────────────────────────────────────────────────────────
REPO="sengac/fspec"
BIN_NAME="fspec"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
GITHUB_API="https://api.github.com/repos/$REPO/releases?per_page=10"
GITHUB_RELEASES="https://github.com/$REPO/releases/download"

# ── Color helpers (stderr only, so piped stdout stays clean) ─────────────────
if [[ -t 2 ]]; then
  RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
  CYAN='\033[0;36m'; MAGENTA='\033[0;35m'; NC='\033[0m'
else
  RED=''; GREEN=''; YELLOW=''; CYAN=''; MAGENTA=''; NC=''
fi

info()    { echo -e "${CYAN}INFO: $*${NC}" >&2; }
success() { echo -e "${GREEN}✓ $*${NC}" >&2; }
warning() { echo -e "${YELLOW}⚠ $*${NC}" >&2; }
error()   { echo -e "${RED}✗ $*${NC}" >&2; }
header()  { echo -e "${MAGENTA}$*${NC}" >&2; }

# ── Usage ────────────────────────────────────────────────────────────────────
usage() {
  cat <<EOF
fspec Native Installer (macOS / Linux)

Downloads the latest prebuilt fspec binary from GitHub Releases.
Requires only curl, tar, and a sha256 tool — no Rust toolchain needed.

Usage: install.sh [options]

Options:
  --dir <path>   Installation directory (default: ~/.local/bin)
  --help         Show this help message

Environment variables:
  INSTALL_DIR    Installation directory (overrides --dir)

Examples:
  curl -fsSL https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.sh | bash
  ./scripts/install.sh --dir /usr/local/bin

To build from source instead, use: scripts/build-install.sh
EOF
  exit 0
}

# ── Parse arguments ──────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dir)  INSTALL_DIR="$2"; shift 2 ;;
    --help) usage ;;
    *)      error "Unknown option: $1"; usage; exit 1 ;;
  esac
done

# ── Prerequisites ────────────────────────────────────────────────────────────
check_prereq() {
  local cmd="$1" msg="$2"
  if ! command -v "$cmd" &>/dev/null; then
    error "$msg"
    exit 1
  fi
}

header "fspec Native Installer"
echo "" >&2

info "Checking prerequisites..."
check_prereq curl "curl is required for installation"
check_prereq tar  "tar is required to extract the release archive"

# ── Platform detection ───────────────────────────────────────────────────────
# Maps the host to the release asset target triple (UPD-002 contract:
# asset filename is fspec-<target-triple>.tar.gz, no version segment).
detect_platform() {
  local uname_s uname_m
  uname_s=$(uname -s)
  uname_m=$(uname -m)

  case "$uname_s" in
    Darwin)
      case "$uname_m" in
        arm64)  echo "aarch64-apple-darwin" ;;
        x86_64) echo "x86_64-apple-darwin" ;;
        *)      error "Unsupported macOS architecture: $uname_m"; exit 1 ;;
      esac
      ;;
    Linux)
      case "$uname_m" in
        x86_64)   echo "x86_64-unknown-linux-gnu" ;;
        aarch64|arm64) echo "aarch64-unknown-linux-gnu" ;;
        *)        error "Unsupported Linux architecture: $uname_m"; exit 1 ;;
      esac
      ;;
    MINGW*|MSYS*)
      error "Windows native is not supported by this script."
      error "Use WSL (then run this script), or the PowerShell installer:"
      error "  powershell -ExecutionPolicy ByPass -c \"irm https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.ps1 | iex\""
      exit 1
      ;;
    *)
      error "Unsupported OS: $uname_s"
      exit 1
      ;;
  esac
}

PLATFORM="$(detect_platform)"
info "Detected platform: $PLATFORM"

# ── Release resolution ───────────────────────────────────────────────────────
# Asset URL for a given tag (tag keeps its v prefix in the URL path).
asset_url() {
  local tag="$1"
  echo "${GITHUB_RELEASES}/${tag}/fspec-${PLATFORM}.tar.gz"
}

# HEAD-check whether a release has an asset for this platform.
asset_exists() {
  local url="$1"
  curl -fIsL --connect-timeout 10 "$url" >/dev/null 2>&1
}

# Fallback: follow the releases/latest redirect to get the tag.
fetch_latest_tag_fallback() {
  local location
  location=$(curl -fsSIL --connect-timeout 10 "https://github.com/$REPO/releases/latest" \
    | tr -d '\r' \
    | awk '/^location:/ {print $2}' \
    | tail -n1)
  [[ -z "$location" ]] && return 1
  echo "$location" | sed -nE 's|.*/tag/([^/]+)$|\1|p'
}

# Find the most recent release that has an asset for this platform.
find_release_tag() {
  local response_file
  response_file=$(mktemp)
  local tags=""

  if curl -fsSL --connect-timeout 10 "$GITHUB_API" > "$response_file" 2>/dev/null; then
    tags=$(grep -oE '"tag_name":[[:space:]]*"[^"]+"' "$response_file" | cut -d'"' -f4)
  fi
  rm -f "$response_file"

  if [[ -n "$tags" ]]; then
    local tag
    for tag in $tags; do
      if asset_exists "$(asset_url "$tag")"; then
        echo "$tag"
        return 0
      fi
    done
  else
    warning "GitHub API unavailable — falling back to the releases/latest redirect"
  fi

  # Fallback: latest release tag only.
  local fallback_tag
  fallback_tag=$(fetch_latest_tag_fallback) || return 1
  if asset_exists "$(asset_url "$fallback_tag")"; then
    echo "$fallback_tag"
    return 0
  fi
  return 1
}

info "Resolving latest release with a $PLATFORM asset..."
RELEASE_TAG="$(find_release_tag)" || {
  error "No release with a $PLATFORM asset found"
  exit 1
}
success "Found release: $RELEASE_TAG ($PLATFORM)"

# ── Download ─────────────────────────────────────────────────────────────────
TEMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t fspec)"
trap 'rm -rf "$TEMP_DIR"' EXIT INT TERM

ARCHIVE="$TEMP_DIR/fspec-${PLATFORM}.tar.gz"
DOWNLOAD_URL="$(asset_url "$RELEASE_TAG")"

info "Downloading $DOWNLOAD_URL ..."
if ! curl -fSL -# -o "$ARCHIVE" "$DOWNLOAD_URL"; then
  error "Failed to download $DOWNLOAD_URL"
  exit 1
fi
success "Downloaded $(du -h "$ARCHIVE" | cut -f1)"

# ── Checksum verification ────────────────────────────────────────────────────
verify_checksum() {
  local archive="$1"
  local release_tag="$2"
  local basename_file
  basename_file=$(basename "$archive")

  local checksums_url="${GITHUB_RELEASES}/${release_tag}/checksums.txt"
  local checksums_file="$TEMP_DIR/checksums.txt"

  if ! curl -fsSL --connect-timeout 10 -o "$checksums_file" "$checksums_url" 2>/dev/null; then
    warning "checksums.txt not found on release $release_tag — skipping checksum verification"
    return 0
  fi

  # checksums.txt format: <hash>  <filename> (sha256sum, two spaces)
  local expected
  expected=$(grep -F "$basename_file" "$checksums_file" 2>/dev/null | awk '{print $1}' | head -n1)
  if [[ -z "$expected" ]]; then
    warning "No checksum entry for $basename_file — skipping checksum verification"
    return 0
  fi

  local actual=""
  if command -v sha256sum &>/dev/null; then
    actual=$(sha256sum "$archive" | awk '{print $1}')
  elif command -v shasum &>/dev/null; then
    actual=$(shasum -a 256 "$archive" | awk '{print $1}')
  elif command -v sha256 &>/dev/null; then
    actual=$(sha256 -q "$archive")
  else
    warning "No checksum tool (sha256sum/shasum/sha256) found — skipping checksum verification"
    return 0
  fi

  if [[ "$actual" != "$expected" ]]; then
    error "Checksum mismatch for $basename_file!"
    error "Expected: $expected"
    error "Got:      $actual"
    exit 1
  fi

  success "Checksum verified: $expected"
}

info "Verifying binary integrity..."
verify_checksum "$ARCHIVE" "$RELEASE_TAG"

# ── Extract ──────────────────────────────────────────────────────────────────
info "Extracting binary..."
tar -xzf "$ARCHIVE" -C "$TEMP_DIR"

EXTRACTED_BINARY="$(find "$TEMP_DIR" -type f -name "$BIN_NAME" | head -1)"
if [[ -z "$EXTRACTED_BINARY" ]]; then
  error "Binary not found in archive"
  exit 1
fi

# ── Install ──────────────────────────────────────────────────────────────────
echo "" >&2
info "Installing to $INSTALL_DIR..."

if [[ ! -d "$INSTALL_DIR" ]]; then
  info "Creating directory: $INSTALL_DIR"
  mkdir -p "$INSTALL_DIR"
fi

if ! cp "$EXTRACTED_BINARY" "$INSTALL_DIR/$BIN_NAME"; then
  error "Failed to install binary to $INSTALL_DIR/$BIN_NAME"
  info "You may need to use: sudo cp $EXTRACTED_BINARY $INSTALL_DIR/$BIN_NAME"
  exit 1
fi
chmod +x "$INSTALL_DIR/$BIN_NAME"
success "Installed to $INSTALL_DIR/$BIN_NAME"

# ── PATH check ───────────────────────────────────────────────────────────────
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
  echo "" >&2
  warning "Installation directory is not in PATH"
  echo "" >&2
  info "Add the following to your shell config (~/.zshrc or ~/.bashrc):"
  echo "" >&2
  echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
  echo "" >&2
  info "Then run: source ~/.zshrc  (or ~/.bashrc)"
  echo "" >&2
fi

# ── Verify ───────────────────────────────────────────────────────────────────
echo "" >&2
if "$INSTALL_DIR/$BIN_NAME" --version &>/dev/null; then
  VERSION=$("$INSTALL_DIR/$BIN_NAME" --version 2>&1)
  success "Version check passed: $VERSION"
else
  warning "Could not verify installation, but binary is in place"
fi

echo "" >&2
success "Installation complete!"
info "Run 'fspec' to start."
