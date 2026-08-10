# Building the fspec Binary

This document describes how to build the `fspec` standalone Rust binary.

---

## Quick Start

```bash
cd codelet
cargo build --profile release-slim -p codelet-fspec
ls -lh target/release-slim/fspec   # ≈ 150 MB
```

---

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

2. **Protocol Buffers compiler (`protoc`)** — build-time only
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

3. **Docker Desktop** (for Linux and Windows cross-compilation)
   - Download from https://www.docker.com/products/docker-desktop/
   - Ensure Docker is running before building

---

## Release Profiles

`codelet/Cargo.toml` defines **two** optimised profiles. They share the
same `opt-level`, `lto = "fat"` and `codegen-units = 1` — the only
difference is debug-info retention.

| Profile          | `debug` | `strip`     | Typical size | Use when                                      |
|------------------|---------|-------------|--------------|-----------------------------------------------|
| `release`        | `1`     | `"none"`    | ~800 MB      | Local profiling builds (pprof-rs symbol resolution) |
| `release-slim`   | `false` | `"symbols"` | ~150 MB      | **Shipping the standalone `fspec` binary**    |

### Why Two Profiles?

The default `[profile.release]` block retains DWARF line tables (`debug = 1`)
and the full symbol table (`strip = "none"`) with `split-debuginfo = "off"` so
the DWARF stays *embedded* in the artifact. This is required for **pprof-rs
sampling profiler** builds — the profiler needs line-table DWARF to resolve
Rust symbols to file:line attribution inside the binary.

The standalone `fspec` distribution binary does not need this:

- It is shipped to end users — every megabyte of DWARF is dead weight.
- Profiling is a developer-only concern, handled by the `release` profile.
- Distribution only needs optimised code, not debug symbols.

### What `release-slim` Does

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

---

## Building for Current Platform

```bash
cd codelet

# Distribution build (recommended):
cargo build --profile release-slim -p codelet-fspec

# Profiling build (keeps DWARF for pprof):
cargo build --release -p codelet-fspec
```

---

## Cross-Compiling

### Linux aarch64

```bash
cargo build --profile release-slim -p codelet-fspec \
  --target aarch64-unknown-linux-gnu
```

### macOS Universal

```bash
# Apple Silicon
cargo build --profile release-slim -p codelet-fspec \
  --target aarch64-apple-darwin

# Intel
cargo build --profile release-slim -p codelet-fspec \
  --target x86_64-apple-darwin
```

### Windows x64 (via cargo-xwin in Docker)

```bash
cargo xwin build --profile release-slim -p codelet-fspec \
  --target x86_64-pc-windows-msvc --cross-compiler clang
```

Artifacts land at `codelet/target/<triple>/release-slim/fspec[.exe]`.

---

## Testing

### ⚠️ NEVER run unscoped `cargo test` or `cargo test --workspace`

A plain `cargo test --workspace` compiled all 944 integration-test binaries
with full DWARF debug info (1.4–2 GB PER BINARY — each one statically links
the whole crate graph including arrow, datafusion, lance and tantivy).
`target/debug/deps` grew to 299 GB and the machine crashed mid-link.

**Safe invocation patterns:**

```bash
# 1. Scope by package + target (preferred):
cargo test -p codelet-fspec --test no_napi_dependency

# 2. Broader runs: list packages explicitly, drop debug info, bound link parallelism:
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  cargo test -p codelet-fspec -p codelet-fspec-core -j 12 --no-fail-fast

# 3. Use the dedicated profile:
cargo test --profile ci-test -p codelet-fspec
```

---

## Troubleshooting

### "protoc: command not found"

Install Protocol Buffers compiler (see Prerequisites above).

### "rustup: command not found"

You have Homebrew Rust instead of rustup. Follow the prerequisites to switch.

### Build runs out of memory

The build sets `RUST_MIN_STACK=16777216` (16MB) to handle deep recursion. If builds still fail, increase available memory.

### "Docker is not running" (for cross-compilation)

Start Docker Desktop before running cross-compilation builds.

### MSVC sysroot download is slow (Windows cross-compilation)

The first Windows build downloads ~300MB of MSVC headers/libraries. Subsequent builds use the cached version in `~/.cache/cargo-xwin/`.

---

## Verifying Builds

After building, verify the binary:

```bash
# Check binary exists and size
ls -lh target/release-slim/fspec

# Run version check
./target/release-slim/fspec --version

# Run help
./target/release-slim/fspec --help
```

---

## Don't Strip the Wrong Artifact

- ✅ **Do** use `--profile release-slim` for the standalone `fspec` ELF.
- ❌ **Do not** add `strip = "symbols"` or `debug = false` to
  `[profile.release]`. That would break pprof-rs symbol resolution
  for local profiling builds.
- ❌ **Do not** post-process `release` builds with `strip(1)` — use
  `--profile release-slim` instead.

If you only need a slim Linux binary right now without rebuilding,
`strip -s codelet/target/release/fspec` produces the same ~150 MB
output, but `--profile release-slim` is the supported path.
