# Building & Installing fspec

fspec is a pure-Rust binary. This document covers building from source, installing, cross-compilation, and troubleshooting.

---

## Quick Start

```bash
# Install (builds from source, installs to ~/.local/bin):
./scripts/install.sh

# Build only (no installation):
./scripts/build.sh

# Cross-compile Windows binary from macOS/Linux:
./scripts/build-cross.sh
```

---

## Installation Methods

| Method | Platforms | Command | Notes |
| --- | --- | --- | --- |
| **install.sh** | macOS, Linux | `./scripts/install.sh` | Builds from source, installs to `~/.local/bin` |
| **build.sh** | macOS, Linux | `./scripts/build.sh` | Builds only, no installation |
| **build-cross.sh** | macOS, Linux → Windows/Linux | `./scripts/build-cross.sh` | Cross-compiles Windows/Linux binaries |
| **cargo build** | All | `cargo build` | Manual build for contributors |

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

   `protoc` is required at **build time only** because the nanograph dependency uses Lance, which uses `prost-build` to compile `.proto` schema files during `cargo build`. The compiled fspec binary has **no runtime dependency** on protobuf — all generated code is baked in at compile time.

### Cross-Compilation Prerequisites (macOS)

For `build-cross.sh`, LLVM and lld must be available on the PATH. The script installs them automatically, but you may also add them permanently:

```bash
# Add to ~/.zshrc for future sessions:
echo 'export PATH="/opt/homebrew/opt/llvm/bin:$PATH"' >> ~/.zshrc
echo 'export PATH="/opt/homebrew/opt/lld/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

---

## Build Scripts

### `scripts/install.sh` — Build & Install

Builds and installs the Rust binary from source.

```bash
# Default install (~/.local/bin):
./scripts/install.sh

# Custom install directory:
./scripts/install.sh --dir /usr/local/bin

# Use release profile (with debug info):
./scripts/install.sh --profile release
```

**How it works:**
1. Checks for `cargo` and `git` prerequisites
2. Detects whether running from the repository or piped via `curl`
3. If piped without a local repo, clones the fspec repository
4. Builds the Rust binary from source using `cargo build`
5. Copies the binary to `~/.local/bin` (configurable)
6. Provides PATH guidance if the directory isn't already in your PATH

### `scripts/build.sh` — Native Build

Builds `fspec` for the current platform (macOS or Linux).

```bash
# Build only:
./scripts/build.sh

# Build and package as tarball in dist/:
./scripts/build.sh --package

# Use release profile (with debug info for profiling):
./scripts/build.sh --profile release
```

**Output:**
- Binary: `rust/target/release-slim/fspec` (~150 MB)
- Archive: `dist/fspec-<arch>-<platform>.tar.gz` (~50 MB)

### `scripts/build-cross.sh` — Cross-Compile Windows & Linux

Cross-compiles Windows and Linux binaries from macOS or Linux using `cargo-xwin` (Windows) and `cargo-zigbuild` (Linux/macOS). **No Docker required.**

```bash
# Build x86_64 Windows (default):
./scripts/build-cross.sh

# Build x86_64 Linux:
./scripts/build-cross.sh --target x86_64-unknown-linux-gnu

# Build ARM64 Linux:
./scripts/build-cross.sh --target aarch64-unknown-linux-gnu

# Build all supported targets:
./scripts/build-cross.sh --all
```

**Output:**
- Windows: `dist/fspec-x86_64-pc-windows-msvc.zip` (~65 MB)
- Linux x86_64: `dist/fspec-x86_64-unknown-linux-gnu.tar.gz` (~58 MB)
- Linux ARM64: `dist/fspec-aarch64-unknown-linux-gnu.tar.gz` (~54 MB)

**Auto-installed prerequisites:**
- `cargo-xwin` (Windows cross-compilation)
- `cargo-zigbuild` (Linux/macOS cross-compilation)
- `zig` (cross-linker for Linux targets)
- Full LLVM toolchain (via Homebrew on macOS, apt on Linux)
- `lld` linker (via Homebrew on macOS)
- `llvm-tools` rustup component

---

## Release Profiles

`rust/Cargo.toml` defines **two** optimised profiles. They share the same `opt-level`, `lto = "fat"` and `codegen-units = 1` — the only difference is debug-info retention.

| Profile          | `debug` | `strip`     | Typical size | Use when                                      |
|------------------|---------|-------------|--------------|-----------------------------------------------|
| `release`        | `1`     | `"none"`    | ~800 MB      | Local profiling builds (pprof-rs symbol resolution) |
| `release-slim`   | `false` | `"symbols"` | ~150 MB      | **Shipping the standalone `fspec` binary**    |

### Why Two Profiles?

The default `[profile.release]` block retains DWARF line tables (`debug = 1`) and the full symbol table (`strip = "none"`) with `split-debuginfo = "off"` so the DWARF stays *embedded* in the artifact. This is required for **pprof-rs sampling profiler** builds — the profiler needs line-table DWARF to resolve Rust symbols to file:line attribution inside the binary.

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

Result: identical code (same LTO, same opt-level) with the DWARF sections (`.debug_info`, `.debug_str`, `.debug_ranges`, `.debug_line`, `.debug_loc`, …) and the Rust symbol table removed. Verified on Linux aarch64:

```text
release       (debug=1, strip=none)     797 MB   → ~91 MB .text + ~677 MB DWARF
release-slim  (debug=0, strip=symbols)  149 MB   → ~91 MB .text only
```

### Don't Strip the Wrong Artifact

- ✅ **Do** use `--profile release-slim` for the standalone `fspec` ELF.
- ❌ **Do not** add `strip = "symbols"` or `debug = false` to `[profile.release]`. That would break pprof-rs symbol resolution for local profiling builds.
- ❌ **Do not** post-process `release` builds with `strip(1)` — use `--profile release-slim` instead.

---

## Building for Current Platform

```bash
cd rust

# Distribution build (recommended):
cargo build --profile release-slim -p codelet-fspec

# Profiling build (keeps DWARF for pprof):
cargo build --release -p codelet-fspec
```

### Verifying Builds

```bash
# Check binary exists and size
ls -lh target/release-slim/fspec

# Run version check
./target/release-slim/fspec --version

# Run help
./target/release-slim/fspec --help
```

---

## Cross-Compiling

### Using build-cross.sh (Recommended)

```bash
# Build x86_64 Windows (default):
./scripts/build-cross.sh

# Build x86_64 Linux:
./scripts/build-cross.sh --target x86_64-unknown-linux-gnu

# Build ARM64 Linux:
./scripts/build-cross.sh --target aarch64-unknown-linux-gnu

# Build all supported targets:
./scripts/build-cross.sh --all
```

The first Windows build downloads ~300 MB of MSVC headers/libraries and takes ~20 minutes to compile. Subsequent builds are faster since the MSVC sysroot is cached in `~/.cache/cargo-xwin/`.

### Manual Cross-Compilation

#### Windows (cargo-xwin)

```bash
# Install cargo-xwin (one-time):
cargo install --locked cargo-xwin

# Add LLVM to PATH (macOS):
export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
export PATH="/opt/homebrew/opt/lld/bin:$PATH"

# Add Windows target:
rustup target add x86_64-pc-windows-msvc

# Build:
cd rust
cargo xwin build --profile release-slim -p codelet-fspec \
  --target x86_64-pc-windows-msvc
```

#### Linux (cargo-zigbuild)

```bash
# Install cargo-zigbuild and zig (one-time):
cargo install --locked cargo-zigbuild
brew install zig  # macOS

# Add Linux target:
rustup target add x86_64-unknown-linux-gnu

# Build x86_64 Linux (requires AVX-512 flags for lance-linalg):
cd rust
TARGET_CC="zig cc" TARGET_CFLAGS="-mcpu=sapphirerapids -mavx512bw" \
  cargo zigbuild --profile release-slim -p codelet-fspec \
  --target x86_64-unknown-linux-gnu

# Build ARM64 Linux (no AVX-512 flags needed):
rustup target add aarch64-unknown-linux-gnu
TARGET_CC="zig cc" \
  cargo zigbuild --profile release-slim -p codelet-fspec \
  --target aarch64-unknown-linux-gnu
```

#### macOS

```bash
cd rust

# Apple Silicon
cargo build --profile release-slim -p codelet-fspec \
  --target aarch64-apple-darwin

# Intel
cargo build --profile release-slim -p codelet-fspec \
  --target x86_64-apple-darwin
```

Artifacts land at `rust/target/<triple>/release-slim/fspec[.exe]`.

---

## After Installation

### 1. Verify Installation

```bash
fspec --version
```

### 2. Set Your AI Provider API Key

```bash
# Anthropic (recommended)
export ANTHROPIC_API_KEY="sk-ant-..."

# Or any supported provider — see Supported Providers below
```

### 3. Start the Factory

```bash
cd /path/to/your/project
fspec
```

This opens the Kanban board — your factory floor with AI workstations ready to take jobs.

---

## Supported AI Providers

fspec works with any AI provider that supports tool calling. Set the corresponding environment variable:

| Provider | Environment Variable |
| --- | --- |
| **Anthropic** | `ANTHROPIC_API_KEY` or `CLAUDE_CODE_OAUTH_TOKEN` |
| **Google Gemini** | `GOOGLE_GENERATIVE_AI_API_KEY` |
| **OpenAI** | `OPENAI_API_KEY` |
| **Codex (ChatGPT)** | OAuth (automatic via `/provider`) |
| **xAI** | `XAI_API_KEY` |
| **DeepSeek** | `DEEPSEEK_API_KEY` |
| **Z.AI** | `ZAI_API_KEY` |
| **Mistral** | `MISTRAL_API_KEY` |
| **Groq** | `GROQ_API_KEY` |
| **OpenRouter** | `OPENROUTER_API_KEY` |
| **Together AI** | `TOGETHER_API_KEY` |
| **Azure OpenAI** | `AZURE_OPENAI_API_KEY` |

**OpenAI-compatible APIs** — Ollama, vLLM, LM Studio, and any server implementing the OpenAI API format work via the OpenAI provider with `OPENAI_API_KEY`.

Configure or switch providers interactively with `/provider` in any session.

---

## Testing

### ⚠️ NEVER run unscoped `cargo test` or `cargo test --workspace`

A plain `cargo test --workspace` compiled all 944 integration-test binaries with full DWARF debug info (1.4–2 GB PER BINARY — each one statically links the whole crate graph including arrow, datafusion, lance and tantivy). `target/debug/deps` grew to 299 GB and the machine crashed mid-link.

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

## Installation Paths

### macOS & Linux

| Path | Description |
| --- | --- |
| `~/.local/bin/fspec` | Default (install.sh) |
| `/usr/local/bin/fspec` | System-wide (requires sudo) |
| `rust/target/release-slim/fspec` | Build from source |

### Windows

| Path | Description |
| --- | --- |
| `%USERPROFILE%\.local\bin\fspec.exe` | Default |
| `C:\Program Files\fspec\fspec.exe` | System-wide (requires admin) |
| `rust\target\release-slim\fspec.exe` | Build from source |

---

## Uninstall

### macOS/Linux

```bash
# Remove the binary
rm ~/.local/bin/fspec
# or wherever you installed it:
rm /usr/local/bin/fspec
```

### Windows

```powershell
# Remove the binary
Remove-Item "$env:USERPROFILE\.local\bin\fspec.exe"
```

---

## Troubleshooting

### Command not found after installation

**macOS/Linux:**

```bash
# Check if ~/.local/bin is in your PATH
echo $PATH | tr ':' '\n' | grep local

# If not, add it to your shell configuration:
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc   # zsh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc  # bash

# Reload your shell
source ~/.zshrc   # or ~/.bashrc
```

**Windows:**

Restart PowerShell or Command Prompt. If still not found, add the installation directory to your PATH via System → Advanced system settings → Environment Variables.

### Build fails with missing protoc

```bash
# macOS
brew install protobuf

# Ubuntu/Debian
sudo apt install -y protobuf-compiler

# Verify
protoc --version  # libprotoc 3.x or higher
```

### "rustup: command not found"

You have Homebrew Rust instead of rustup. Follow the prerequisites section to switch.

### Build runs out of memory

The build sets `RUST_MIN_STACK=16777216` (16 MB) to handle deep recursion. If builds still fail, increase available memory.

### Cross-compilation fails with "can't find crate for core"

The Windows target may not be installed for the active toolchain. The `build-cross.sh` script handles this automatically. To fix manually:

```bash
# Check which toolchain cargo is using:
rustup show active-toolchain

# Add the target to that specific toolchain:
rustup target add x86_64-pc-windows-msvc --toolchain <toolchain-name>
```

### Cross-compilation fails with "failed to find tool llvm-lib"

LLVM is keg-only on macOS and not in PATH by default. The `build-cross.sh` script adds it automatically. To add it permanently:

```bash
echo 'export PATH="/opt/homebrew/opt/llvm/bin:$PATH"' >> ~/.zshrc
echo 'export PATH="/opt/homebrew/opt/lld/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### macOS Gatekeeper warning

If macOS blocks the binary with "cannot be opened because the developer cannot be verified":

```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine ~/.local/bin/fspec
```
