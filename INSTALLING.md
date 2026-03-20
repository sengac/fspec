# Installation Guide

fspec supports multiple installation methods. Choose the one that works best for you.

## Quick Install

### macOS & Linux (Native Installer — Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.sh | bash
```

No dependencies required — downloads a self-contained binary with Node.js embedded.

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.ps1 | iex
```

### npm (All Platforms)

```bash
npm install -g @sengac/fspec
```

> **Note:** The npm package requires Node.js >= 18.0.0 and includes native binaries for supported platforms. The native installer downloads a standalone SEA (Single Executable Application) binary with no runtime dependencies.

## Installation Methods

| Method | Platforms | Command | Notes |
| --- | --- | --- | --- |
| **Native Installer** | macOS, Linux | `curl ... \| bash` (see above) | Recommended — standalone binary, no dependencies |
| **PowerShell Installer** | Windows | `irm ... \| iex` (see above) | Standalone binary for Windows |
| **npm** | All | `npm install -g @sengac/fspec` | Requires Node.js >= 18 |
| **Build from Source** | All | See [Building from Source](#building-from-source) | For contributors and advanced users |

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
3. Downloads the platform-specific SEA binary archive
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

For contributors or users who want to build from source:

### Prerequisites

- **Node.js >= 25.5.0** (for SEA binary building) or **>= 18.0.0** (for npm usage)
- **Rust** (via rustup) — required for the NAPI-RS native addon
- **Protocol Buffers compiler** (`protoc`) — required at build time
- See [BUILD.md](BUILD.md) for complete build prerequisites

### Build Steps

```bash
# Clone the repository
git clone https://github.com/sengac/fspec.git
cd fspec

# Install dependencies
npm install

# Build the Vite bundle + NAPI-RS native addon
npm run build

# Option A: Run directly via Node.js
node dist/index.js --version

# Option B: Build a standalone SEA binary (requires Node.js >= 25.5.0)
npm run build:sea

# Install the SEA binary locally
npm run install:local
# or manually:
cp dist/sea/fspec /usr/local/bin/fspec
```

### Development Mode

```bash
# Watch mode — rebuilds on file changes
npm run dev

# Run tests
npm test

# Format code
npm run format
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

### npm install fails with native addon errors

The npm package includes pre-built native binaries. If the binary for your platform isn't available:

```bash
# Try installing with build from source
npm install -g @sengac/fspec --build-from-source

# Or use the native installer instead
curl -fsSL https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.sh | bash
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

### npm

```bash
npm uninstall -g @sengac/fspec
```

## Installation Paths

### macOS & Linux

| Path | Description |
| --- | --- |
| `~/.local/bin/fspec` | Default (native installer) |
| `/usr/local/bin/fspec` | System-wide (requires sudo) |
| `$(npm prefix -g)/bin/fspec` | npm global install |

### Windows

| Path | Description |
| --- | --- |
| `%USERPROFILE%\.local\bin\fspec.exe` | Default (PowerShell installer) |
| `C:\Program Files\fspec\fspec.exe` | System-wide (requires admin) |
| `%APPDATA%\npm\fspec.cmd` | npm global install |

## Additional Resources

- **[README.md](README.md)** — Project overview and quick start
- **[BUILD.md](BUILD.md)** — Detailed build instructions for contributors
- **[GitHub Releases](https://github.com/sengac/fspec/releases)** — Download binaries manually
