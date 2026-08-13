# Installation Guide

fspec is a pure-Rust binary. Choose the installation method that works best for you.

## Quick Install

### macOS & Linux (from source)

```bash
# From the repository:
./scripts/install.sh

# Install to a custom directory:
./scripts/install.sh --dir /usr/local/bin
```

Requires `cargo` (Rust) and `git` to be installed. The script builds from source
and installs the binary to `~/.local/bin` by default.

### Windows

Build from source on macOS/Linux using cross-compilation, then copy the binary:

```bash
# Cross-compile Windows binary from macOS/Linux:
./scripts/build-cross.sh

# The packaged binary is in dist/fspec-x86_64-pc-windows-msvc.zip
```

Or build natively on Windows with Rust installed:

```powershell
cd codelet
cargo build --profile release-slim -p codelet-fspec
copy target\release-slim\fspec.exe $env:USERPROFILE\.local\bin\
```

### Build from Source

```bash
./scripts/build.sh
```

## Installation Methods

| Method | Platforms | Command | Notes |
| --- | --- | --- | --- |
| **install.sh** | macOS, Linux | `./scripts/install.sh` | Builds from source, installs to `~/.local/bin` |
| **build.sh** | macOS, Linux | `./scripts/build.sh` | Builds only, no installation |
| **build-cross.sh** | macOS, Linux → Windows | `./scripts/build-cross.sh` | Cross-compiles Windows binaries |
| **cargo build** | All | `cargo build` | Manual build for contributors |

## Build Scripts

### `scripts/install.sh`

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

### `scripts/build.sh`

Builds `fspec` for the current platform.

```bash
# Build only:
./scripts/build.sh

# Build and package as tarball in dist/:
./scripts/build.sh --package
```

### `scripts/build-cross.sh`

Cross-compiles Windows and Linux binaries from macOS or Linux.
**No Docker required.**

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

**Auto-installs on first run:**
- `cargo-xwin` (Windows cross-compilation)
- `cargo-zigbuild` (Linux/macOS cross-compilation)
- `zig` (cross-linker for Linux targets)
- Full LLVM toolchain (via Homebrew on macOS, apt on Linux)
- `lld` linker (via Homebrew on macOS)
- `llvm-tools` rustup component

**Prerequisites:**
- Rust (cargo) installed
- Homebrew (macOS) or apt (Linux)

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

## Cross-Compilation Details

### Prerequisites (macOS)

The cross-compilation script auto-installs LLVM and lld, but you may want to
add them to your PATH permanently:

```bash
# Add to ~/.zshrc:
echo 'export PATH="/opt/homebrew/opt/llvm/bin:$PATH"' >> ~/.zshrc
echo 'export PATH="/opt/homebrew/opt/lld/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### First Build

The first Windows cross-compilation downloads ~300MB of MSVC headers/libraries
and takes ~20 minutes to compile. Subsequent builds are faster since the MSVC
sysroot is cached in `~/.cache/cargo-xwin/`.

### Output

```
dist/
├── fspec-x86_64-pc-windows-msvc.zip       (~65 MB)
├── fspec-x86_64-unknown-linux-gnu.tar.gz  (~58 MB)
└── fspec-aarch64-unknown-linux-gnu.tar.gz (~54 MB)
```

Each archive contains `fspec` (or `fspec.exe` for Windows) ready to run on the target platform.

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

Restart PowerShell or Command Prompt. If still not found, add the installation
directory to your PATH via System → Advanced system settings → Environment Variables.

### Cross-compilation fails with "can't find crate for core"

The Windows target may not be installed for the active toolchain. The
`build-cross.sh` script handles this automatically. To fix manually:

```bash
# Check which toolchain cargo is using:
rustup show active-toolchain

# Add the target to that specific toolchain:
rustup target add x86_64-pc-windows-msvc --toolchain <toolchain-name>
```

### Cross-compilation fails with "failed to find tool llvm-lib"

LLVM is keg-only on macOS and not in PATH by default. The `build-cross.sh`
script adds it automatically. To add it permanently:

```bash
echo 'export PATH="/opt/homebrew/opt/llvm/bin:$PATH"' >> ~/.zshrc
echo 'export PATH="/opt/homebrew/opt/lld/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Build fails with missing protoc

```bash
# macOS
brew install protobuf

# Ubuntu/Debian
sudo apt install -y protobuf-compiler

# Verify
protoc --version  # libprotoc 3.x or higher
```

### macOS Gatekeeper warning

If macOS blocks the binary with "cannot be opened because the developer cannot be verified":

```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine ~/.local/bin/fspec
```

### Still stuck?

- Open an issue: https://github.com/sengac/fspec/issues
- Check the [README](README.md) for the latest documentation
- See [BUILD.md](BUILD.md) for detailed build instructions

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
| `codelet\target\release-slim\fspec.exe` | Build from source |

## Additional Resources

- **[README.md](README.md)** — Project overview and quick start
- **[BUILD.md](BUILD.md)** — Detailed build instructions, profiles, and troubleshooting
- **[scripts/](scripts/)** — Build and installation scripts
