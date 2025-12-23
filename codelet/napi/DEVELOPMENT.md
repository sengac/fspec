# codelet-napi Development Guide

This document describes the development, release, and publish workflow for `@sengac/codelet-napi`, the NAPI-RS native module that provides Rust AI agent bindings for Node.js.

## Architecture Overview

```
fspec monorepo
├── package.json                 # workspaces: ["codelet/napi"]
│                                # dependencies: "@sengac/codelet-napi": "^0.1.0"
├── codelet/
│   ├── Cargo.toml               # Rust workspace
│   ├── patches/rig-core/        # Patched rig-core dependency
│   └── napi/
│       ├── package.json         # @sengac/codelet-napi
│       ├── src/lib.rs           # Rust NAPI bindings
│       ├── index.js             # ESM wrapper (auto-generated)
│       ├── index.d.ts           # TypeScript types (auto-generated)
│       └── *.node               # 6 platform binaries (built by CI)
```

## npm Workspaces

The project uses npm workspaces to allow the same package reference to work for both development and production:

```json
// Root package.json
{
  "workspaces": ["codelet/napi"],
  "dependencies": {
    "@sengac/codelet-napi": "^0.1.0"
  }
}
```

- **During development**: npm resolves `@sengac/codelet-napi` to `codelet/napi/` via workspace symlink
- **For end users**: npm resolves `@sengac/codelet-napi` from the npm registry

## Platform Targets

The module builds for 6 platform targets:

| Platform | CI Runner | Binary File |
|----------|-----------|-------------|
| macOS ARM64 (M1/M2) | `macos-14` | `codelet-napi.darwin-arm64.node` |
| macOS Intel | `macos-15-intel` | `codelet-napi.darwin-x64.node` |
| Linux x64 | `ubuntu-latest` | `codelet-napi.linux-x64-gnu.node` |
| Linux ARM64 | `ubuntu-24.04-arm` | `codelet-napi.linux-arm64-gnu.node` |
| Windows x64 | `windows-latest` | `codelet-napi.win32-x64-msvc.node` |
| Windows ARM64 | `windows-latest` (cross-compile) | `codelet-napi.win32-arm64-msvc.node` |

## Development Workflow

### Local Development

```bash
# 1. Make changes to Rust code
vim codelet/napi/src/lib.rs

# 2. Build locally (creates binary for your platform only)
cd codelet/napi && npm run build

# 3. Build fspec with local codelet-napi
npm run build

# 4. Run tests
npm test
```

### Build Commands

```bash
# Release build (optimized)
npm run build

# Debug build (faster compilation, includes debug symbols)
npm run build:debug
```

## Release / Publish Workflow

### Overview

```
Development → /release-codelet-napi → git push → CI builds → git pull → /publish-codelet-napi
```

### Step 1: Release (`/release-codelet-napi`)

The release command:
1. Analyzes commits since last tag to determine version bump
2. Updates `codelet/napi/package.json` version
3. Creates a release commit
4. Creates a git tag (`codelet-napi-v{version}`)

```bash
# Run the release command
/release-codelet-napi

# Output example:
# Analyzing commits since codelet-napi-v0.1.0...
#   - feat: add new persistence API
#   - fix: memory leak in session handling
# Determined version bump: minor (0.1.0 → 0.2.0)
# ✓ Created tag: codelet-napi-v0.2.0
```

### Step 2: Push to Trigger CI

```bash
git push && git push origin codelet-napi-v0.2.0
```

This triggers the GitHub Actions workflow which:
1. **Build job**: Builds native binaries on 6 platforms in parallel
2. **Test job**: Runs smoke tests on 5 platforms (Windows ARM64 excluded)
3. **Commit-binaries job**: Commits all 6 `.node` files back to the repo

Monitor progress: https://github.com/sengac/fspec/actions

### Step 3: Pull Binaries

```bash
# Wait for CI to complete (~10 minutes), then:
git pull
```

### Step 4: Publish (`/publish-codelet-napi`)

The publish command:
1. Verifies npm authentication
2. Checks all 6 binaries exist
3. Compares local version with npm registry
4. Publishes to npm if version is new

```bash
# Run the publish command
/publish-codelet-napi

# Output example:
# Checking npm authentication...
#   Logged in as: sengac
# Checking binaries...
#   ✓ All 6 platform binaries present
# Publishing @sengac/codelet-napi@0.2.0...
#   ✓ Published successfully!
```

## CI/CD Pipeline

The workflow is defined in `.github/workflows/build-codelet-napi.yml`:

```yaml
# Triggers
on:
  push:
    branches: [main, codelet-integration]
    paths: ['codelet/**']
  pull_request:
    paths: ['codelet/**']
  workflow_dispatch:

# Jobs
jobs:
  build:      # 6 parallel platform builds
  test:       # 5 parallel smoke tests
  commit-binaries:  # Commits .node files to repo
```

### Why Binaries Are Committed to Repo

Instead of publishing platform packages to npm from CI, binaries are committed directly to the repo because:

1. **Simpler npm publishing**: Single package with all binaries included
2. **No npm token in CI**: Publishing happens from local machine with user auth
3. **Easier debugging**: Binaries are visible in the repo
4. **Version control**: Binary changes are tracked in git history

## File Reference

| File | Purpose |
|------|---------|
| `.github/workflows/build-codelet-napi.yml` | CI workflow |
| `.claude/commands/release-codelet-napi.md` | Release command spec |
| `.claude/commands/publish-codelet-napi.md` | Publish command spec |
| `codelet/napi/package.json` | npm package config |
| `codelet/napi/src/lib.rs` | Main Rust NAPI bindings |
| `codelet/Cargo.toml` | Rust workspace config |
| `vite.config.ts` | Externalizes `@sengac/codelet-napi` |

## Troubleshooting

### Build Fails Locally

```bash
# Ensure Rust is installed
rustup --version

# Ensure correct target is installed
rustup target add aarch64-apple-darwin  # or your platform

# Clean and rebuild
cd codelet && cargo clean
cd napi && npm run build
```

### CI Build Fails

1. Check GitHub Actions logs: https://github.com/sengac/fspec/actions
2. Common issues:
   - Rust compilation errors in `codelet/napi/src/`
   - Missing dependencies in `codelet/Cargo.toml`
   - Platform-specific code issues (check `#[cfg(unix)]` / `#[cfg(windows)]`)

### Publish Fails

```bash
# Not logged in
npm login

# Version already published
# The command will skip if version matches npm registry

# Missing binaries
# Wait for CI to complete and run: git pull
```

## Version History

- `0.1.0` - Initial release with CodeletSession, persistence APIs, multi-provider support
