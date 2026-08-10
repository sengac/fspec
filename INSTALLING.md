# Installation Guide

fspec is a pure-Rust binary. Choose the installation method that works best for you.

## Quick Install

### macOS & Linux

```bash
curl -fsSL https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.sh | bash
```

No dependencies required — downloads a self-contained Rust binary (~150 MB).

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.ps1 | iex
```

### Build from Source

```bash
cd codelet
cargo build --profile release-slim -p codelet-fspec
cp target/release-slim/fspec ~/.local/bin/
```

## Installation Methods

| Method | Platforms | Command | Notes |
| --- | --- | --- | --- |
| **Native Installer** | macOS, Linux | `curl ... \| bash` (see above) | Recommended — standalone binary, no dependencies |
| **PowerShell Installer** | Windows | `irm ... \| iex` (see above) | Standalone binary for Windows |
| **Build from Source** | All | `cargo build` (see above) | For contributors and advanced users |

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

## Native Installer Details

### How It Works

The native installer:

1. Detects your OS and CPU architecture
2. Fetches the latest release from GitHub
3. Downloads the platform-specific binary
4. Verifies the SHA-256 checksum (if available)
5. Extracts and installs the binary to `~/.local/bin` (configurable)
6. Adds PATH guidance if the directory isn't already in your PATH

### Installer Options

#### macOS & Linux

```bash
# Install to default location (~/.local/bin)
curl -fsSL https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.sh | bash

# Install to a custom directory
curl -fsSL https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.sh | bash -s -- --dir /usr/local/bin

# Or set via environment variable
INSTALL_DIR=/opt/fspec curl -fsSL https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.sh | bash
```

#### Windows (PowerShell)

```powershell
# Install to default location (%USERPROFILE%\.local\bin)
irm https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.ps1 | iex

# Install to a custom directory
.\install.ps1 -InstallDir "C:\Program Files\fspec"
```

> **Note:** Installing to `C:\Program Files` may require running PowerShell as Administrator.

### Supported Platforms

| Platform | Architecture | Archive Format |
| --- | --- | --- |
| macOS | Apple Silicon (ARM64) | `.tar.gz` |
| macOS | Intel (x86_64) | `.tar.gz` |
| Linux | x86_64 | `.tar.gz` |
| Linux | ARM64 | `.tar.gz` |
| Windows | x86_64 | `.zip` |
| Windows | ARM64 | `.zip` |

## Building from Source

### Prerequisites

- **Rust** (via rustup)
- **Protocol Buffers compiler** (`protoc`) — required at build time

See [BUILD.md](BUILD.md) for complete build prerequisites.

### Build Steps

```bash
# Clone the repository
git clone https://github.com/sengac/fspec.git
cd fspec

# Build the standalone binary
cd codelet
cargo build --profile release-slim -p codelet-fspec

# Install locally
cp target/release-slim/fspec ~/.local/bin/
```

### Development Mode

```bash
# Build and run directly
cd codelet
cargo run -p codelet-fspec

# Run tests
cargo test -p codelet-fspec

# Check code
cargo clippy -p codelet-fspec
```

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

### Installation fails with network errors

```bash
# 1. Check internet connection
curl https://api.github.com

# 2. Check GitHub is accessible
curl -I https://github.com/sengac/fspec/releases

# 3. Check GitHub status
# Visit https://www.githubstatus.com/

# 4. Try again in a fresh terminal
```

### Permission denied

**macOS/Linux:**

```bash
# Option 1: Install to user directory (no sudo needed)
INSTALL_DIR=$HOME/.local/bin curl -fsSL https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.sh | bash

# Option 2: Fix permissions on /usr/local/bin
sudo chmod +x /usr/local/bin/fspec
```

**Windows:**

Run PowerShell as Administrator if installing to a system directory.

### macOS Gatekeeper warning

If macOS blocks the binary with "cannot be opened because the developer cannot be verified":

```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine ~/.local/bin/fspec
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

### Still stuck?

- Open an issue: https://github.com/sengac/fspec/issues
- Check the [README](README.md) for the latest documentation

## Uninstall

### Native Installer

```bash
# macOS/Linux — remove the binary
rm ~/.local/bin/fspec
# or wherever you installed it:
rm /usr/local/bin/fspec
```

```powershell
# Windows — remove the binary
Remove-Item "$env:USERPROFILE\.local\bin\fspec.exe"
```

## Installation Paths

### macOS & Linux

| Path | Description |
| --- | --- |
| `~/.local/bin/fspec` | Default (native installer) |
| `/usr/local/bin/fspec` | System-wide (requires sudo) |
| `codelet/target/release-slim/fspec` | Build from source |

### Windows

| Path | Description |
| --- | --- |
| `%USERPROFILE%\.local\bin\fspec.exe` | Default (PowerShell installer) |
| `C:\Program Files\fspec\fspec.exe` | System-wide (requires admin) |
| `codelet\target\release-slim\fspec.exe` | Build from source |

## Additional Resources

- **[README.md](README.md)** — Project overview and quick start
- **[BUILD.md](BUILD.md)** — Detailed build instructions for contributors
- **[GitHub Releases](https://github.com/sengac/fspec/releases)** — Download binaries manually
