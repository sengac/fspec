# Building codelet Binaries

This document describes how to build the two distributable Rust artifacts in this repository:

1. **`codelet-napi`** — NAPI-RS bindings consumed by the TypeScript `fspec` CLI ([Building codelet-napi Multi-Platform Binaries](#building-codelet-napi-multi-platform-binaries)).
2. **`fspec` (standalone)** — pure-Rust port of the `fspec` CLI shipped as a single ELF / Mach-O / PE binary ([Building the standalone `fspec` Rust binary](#building-the-standalone-fspec-rust-binary)).

---

## Building codelet-napi Multi-Platform Binaries

This section describes how to build the `codelet-napi` NAPI-RS binaries for all 6 supported platforms from a macOS machine.

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

3. **Protocol Buffers compiler (`protoc`)** — build-time only
   ```bash
   # macOS
   brew install protobuf

   # Ubuntu/Debian
   sudo apt install -y protobuf-compiler

   # Verify
   protoc --version  # libprotoc 3.x or higher
   ```

   `protoc` is required at **build time only** because the nanograph dependency
   uses Lance, which uses `prost-build` to compile `.proto` schema files during
   `cargo build`. The compiled fspec binary has **no runtime dependency** on
   protobuf — all generated code is baked in at compile time.

4. **Docker Desktop** (for Linux and Windows builds)
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
node -e "const { BackgroundSession } = require('./index.js'); console.log('OK')"
```

---

## Building the standalone `fspec` Rust binary

The pure-Rust port of the `fspec` CLI lives in `codelet/fspec` and compiles
to a single statically-linked binary (`fspec`). It is independent of the
NAPI artifacts above — no Node.js runtime, no `.node` files, no SEA.

### TL;DR

```bash
# Distribution build (recommended for shipping / install scripts):
cd codelet
cargo build --profile release-slim -p codelet-fspec
ls -lh target/release-slim/fspec       # ≈ 150 MB

# Local profiling build (keeps DWARF line tables for pprof-rs):
cargo build --release -p codelet-fspec
ls -lh target/release/fspec            # ≈ 800 MB
```

### Why two release profiles?

`codelet/Cargo.toml` defines **two** optimised profiles. They share the
same `opt-level`, `lto = "fat"` and `codegen-units = 1` — the only
difference is debug-info retention.

| Profile          | `debug` | `strip`     | Typical size | Use when                                      |
|------------------|---------|-------------|--------------|-----------------------------------------------|
| `release`        | `1`     | `"none"`    | ~800 MB      | Building `codelet-napi.node` for SEA + pprof  |
| `release-slim`   | `false` | `"symbols"` | ~150 MB      | Shipping the standalone `fspec` binary        |

#### The `[profile.release]` constraint

The default `[profile.release]` block deliberately retains DWARF line
tables (`debug = 1`) and the full symbol table (`strip = "none"`) with
`split-debuginfo = "off"` so the DWARF stays *embedded* in the artifact.
This exists for one reason: **the `codelet-napi` Node SEA pprof
contract**.

At launch, the Node Single-Executable-Application extracts the embedded
`codelet-napi.node` into a random temp directory
(`/var/folders/.../T/fspec-sea-<pid>/codelet-napi.node`). macOS's
path-convention `.dSYM` lookup then searches for a sibling
`codelet-napi.node.dSYM` bundle in that temp dir — which never exists
— so without embedded DWARF, every `pprof` sample resolves to a raw
address and `AgentManager.profile` returns phantom hot-spots like
`_napi_register_module_v1` instead of real Rust function attribution.

The standalone `fspec` binary has **none** of those constraints:

- It is not packed into a SEA — DWARF doesn't need to ride with the file.
- It is not the pprof sampling target — `codelet-napi` is.
- It is shipped to end users — every megabyte of DWARF is dead weight.

#### What `release-slim` does

```toml
[profile.release-slim]
inherits = "release"
strip = "symbols"      # drop .debug_*, .symtab, .strtab
debug = false          # don't emit DWARF in the first place
split-debuginfo = "off"
```

Result: identical code (same LTO, same opt-level) with the DWARF
sections (`.debug_info`, `.debug_str`, `.debug_ranges`, `.debug_line`,
`.debug_loc`, …) and the Rust symbol table removed. Verified on Linux
aarch64:

```text
release       (debug=1, strip=none)     797 MB   → ~91 MB .text + ~677 MB DWARF
release-slim  (debug=0, strip=symbols)  149 MB   → ~91 MB .text only
```

### Cross-compiling the standalone binary

The standalone `fspec` binary has the same toolchain requirements as
`codelet-napi` (Rust ≥ 1.80, `protoc`). Cross-compile by passing a
`--target` triple:

```bash
# Linux aarch64
cargo build --profile release-slim -p codelet-fspec \
  --target aarch64-unknown-linux-gnu

# macOS universal (run on Apple Silicon)
cargo build --profile release-slim -p codelet-fspec \
  --target aarch64-apple-darwin
cargo build --profile release-slim -p codelet-fspec \
  --target x86_64-apple-darwin

# Windows x64 (via cargo-xwin in Docker — see Dockerfile.windows)
cargo xwin build --profile release-slim -p codelet-fspec \
  --target x86_64-pc-windows-msvc --cross-compiler clang
```

Artifacts land at `codelet/target/<triple>/release-slim/fspec[.exe]`.

### Don't strip the wrong artifact

- ✅ **Do** use `--profile release-slim` for the standalone `fspec` ELF.
- ❌ **Do not** add `strip = "symbols"` or `debug = false` to
  `[profile.release]`. That would silently break pprof attribution
  inside the `codelet-napi.node` shipped through the Node SEA on macOS.
- ❌ **Do not** post-process the SEA-bound `.node` file with `strip(1)`
  for the same reason.

If you only need a slim Linux binary right now without rebuilding,
`strip -s codelet/target/release/fspec` produces the same ~150 MB
output, but `--profile release-slim` is the supported path.
