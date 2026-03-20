#!/usr/bin/env bash

# fspec Native Installer
# Downloads and installs the latest fspec SEA binary from GitHub Releases
# Supports: macOS (Intel/Apple Silicon), Linux (x64/ARM64)

set -euo pipefail

# Check for curl existence
if ! command -v curl >/dev/null 2>&1; then
  printf '✗ curl is required for installation\n' >&2
  exit 1
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
REPO="sengac/fspec"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
BIN_NAME="fspec"
GITHUB_API="https://api.github.com/repos/$REPO/releases?per_page=10"
GITHUB_RELEASES="https://github.com/$REPO/releases/download"

# Ensure INSTALL_DIR exists
mkdir -p "$INSTALL_DIR"

# Hide/show cursor
show_cursor() { printf "\033[?25h"; }
hide_cursor() { printf "\033[?25l"; }
trap show_cursor EXIT INT TERM

# Logging functions (all output to stderr to avoid interfering with command output)
log_info() {
  printf '%b\n' "${BLUE}INFO:${NC} $1" >&2
}

log_success() {
  printf '%b\n' "${GREEN}✓${NC} $1" >&2
}

log_error() {
  printf '%b\n' "${RED}✗${NC} $1" >&2
}

log_warning() {
  printf '%b\n' "${YELLOW}⚠${NC} $1" >&2
}

# Spinner for long running tasks
show_spinner() {
  local pid=$1
  local msg=$2
  local frames='⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏'
  local n=10
  local i=0

  # Don't show spinner if not in a TTY
  if [[ ! -t 2 ]]; then
    return 0
  fi

  hide_cursor
  while kill -0 "$pid" 2>/dev/null; do
    local frame="${frames:i:1}"
    printf "\r${BLUE}%s${NC} %s" "$frame" "$msg" >&2
    i=$(( (i + 1) % n ))
    sleep 0.1
  done
  printf "\r\033[K" >&2 # Clear line
  show_cursor
}

# Detect OS and architecture
detect_platform() {
  local os
  local arch
  local uname_s
  local uname_m

  uname_s=$(uname -s)
  uname_m=$(uname -m)

  case "$uname_s" in
    Darwin)
      os="apple-darwin"
      if [[ "$uname_m" == "arm64" ]]; then
        arch="aarch64"
      else
        arch="x86_64"
      fi
      ;;
    Linux)
      os="unknown-linux-gnu"
      if [[ "$uname_m" == "x86_64" ]]; then
        arch="x86_64"
      elif [[ "$uname_m" == "aarch64" || "$uname_m" == "arm64" ]]; then
        arch="aarch64"
      else
        log_error "Unsupported Linux architecture: $uname_m"
        exit 1
      fi
      ;;
    MINGW*|MSYS*)
      log_error "Windows native is not supported via this script. Use install.ps1 for PowerShell."
      exit 1
      ;;
    *)
      log_error "Unsupported OS: $uname_s"
      exit 1
      ;;
  esac

  echo "${arch}-${os}"
}

# Candidate platforms by preference for current host
get_candidate_platforms() {
  local uname_s
  local uname_m
  uname_s=$(uname -s)
  uname_m=$(uname -m)

  case "$uname_s-$uname_m" in
    Linux-x86_64)
      echo "x86_64-unknown-linux-gnu"
      ;;
    Linux-aarch64|Linux-arm64)
      echo "aarch64-unknown-linux-gnu"
      ;;
    Darwin-arm64)
      echo "aarch64-apple-darwin x86_64-apple-darwin"
      ;;
    Darwin-x86_64)
      echo "x86_64-apple-darwin"
      ;;
    *)
      detect_platform
      ;;
  esac
}

# Fetch recent releases info from GitHub API
fetch_recent_releases() {
  local response_file
  response_file=$(mktemp)

  # Try to fetch releases with a 10s timeout
  (curl -fsSL --connect-timeout 10 "$GITHUB_API" > "$response_file" 2>/dev/null) &
  local pid=$!
  show_spinner "$pid" "Fetching recent releases..."
  wait "$pid" || true

  local response
  response=$(cat "$response_file")
  rm -f "$response_file"

  if [[ -z "$response" || "$response" == "[]" ]]; then
    log_error "Failed to fetch releases info from GitHub API or no releases found"
    log_info "Ensure you have internet connection and GitHub is accessible"
    exit 1
  fi

  echo "$response"
}

# Extract download URL for the detected platform
get_download_url() {
  local release_tag="$1"
  local platform="$2"

  # Determine file extension based on platform
  local file_ext
  if [[ "$platform" == *"darwin"* ]] || [[ "$platform" == *"linux"* ]]; then
    file_ext="tar.gz"
  else
    file_ext="zip"
  fi

  # Strip 'v' prefix from tag for filename only
  local version_tag="${release_tag#v}"
  local filename="fspec-${version_tag}-${platform}.${file_ext}"
  echo "${GITHUB_RELEASES}/${release_tag}/${filename}"
}

# Check if a specific version is available for the platform
check_version_available() {
  local version="$1"
  local platform="$2"

  local download_url
  download_url=$(get_download_url "$version" "$platform")

  # Use HEAD request to check availability with a timeout
  if curl -fIsL --connect-timeout 5 "$download_url" > /dev/null 2>&1; then
    return 0
  else
    return 1
  fi
}

# Find the most recent release with assets for the given platform
find_latest_release_tag() {
  local all_releases="$1"
  local platform="$2"

  # Extract all tag names
  local tags
  tags=$(echo "$all_releases" | grep -oE '"tag_name":\s*"[^"]+"' | cut -d'"' -f4)

  if [[ -z "$tags" ]]; then
    return 1
  fi

  # Iterate through tags to find one with assets
  for tag in $tags; do
    if [[ -n "$tag" ]]; then
      if check_version_available "$tag" "$platform"; then
        echo "$tag"
        return 0
      fi
    fi
  done

  return 1
}

# Download binary with progress
download_binary() {
  local url="$1"
  local output_file="$2"

  log_info "Downloading binary..."

  # Use curl -# for a simple progress bar
  if ! curl -fSL -# -o "$output_file" "$url"; then
    log_error "Failed to download binary from $url"
    exit 1
  fi
}

# Verify checksum if available
verify_checksum() {
  local binary_file="$1"
  local release_tag="$2"

  log_info "Verifying binary integrity..."

  local basename_file
  basename_file=$(basename "$binary_file")

  local temp_checksums
  temp_checksums=$(mktemp)

  # Try to download checksums.txt first
  local checksums_url="${GITHUB_RELEASES}/${release_tag}/checksums.txt"
  local expected_checksum=""

  if curl -fsSL -o "$temp_checksums" "$checksums_url" 2>/dev/null; then
    expected_checksum=$(grep "$basename_file" "$temp_checksums" 2>/dev/null | awk '{print $1}' || true)
  fi

  # If not found in checksums.txt, try individual .sha256 file
  if [[ -z "$expected_checksum" ]]; then
    local sha_url="${GITHUB_RELEASES}/${release_tag}/${basename_file%.tar.gz}.sha256"
    if curl -fsSL -o "$temp_checksums" "$sha_url" 2>/dev/null; then
      expected_checksum=$(cat "$temp_checksums" | awk '{print $1}')
    fi
  fi

  rm -f "$temp_checksums"

  if [[ -z "$expected_checksum" ]]; then
    log_warning "Checksum not found for $basename_file, skipping verification"
    return 0
  fi

  # Compute actual checksum
  local actual_checksum=""
  if command -v sha256sum &> /dev/null; then
    actual_checksum=$(sha256sum "$binary_file" | awk '{print $1}')
  elif command -v shasum &> /dev/null; then
    actual_checksum=$(shasum -a 256 "$binary_file" | awk '{print $1}')
  elif command -v sha256 &> /dev/null; then
    actual_checksum=$(sha256 -q "$binary_file")
  else
    log_warning "No checksum tool (sha256sum/shasum/sha256) found, skipping verification"
    return 0
  fi

  if [[ "$actual_checksum" != "$expected_checksum" ]]; then
    log_error "Checksum mismatch for $basename_file!"
    log_error "Expected: $expected_checksum"
    log_error "Got:      $actual_checksum"
    exit 1
  fi

  log_success "Checksum verified: $expected_checksum"
}

# Extract binary from archive
extract_binary() {
  local archive="$1"
  local platform="$2"
  local temp_dir
  temp_dir=$(mktemp -d)

  log_info "Extracting binary..."

  if [[ "$platform" == *"darwin"* ]] || [[ "$platform" == *"linux"* ]]; then
    tar -xzf "$archive" -C "$temp_dir"
  else
    if command -v 7z &> /dev/null; then
      7z x "$archive" -o"$temp_dir" > /dev/null
    elif command -v unzip &> /dev/null; then
      unzip -q "$archive" -d "$temp_dir"
    else
      log_error "Neither 7z nor unzip found. Cannot extract archive."
      exit 1
    fi
  fi

  # Find the binary
  local binary_path
  binary_path=$(find "$temp_dir" -type f \( -name "$BIN_NAME" -o -name "$BIN_NAME.exe" \) | head -1)

  if [[ -z "$binary_path" ]]; then
    log_error "Binary not found in archive"
    exit 1
  fi

  echo "$binary_path"
}

# Install binary to target directory
install_binary() {
  local source="$1"
  local target="$2"

  log_info "Installing to $target..."

  # Make source executable
  chmod +x "$source"

  # Copy to installation directory
  if ! cp "$source" "$target"; then
    log_error "Failed to install binary to $target"
    log_info "You may need to use: sudo cp $source $target"
    exit 1
  fi

  chmod +x "$target"
  log_success "Binary installed to $target"
}

# Check if install directory is in PATH
check_path() {
  local install_path="$1"

  if [[ ":$PATH:" == *":$install_path:"* ]]; then
    return 0
  fi

  return 1
}

# Add install directory to PATH (for common shells)
add_to_path() {
  local install_path="$1"
  local shell_name
  shell_name=$(basename "$SHELL")

  log_warning "Installation directory is not in PATH"
  log_info "Add the following to your shell configuration file:"
  echo ""
  echo "  export PATH=\"$install_path:\$PATH\""
  echo ""

  case "$shell_name" in
    bash)
      echo "Add to: ~/.bashrc or ~/.bash_profile"
      ;;
    zsh)
      echo "Add to: ~/.zshrc"
      ;;
    fish)
      echo "Add to: ~/.config/fish/config.fish (using: set -gx PATH $install_path \$PATH)"
      ;;
    *)
      echo "Add to: ~/.${shell_name}rc or equivalent"
      ;;
  esac
}

# Cleanup temporary files
cleanup() {
  rm -f /tmp/fspec-* /tmp/fspec.tar.gz /tmp/fspec.zip
}

# Main installation flow
main() {
  log_info "fspec Native Installer"
  echo ""

  # Detect preferred platform
  local current_platform
  current_platform=$(detect_platform)
  log_info "Detected platform: $current_platform"

  # Create temporary directory for downloads
  local temp_dir
  temp_dir=$(mktemp -d 2>/dev/null || mktemp -d -t 'fspec')
  trap "rm -rf $temp_dir; show_cursor" EXIT INT TERM

  # Fetch recent releases
  local all_releases
  all_releases=$(fetch_recent_releases)

  # Find the most recent release with assets for this platform
  local release_tag=""
  local selected_platform=""
  local candidate_platforms
  candidate_platforms=$(get_candidate_platforms)

  for candidate in $candidate_platforms; do
    local tag_file
    tag_file="$temp_dir/tag_$candidate"
    (find_latest_release_tag "$all_releases" "$candidate" > "$tag_file" 2>/dev/null) &
    local pid=$!
    show_spinner "$pid" "Checking for compatible binaries for $candidate..."
    wait "$pid" || true

    local candidate_tag
    candidate_tag=$(cat "$tag_file" 2>/dev/null || true)
    if [[ -n "$candidate_tag" ]]; then
      release_tag="$candidate_tag"
      selected_platform="$candidate"
      break
    fi
  done

  if [[ -z "$release_tag" || -z "$selected_platform" ]]; then
    log_error "No releases with compatible binaries found for platform: $current_platform"
    log_info "Available platforms for this host were: $candidate_platforms"
    log_info "You can install via npm instead: npm install -g @sengac/fspec"
    exit 1
  fi

  platform="$selected_platform"
  log_success "Found compatible version: $release_tag ($platform)"

  # Download binary
  local archive_file="$temp_dir/fspec-binary.tar.gz"
  local download_url
  download_url=$(get_download_url "$release_tag" "$platform")

  download_binary "$download_url" "$archive_file"
  log_success "Downloaded successfully"

  # Verify checksum
  verify_checksum "$archive_file" "$release_tag"

  # Extract binary
  local binary_path
  binary_path=$(extract_binary "$archive_file" "$platform")

  # Install binary
  local target_path="$INSTALL_DIR/$BIN_NAME"
  install_binary "$binary_path" "$target_path"

  # Check if in PATH
  if ! check_path "$INSTALL_DIR"; then
    add_to_path "$INSTALL_DIR"
  fi

  echo ""
  log_success "Installation complete!"
  log_info "fspec is ready to use"
  echo ""

  # Test installation
  if "$target_path" --version &>/dev/null; then
    log_success "Version check passed: $($target_path --version)"
  else
    log_warning "Could not verify installation, but binary appears to be installed"
  fi

  echo ""
  log_info "To get started:"
  log_info "  cd /path/to/your/project"
  log_info "  fspec"
}

# Show usage
show_usage() {
  cat <<'USAGE'
fspec Native Installer

Usage: ./install.sh [options]

Options:
  -d, --dir DIR      Installation directory (default: ~/.local/bin)
  -h, --help         Show this help message

Examples:
  ./install.sh                           # Install to ~/.local/bin
  ./install.sh --dir /usr/local/bin      # Install to /usr/local/bin (may need sudo)
  curl -fsSL ... | bash                  # Pipe from GitHub (see INSTALLING.md)

Environment variables:
  INSTALL_DIR        Set installation directory
USAGE
}

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    -d|--dir)
      INSTALL_DIR="$2"
      shift 2
      ;;
    -h|--help)
      show_usage
      exit 0
      ;;
    *)
      log_error "Unknown option: $1"
      show_usage
      exit 1
      ;;
  esac
done

main
