# NAPI-007: Existing Setup Analysis

## Current State

### codelet-napi Module Structure

**Location**: `codelet/napi/`

**Key Files**:
- `Cargo.toml` - Rust crate configuration with NAPI-RS dependencies
- `package.json` - npm package configuration with 6 target platforms
- `src/lib.rs` - Module root exposing CodeletSession
- `src/session.rs` - Main class with prompt(), interrupt(), compact(), etc.
- `src/types.rs` - TypeScript interface definitions
- `src/output.rs` - Stream handling via ThreadsafeFunction
- `src/persistence/` - Session persistence (history, blobs)

**Current Build Output** (macOS ARM only):
- `codelet-napi.darwin-arm64.node` (49MB)
- `index.js` (26KB) - JavaScript wrapper
- `index.d.ts` (11KB) - TypeScript types

### Package Configuration

**codelet/napi/package.json**:
```json
{
  "name": "codelet-napi",
  "version": "0.1.0",
  "napi": {
    "binaryName": "codelet-napi",
    "targets": [
      "aarch64-apple-darwin",
      "x86_64-apple-darwin",
      "aarch64-unknown-linux-gnu",
      "x86_64-unknown-linux-gnu",
      "aarch64-pc-windows-msvc",
      "x86_64-pc-windows-msvc"
    ]
  },
  "private": true
}
```

### Cargo Workspace

**codelet/Cargo.toml** defines workspace with 7 crates:
- cli, common, core, napi, providers, tools, tui

**Patched Dependency**:
```toml
[patch.crates-io]
rig-core = { path = "patches/rig-core" }
```

### fspec Integration

**package.json** dependency (currently local):
```json
"codelet-napi": "file:codelet/napi"
```

**vite.config.ts** externalizes:
```javascript
external: ['codelet-napi', ...]
```

## Required Changes

### 1. GitHub Actions Workflow

Create `.github/workflows/build-codelet-napi.yml`:

**Triggers**:
- `push` to paths: `codelet/**`
- `pull_request` to paths: `codelet/**`
- `push` tags: `codelet-napi-v*`

**Jobs**:
1. `build` - Matrix build for 6 platforms
2. `test` - Verify binaries on each OS
3. `publish` - npm publish on version tags

**Matrix Strategy**:
```yaml
matrix:
  include:
    - target: aarch64-apple-darwin
      os: macos-14
    - target: x86_64-apple-darwin
      os: macos-13
    - target: aarch64-unknown-linux-gnu
      os: ubuntu-latest
      use-cross: true
    - target: x86_64-unknown-linux-gnu
      os: ubuntu-latest
    - target: aarch64-pc-windows-msvc
      os: windows-latest
      use-cross: true
    - target: x86_64-pc-windows-msvc
      os: windows-latest
```

### 2. Package.json Updates

**codelet/napi/package.json**:
- Change `name` to `@sengac/codelet-napi`
- Set `private: false`
- Add platform packages via `napi prepublish -t npm`

**package.json** (root fspec):
- Change `"codelet-napi": "file:codelet/napi"` to `"@sengac/codelet-napi": "^0.1.0"`

### 3. Repository Secrets

Required GitHub secrets for npm publishing:
- `NPM_TOKEN` - npm access token for @sengac scope

## Reference: Similar Projects

### @sengac/tree-sitter-* Packages

These use `prebuildify` (not NAPI-RS) but same distribution pattern:
- Main package with `optionalDependencies`
- Platform-specific packages auto-installed by npm
- Prebuilds for same 6 platforms

### NAPI-RS Template

Official template at `github.com/napi-rs/package-template`:
- 13+ platform targets
- Matrix build with cargo-zigbuild for musl
- Docker/QEMU for ARM testing
- Semantic versioning from git tags

## Key Dependencies

This work unit depends on:
- **NAPI-001**: Codelet NAPI-RS Native Module Bindings (done)

## Files to Create/Modify

| File | Action |
|------|--------|
| `.github/workflows/build-codelet-napi.yml` | CREATE |
| `codelet/napi/package.json` | MODIFY (name, private, add npm scripts) |
| `package.json` | MODIFY (change codelet-napi dependency) |

## Build Command Reference

Local build (current):
```bash
cd codelet/napi && npm run build
# Runs: napi build --platform --release
```

CI build (NAPI-RS):
```bash
napi build --platform --release --target <target>
```

Prepublish (generates platform packages):
```bash
napi prepublish -t npm
```
