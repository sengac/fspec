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
  BUILD_JOBS         Max parallel rustc processes (default: 4, bounds peak memory)
  NO_IPV4_PROXY=1    Disable the local IPv4-only MSVC download proxy (default: on)
  ZIG_VERSION        Zig version for auto-install (default: 0.16.0)
  LLVM_VERSION       LLVM version for auto-install (default: 18.1.8)

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
  - No root/sudo required: the script auto-installs zig (official binary),
    the MSVC cross-compile toolchain (clang-cl/lld/llvm via Homebrew on macOS
    or a prebuilt LLVM bundle on Linux), and cargo-xwin/cargo-zigbuild.
  - Windows (cargo-xwin) builds route MSVC payload downloads through a local
    IPv4-only loopback proxy (auto-started) so they succeed on hosts whose
    IPv6 egress is broken. Set NO_IPV4_PROXY=1 to disable.
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
# Prefer the official prebuilt binary: no root, no pip, works on every POSIX
# host (pip3 fails on PEP 668 systems such as Ubuntu 24.04). Fallbacks:
# Homebrew, then pip (legacy path).
if ! command -v zig &>/dev/null; then
  ZIG_VERSION="${ZIG_VERSION:-0.16.0}"
  case "$(uname -m)" in
    x86_64|amd64)  zig_arch=x86_64 ;;
    aarch64|arm64) zig_arch=aarch64 ;;
    *) error "unsupported host arch for zig install: $(uname -m)"; exit 1 ;;
  esac
  case "$(uname -s)" in
    Darwin) zig_os=macos ;;
    Linux)  zig_os=linux ;;
    *) error "unsupported host OS for zig install: $(uname -s)"; exit 1 ;;
  esac
  zig_url="https://ziglang.org/download/${ZIG_VERSION}/zig-${zig_arch}-${zig_os}-${ZIG_VERSION}.tar.xz"
  zig_tmp="$(mktemp -d)"
  zig_dir="$HOME/.local/share/zig-${ZIG_VERSION}"
  info "zig not found — installing official binary ${ZIG_VERSION}..."
  if curl -fsSL "$zig_url" -o "$zig_tmp/zig.tar.xz"; then
    mkdir -p "$zig_dir"
    tar -xf "$zig_tmp/zig.tar.xz" -C "$zig_dir"
    mkdir -p "$HOME/.local/bin"
    ln -sf "$zig_dir/zig-${zig_arch}-${zig_os}-${ZIG_VERSION}/zig" "$HOME/.local/bin/zig"
    case ":$PATH:" in
      *":$HOME/.local/bin:"*) ;;
      *) export PATH="$HOME/.local/bin:$PATH" ;;
    esac
    rm -rf "$zig_tmp"
    success "zig ${ZIG_VERSION} installed (official binary)"
  else
    info "zig tarball download failed — falling back to Homebrew/pip..."
    rm -rf "$zig_tmp"
    if command -v brew &>/dev/null; then
      brew install zig
    else
      info "Homebrew not found — trying pip..."
      pip3 install ziglang
    fi
    success "zig installed"
  fi
fi

# ── Ensure the MSVC cross-compile toolchain (clang-cl + lld + llvm-lib) ──────
# cargo-xwin compiles C dependencies with the MSVC-mode clang driver
# (`clang-cl`), links with `lld-link`, and uses `llvm-lib` as the archiver.
# Distros ship `clang-cl` rarely, so gate on all three. Preference order:
#   1. Already on PATH (distro package, Homebrew, or a previous run of this
#      script which symlinks into $HOME/.local/bin)
#   2. macOS: Homebrew `llvm` + `lld` (keg-only; PATH is fixed below)
#   3. Linux: official prebuilt clang+llvm bundle (NO root/sudo required).
#      Last series with official aarch64 Linux prebuilts is 18.1.x.
ensure_msvc_toolchain() {
  if command -v clang-cl &>/dev/null && command -v lld-link &>/dev/null \
     && command -v llvm-lib &>/dev/null; then
    info "MSVC toolchain on PATH ($(command -v clang-cl))"
    return 0
  fi

  if [[ "$(uname)" == "Darwin" ]]; then
    if ! command -v brew &>/dev/null; then
      error "Homebrew is required. Install via https://brew.sh/"
      exit 1
    fi
    info "clang-cl/lld-link/llvm-lib not all found — installing via Homebrew..."
    brew install llvm lld
    success "LLVM + lld installed"
    return 0
  fi

  # Linux: prebuilt bundle, no root required.
  local llvm_ver="${LLVM_VERSION:-18.1.8}"
  local bundle
  case "$(uname -m)" in
    x86_64|amd64)  bundle="clang+llvm-${llvm_ver}-x86_64-linux-gnu-ubuntu-18.04" ;;
    aarch64|arm64) bundle="clang+llvm-${llvm_ver}-aarch64-linux-gnu" ;;
    *) error "unsupported host arch for LLVM install: $(uname -m)"; exit 1 ;;
  esac
  local dest="$HOME/.local/share/llvm-${llvm_ver}"
  local tmp
  tmp="$(mktemp -d)"
  info "clang-cl not found — installing prebuilt ${bundle} (no root needed)..."
  if curl -fsSL "https://github.com/llvm/llvm-project/releases/download/llvmorg-${llvm_ver}/${bundle}.tar.xz" \
       -o "$tmp/llvm.tar.xz"; then
    mkdir -p "$dest"
    tar -xf "$tmp/llvm.tar.xz" -C "$dest"
    rm -rf "$tmp"
    local bin_dir="$dest/$bundle/bin"
    # cargo-xwin + cc-rs look the tools up by name on PATH; expose the set.
    local t
    for t in clang clang++ clang-cl clang-cpp clang-linker-wrapper \
             lld lld-link ld.lld llvm-lib llvm-ar llvm-dlltool \
             llvm-nm llvm-objdump llvm-readelf llvm-addr2line llvm-profdata; do
      if [[ -x "$bin_dir/$t" ]]; then
        mkdir -p "$HOME/.local/bin"
        ln -sf "$bin_dir/$t" "$HOME/.local/bin/$t"
      fi
    done
    case ":$PATH:" in
      *":$HOME/.local/bin:"*) ;;
      *) export PATH="$HOME/.local/bin:$PATH" ;;
    esac
    success "prebuilt LLVM ${llvm_ver} installed (tools linked into ~/.local/bin)"
  else
    rm -rf "$tmp"
    error "LLVM download failed and no clang-cl on PATH."
    error "Install LLVM with clang-cl (e.g. 'sudo apt install clang-18 lld' then symlink) and retry."
    exit 1
  fi
}
ensure_msvc_toolchain

# ── Add Homebrew LLVM/lld to PATH (keg-only on macOS) ─────────────────────────
LLVM_BIN="$(dirname "$(which llvm-lib 2>/dev/null || echo '/opt/homebrew/opt/llvm/bin/llvm-lib')")"
if [[ -d "$LLVM_BIN" && ":$PATH:" != *":$LLVM_BIN:"* ]]; then
  export PATH="$LLVM_BIN:$PATH"
  info "Added LLVM to PATH: $LLVM_BIN"
fi

LLD_BIN="$(dirname "$(which lld-link 2>/dev/null || echo '/opt/homebrew/opt/lld/bin/lld-link')")"
if [[ -d "$LLD_BIN" && ":$PATH:" != *":$LLD_BIN:"* ]]; then
  export PATH="$LLD_BIN:$PATH"
  info "Added lld to PATH: $LLD_BIN"
fi

# ── Add llvm-tools rustup component ───────────────────────────────────────────
if ! rustup component list --installed 2>/dev/null | grep -q "llvm-tools"; then
  info "llvm-tools component not found — adding..."
  rustup component add llvm-tools
  success "llvm-tools installed"
fi

# ── IPv4-only loopback proxy for cargo-xwin's MSVC CRT downloads ─────────────
# Microsoft's MSVC payload CDN (download.visualstudio.microsoft.com) often
# answers DNS with an AAAA (IPv6) record that getaddrinfo() orders FIRST
# (RFC 8305). On hosts whose IPv6 egress is broken but IPv4 works, the
# Rust/ureq client connects to the IPv6 address, gets EHOSTUNREACH ("No route
# to host"), and — because std's TcpStream::connect only falls back to the next
# address on ECONNABORTED, not on EHOSTUNREACH — never tries the working IPv4
# address. The build then fails:
#
#   Error: Failed to setup MSVC CRT
#     Caused by: ... HTTP GET request for
#       https://download.visualstudio.microsoft.com/download/pr/... failed
#       io: No route to host (os error 113)
#
# Fix: run a tiny loopback CONNECT proxy that resolves targets with
# socket.getaddrinfo(..., AF_INET) only, and point cargo-xwin's ureq client at
# it via HTTPS_PROXY (ureq reads HTTPS_PROXY/https_proxy by default). This
# forces every MS download onto IPv4. It is safe: loopback-only, byte-for-byte
# transparent (TLS still happens end-to-end). No-op on hosts with working IPv6.
start_ipv4_proxy() {
  if [[ "${NO_IPV4_PROXY:-0}" == "1" ]]; then
    info "NO_IPV4_PROXY=1 — skipping the IPv4-only MSVC download proxy."
    return 0
  fi
  local helper="$REPO_ROOT/scripts/ipv4-proxy.py"
  if ! [[ -f "$helper" ]] || ! command -v python3 &>/dev/null; then
    warning "python3/ipv4-proxy.py unavailable — skipping IPv4 proxy (will rely on direct network)."
    return 0
  fi
  # Pick a free TCP port via a short-lived python bind.
  local port
  port="$(python3 - <<'PY'
import socket
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
  )" || { warning "could not allocate a proxy port — skipping."; return 0; }

  python3 "$helper" "$port" > /dev/null 2>&1 &
  PROXY_PID=$!
  # Wait up to ~3s for the proxy's READY line.
  local i
  for i in $(seq 1 30); do
    if ! kill -0 "$PROXY_PID" 2>/dev/null; then
      warning "IPv4 proxy exited during startup — skipping."
      PROXY_PID=""
      return 0
    fi
    sleep 0.1
  done
  export HTTPS_PROXY="http://127.0.0.1:${port}"
  export NO_PROXY=""
  info "IPv4-only MSVC download proxy running on 127.0.0.1:${port} (pid ${PROXY_PID})"
}

stop_ipv4_proxy() {
  if [[ -n "${PROXY_PID:-}" ]] && kill -0 "$PROXY_PID" 2>/dev/null; then
    kill "$PROXY_PID" 2>/dev/null || true
  fi
  PROXY_PID=""
}

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
  # The cargo-xwin (Windows) path downloads MSVC CRT/SDK payloads from
  # Microsoft's CDN. Route those through the local IPv4-only proxy so the
  # build succeeds on hosts whose IPv6 egress is broken (see start_ipv4_proxy).
  if [[ "$build_tool" == "cargo-xwin" ]]; then
    start_ipv4_proxy
  fi
  # Cap parallel rustc processes to bound peak memory. This mirrors the 2-4
  # vCPU ceiling of GitHub's runners (where this build succeeds without OOM).
  # On a 20-core machine the default -j 20 spawns 20 concurrent rustc
  # processes on heavy crates (lance, datafusion, arrow, tantivy) and OOMs.
  # Override with BUILD_JOBS for faster builds on machines with headroom.
  local jobs="${BUILD_JOBS:-4}"
  # Raise the FD limit instead of forcing codegen-units=1: the old
  # RUSTFLAGS="-C codegen-units=1" made every heavy dep crate compile as one
  # giant CGU, spiking rustc memory far above the profile's 16-CGU setting.
  ulimit -n 65535 2>/dev/null || true
  if [[ "$build_tool" == "cargo-xwin" ]]; then
    (
      cd "$CODELET_DIR"
      cargo xwin build --profile "$BUILD_PROFILE" --target "$target_triple" -p codelet-fspec -j "$jobs"
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
    # Parallelism is capped via -j "$jobs" (see above) to bound peak memory.
    # The release-slim profile keeps codegen-units=16 so heavy dep crates
    # never compile as a single giant CGU.
    local zig_cc="zig cc"
    local zig_cflags=""
    if [[ "$target_triple" == "x86_64"* ]]; then
      zig_cflags="-mcpu=sapphirerapids -mavx512bw"
    fi
    (
      cd "$CODELET_DIR"
      TARGET_CC="$zig_cc" TARGET_CFLAGS="$zig_cflags" TARGET_GLIBC_VERSION=2.17 \
        cargo zigbuild --profile "$BUILD_PROFILE" --target "$target_triple" -p codelet-fspec -j "$jobs"
    )
  fi
  # Tear down the IPv4-only proxy (if one was started for cargo-xwin).
  stop_ipv4_proxy

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
