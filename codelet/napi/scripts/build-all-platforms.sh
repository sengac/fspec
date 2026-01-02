#!/bin/bash
# Build all 6 platform binaries for codelet-napi
# Run this from the codelet/napi directory on macOS

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NAPI_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CODELET_DIR="$(cd "$NAPI_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Docker image names
LINUX_IMAGE="codelet-napi-linux"
WINDOWS_IMAGE="codelet-napi-windows"

# All targets
MACOS_TARGETS=(
    "aarch64-apple-darwin"
    "x86_64-apple-darwin"
)

LINUX_TARGETS=(
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
)

WINDOWS_TARGETS=(
    "x86_64-pc-windows-msvc"
    "aarch64-pc-windows-msvc"
)

# Parse arguments
BUILD_MACOS=true
BUILD_LINUX=true
BUILD_WINDOWS=true
REBUILD_DOCKER=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --macos-only)
            BUILD_LINUX=false
            BUILD_WINDOWS=false
            shift
            ;;
        --linux-only)
            BUILD_MACOS=false
            BUILD_WINDOWS=false
            shift
            ;;
        --windows-only)
            BUILD_MACOS=false
            BUILD_LINUX=false
            shift
            ;;
        --docker-only)
            BUILD_MACOS=false
            shift
            ;;
        --rebuild-docker)
            REBUILD_DOCKER=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --macos-only      Only build macOS targets (no Docker)"
            echo "  --linux-only      Only build Linux targets (Docker)"
            echo "  --windows-only    Only build Windows targets (Docker)"
            echo "  --docker-only     Only build Docker targets (Linux/Windows)"
            echo "  --rebuild-docker  Force rebuild of Linux Docker image"
            echo "  --help            Show this help message"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

cd "$NAPI_DIR"

# Ensure we're in the right directory
if [[ ! -f "package.json" ]] || [[ ! -f "Cargo.toml" ]]; then
    log_error "Must run from codelet/napi directory"
    exit 1
fi

# Check for Docker if building Docker targets
if $BUILD_LINUX || $BUILD_WINDOWS; then
    if ! command -v docker &> /dev/null; then
        log_error "Docker is required for cross-compilation. Install Docker Desktop or use --macos-only"
        exit 1
    fi

    # Check if Docker is running
    if ! docker info &> /dev/null; then
        log_error "Docker is not running. Start Docker Desktop and try again."
        exit 1
    fi
fi

# ============================================================================
# macOS Builds (Native)
# ============================================================================

if $BUILD_MACOS; then
    log_info "Building macOS targets natively..."

    # Ensure x86_64 target is installed
    rustup target add x86_64-apple-darwin 2>/dev/null || true

    for target in "${MACOS_TARGETS[@]}"; do
        log_info "Building $target..."
        npm run build -- --target "$target"
        log_success "Built $target"
    done
fi

# ============================================================================
# Linux Builds (Docker)
# ============================================================================

if $BUILD_LINUX; then
    log_info "Building Linux targets via Docker..."

    # Build or check Docker image
    if $REBUILD_DOCKER || ! docker image inspect "$LINUX_IMAGE" &> /dev/null; then
        log_info "Building Linux Docker image (this may take a few minutes on first run)..."
        docker build -t "$LINUX_IMAGE" -f Dockerfile.cross "$NAPI_DIR"
        log_success "Linux Docker image built"
    else
        log_info "Using existing Linux Docker image. Use --rebuild-docker to rebuild."
    fi

    for target in "${LINUX_TARGETS[@]}"; do
        log_info "Building $target via Docker..."

        # Run build in Docker using napi build
        docker run --rm \
            -v "$CODELET_DIR":/build \
            -w /build/napi \
            -e RUST_MIN_STACK=16777216 \
            "$LINUX_IMAGE" \
            bash -c "npm ci --ignore-scripts && npm run build -- --target $target"

        # Determine expected output file
        case "$target" in
            "x86_64-unknown-linux-gnu")
                NODE_FILE="codelet-napi.linux-x64-gnu.node"
                ;;
            "aarch64-unknown-linux-gnu")
                NODE_FILE="codelet-napi.linux-arm64-gnu.node"
                ;;
        esac

        if [[ -f "$NAPI_DIR/$NODE_FILE" ]]; then
            log_success "Built $target -> $NODE_FILE"
        else
            log_warn "Could not find $NODE_FILE - check Docker build output"
        fi
    done
fi

# ============================================================================
# Windows Builds (Docker with cargo-xwin)
# ============================================================================

if $BUILD_WINDOWS; then
    log_info "Building Windows targets via Docker..."

    # Build Windows Docker image if needed
    if ! docker image inspect "$WINDOWS_IMAGE" &> /dev/null; then
        log_info "Building Windows Docker image..."
        docker build -t "$WINDOWS_IMAGE" -f Dockerfile.windows "$NAPI_DIR"
        log_success "Windows Docker image built"
    else
        log_info "Using existing Windows Docker image."
    fi

    for target in "${WINDOWS_TARGETS[@]}"; do
        log_info "Building $target via Docker..."

        # Run build using cargo xwin directly
        # Note: We build with cargo xwin, not napi build, to use the xwin toolchain
        # Use --cross-compiler clang to avoid /imsvc flag issues with ring crate
        docker run --rm \
            -v "$CODELET_DIR":/io \
            -w /io/napi \
            -e RUST_MIN_STACK=16777216 \
            "$WINDOWS_IMAGE" \
            cargo xwin build --release --target "$target" --cross-compiler clang -p codelet-napi

        # Determine expected output file and copy it
        case "$target" in
            "x86_64-pc-windows-msvc")
                NODE_FILE="codelet-napi.win32-x64-msvc.node"
                BUILT_FILE="$CODELET_DIR/target/$target/release/codelet_napi.dll"
                ;;
            "aarch64-pc-windows-msvc")
                NODE_FILE="codelet-napi.win32-arm64-msvc.node"
                BUILT_FILE="$CODELET_DIR/target/$target/release/codelet_napi.dll"
                ;;
        esac

        if [[ -f "$BUILT_FILE" ]]; then
            cp "$BUILT_FILE" "$NAPI_DIR/$NODE_FILE"
            log_success "Built $target -> $NODE_FILE"
        else
            log_warn "Could not find built artifact at $BUILT_FILE"
            log_warn "Checking for alternative locations..."
            find "$CODELET_DIR/target/$target" -name "*.dll" -o -name "*.node" 2>/dev/null || true
        fi
    done
fi

# ============================================================================
# Summary
# ============================================================================

echo ""
log_info "Build complete! Checking artifacts..."
echo ""

cd "$NAPI_DIR"
for f in *.node; do
    if [[ -f "$f" ]]; then
        SIZE=$(ls -lh "$f" | awk '{print $5}')
        log_success "$f ($SIZE)"
    fi
done

echo ""
log_info "To test locally (macOS only), run:"
echo "  node -e \"const { CodeletSession } = require('./index.js'); console.log('OK')\""
