# Building codelet-napi Multi-Platform Binaries

This document describes how to build the `codelet-napi` NAPI-RS binaries for all 6 supported platforms from a macOS machine.

## Supported Platforms

| Platform | Target | Output File |
|----------|--------|-------------|
| macOS ARM64 | `aarch64-apple-darwin` | `codelet-napi.darwin-arm64.node` |
| macOS x64 | `x86_64-apple-darwin` | `codelet-napi.darwin-x64.node` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | `codelet-napi.linux-arm64-gnu.node` |
| Linux x64 | `x86_64-unknown-linux-gnu` | `codelet-napi.linux-x64-gnu.node` |
| Windows ARM64 | `aarch64-pc-windows-msvc` | `codelet-napi.win32-arm64-msvc.node` |
| Windows x64 | `x86_64-pc-windows-msvc` | `codelet-napi.win32-x64-msvc.node` |

## Prerequisites

### Required Tools

1. **Rust via rustup** (not Homebrew)
   ```bash
   # If you have Homebrew Rust, remove it first
   brew uninstall rust

   # Install rustup
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"

   # Add x86_64 macOS target for cross-compilation
   rustup target add x86_64-apple-darwin
   ```

2. **Node.js 20+**
   ```bash
   node --version  # Should be v20.x or higher
   ```

3. **Docker Desktop** (for Linux and Windows builds)
   - Download from https://www.docker.com/products/docker-desktop/
   - Ensure Docker is running before building

## Quick Start

### Build All Platforms

From the project root:

```bash
npm run build:codelet-napi:all
```

Or from the `codelet/napi` directory:

```bash
npm run build:all
```

### Build Specific Platforms

```bash
# macOS only (no Docker required)
cd codelet/napi && npm run build:macos

# Linux and Windows only (Docker required)
cd codelet/napi && npm run build:docker
```

## Build Scripts

All build scripts are located in `codelet/napi/scripts/` and configured in `codelet/napi/package.json`.

| npm script | Description |
|------------|-------------|
| `npm run build` | Build for current platform only |
| `npm run build:all` | Build all 6 platforms |
| `npm run build:macos` | Build macOS ARM64 + x64 |
| `npm run build:docker` | Build Linux + Windows via Docker |

### Command Line Options

The build script supports several options:

```bash
./scripts/build-all-platforms.sh [OPTIONS]

Options:
  --macos-only      Only build macOS targets (no Docker)
  --linux-only      Only build Linux targets (Docker)
  --windows-only    Only build Windows targets (Docker)
  --docker-only     Only build Docker targets (Linux/Windows)
  --rebuild-docker  Force rebuild of Docker images
  --help            Show help message
```

## How It Works

### macOS Builds

macOS binaries are built natively using rustup's cross-compilation support:
- ARM64: Built directly on Apple Silicon
- x64: Cross-compiled using `rustup target add x86_64-apple-darwin`

### Linux Builds

Linux binaries are built inside Docker using a custom image (`Dockerfile.cross`):
- Uses `rust:latest` base image with Node.js 20
- Includes GCC cross-compilers for both ARM64 and x64
- Runs `napi build` inside the container

### Windows Builds

Windows binaries are built inside Docker using `cargo-xwin` (`Dockerfile.windows`):
- Uses `rust:latest` base image with `cargo-xwin` installed
- Downloads MSVC sysroot automatically on first build
- **Important**: Uses `--cross-compiler clang` flag to avoid compatibility issues with the `ring` crate

## Docker Images

Two Docker images are used for cross-compilation:

| Image Name | Dockerfile | Purpose |
|------------|------------|---------|
| `codelet-napi-linux` | `Dockerfile.cross` | Linux x64/ARM64 builds |
| `codelet-napi-windows` | `Dockerfile.windows` | Windows x64/ARM64 builds |

Images are built automatically on first use and cached. To force a rebuild:

```bash
# Rebuild Linux image
npm run build:all -- --rebuild-docker

# Or manually
docker build -t codelet-napi-linux -f Dockerfile.cross codelet/napi/
docker build -t codelet-napi-windows -f Dockerfile.windows codelet/napi/
```

## Troubleshooting

### "Docker is not running"

Start Docker Desktop before running the build.

### "rustup: command not found"

You have Homebrew Rust instead of rustup. Follow the prerequisites to switch.

### Windows build fails with `/imsvc` error

Ensure the build script uses `--cross-compiler clang`. This is already configured in the build script.

### MSVC sysroot download is slow

The first Windows build downloads ~300MB of MSVC headers/libraries. Subsequent builds use the cached version in `~/.cache/cargo-xwin/`.

### Build runs out of memory

The build sets `RUST_MIN_STACK=16777216` (16MB) to handle deep recursion. If builds still fail, increase Docker's memory allocation in Docker Desktop settings.

## Verifying Builds

After building, verify all binaries exist:

```bash
ls -la codelet/napi/*.node
```

Test the local bindings (macOS only):

```bash
cd codelet/napi
node -e "const { CodeletSession } = require('./index.js'); console.log('OK')"
```

## CI/CD

For automated builds, see `.github/workflows/build-codelet-napi.yml`. The CI workflow builds each platform on its native runner for maximum compatibility.
