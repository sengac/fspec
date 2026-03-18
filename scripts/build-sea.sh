#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# Node.js SEA (Single Executable Application) Build Script for fspec
#
# Produces a standalone binary that embeds the Node.js runtime + all JS deps +
# the NAPI-RS native addon (.node file) as a SEA asset.
#
# Requirements:
#   - Node.js >= 25.5.0 (for --build-sea flag)
#   - macOS: codesign (ships with Xcode CLI tools)
#
# Usage:
#   ./scripts/build-sea.sh                    # Build for current platform
#   ./scripts/build-sea.sh --clean            # Clean dist/sea/ first
#   ./scripts/build-sea.sh --verbose          # Verbose output
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CLEAN=false
VERBOSE=false
for arg in "$@"; do
  case "$arg" in
    --clean) CLEAN=true ;;
    --verbose) VERBOSE=true ;;
  esac
done

log() { echo "[sea] $1"; }
vlog() { $VERBOSE && echo "[sea:verbose] $1" || true; }

# ---------------------------------------------------------------------------
# Validate environment
# ---------------------------------------------------------------------------
NODE_VERSION=$(node -e "console.log(process.version)")
log "Node.js version: $NODE_VERSION"

if ! node --help 2>&1 | grep -q '\-\-build-sea'; then
  echo "❌ Node.js $NODE_VERSION does not support --build-sea"
  echo "   Requires Node.js >= 25.5.0"
  exit 1
fi

PLATFORM=$(node -e "console.log(process.platform)")
ARCH=$(node -e "console.log(process.arch)")
log "Target: $PLATFORM-$ARCH"

# ---------------------------------------------------------------------------
# Directories
# ---------------------------------------------------------------------------
SEA_DIR="$PROJECT_ROOT/dist/sea"
SEA_BUNDLE="$SEA_DIR/fspec-sea.mjs"
SEA_CONFIG="$SEA_DIR/sea-config.json"

if $CLEAN && [ -d "$SEA_DIR" ]; then
  log "Cleaning dist/sea/ ..."
  rm -rf "$SEA_DIR"
fi

mkdir -p "$SEA_DIR"

# ---------------------------------------------------------------------------
# Step 1: Resolve the platform-specific .node file
# ---------------------------------------------------------------------------
log "Step 1: Resolving NAPI-RS native addon ..."

NAPI_DIR="$PROJECT_ROOT/codelet/napi"

case "$PLATFORM-$ARCH" in
  darwin-arm64)  NODE_FILE="codelet-napi.darwin-arm64.node" ;;
  darwin-x64)    NODE_FILE="codelet-napi.darwin-x64.node" ;;
  linux-x64)     NODE_FILE="codelet-napi.linux-x64-gnu.node" ;;
  linux-arm64)   NODE_FILE="codelet-napi.linux-arm64-gnu.node" ;;
  win32-x64)     NODE_FILE="codelet-napi.win32-x64-msvc.node" ;;
  win32-arm64)   NODE_FILE="codelet-napi.win32-arm64-msvc.node" ;;
  *)
    echo "❌ Unsupported platform: $PLATFORM-$ARCH"
    exit 1
    ;;
esac

NODE_FILE_PATH="$NAPI_DIR/$NODE_FILE"
if [ ! -f "$NODE_FILE_PATH" ]; then
  echo "❌ Native addon not found: $NODE_FILE_PATH"
  echo "   Build it first: npm run build:codelet-napi"
  exit 1
fi

log "  Found: $NODE_FILE ($(du -h "$NODE_FILE_PATH" | cut -f1 | xargs))"

# ---------------------------------------------------------------------------
# Step 2: Create NAPI-RS shim + devtools shim
# ---------------------------------------------------------------------------
log "Step 2: Creating shim modules ..."

# NAPI-RS shim: at runtime in SEA mode, extracts .node from assets via process.dlopen()
cat > "$SEA_DIR/napi-sea-shim.mjs" << 'SHIM_EOF'
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';

let nativeBinding;

let isSEA = false;
try {
  const sea = await import('node:sea');
  isSEA = sea.getAssetKeys().length > 0;
} catch { isSEA = false; }

if (isSEA) {
  const sea = await import('node:sea');
  const addonAsset = sea.getRawAsset('codelet-napi.node');
  const tmpDir = path.join(os.tmpdir(), 'fspec-sea-' + process.pid);
  fs.mkdirSync(tmpDir, { recursive: true });
  const addonPath = path.join(tmpDir, 'codelet-napi.node');
  fs.writeFileSync(addonPath, new Uint8Array(addonAsset));
  const mod = { exports: {} };
  process.dlopen(mod, addonPath);
  nativeBinding = mod.exports;
  process.on('exit', () => {
    try { fs.rmSync(tmpDir, { recursive: true, force: true }); } catch {}
  });
} else {
  const require = createRequire(import.meta.url);
  nativeBinding = require('@sengac/codelet-napi');
}

export default nativeBinding;
SHIM_EOF

# Append named exports from the original NAPI-RS index.js
node -e "
const fs = require('fs');
const content = fs.readFileSync('$NAPI_DIR/index.js', 'utf-8');
const exports = [];
for (const line of content.split('\n')) {
  const m = line.match(/^export \{ (\w+) \}$/);
  if (m) exports.push(m[1]);
}
const lines = exports.map(n => 'export const ' + n + ' = nativeBinding.' + n + ';');
fs.appendFileSync('$SEA_DIR/napi-sea-shim.mjs', '\n' + lines.join('\n') + '\n');
console.log('  Shimmed ' + exports.length + ' NAPI exports');
"

# Empty devtools shim (react-devtools-core not available in SEA)
cat > "$SEA_DIR/devtools-shim.mjs" << 'DEVTOOLS_EOF'
export default { initialize() {}, connectToDevTools() {} };
DEVTOOLS_EOF

# ---------------------------------------------------------------------------
# Step 3: Bundle with esbuild (ESM, all deps inlined)
# ---------------------------------------------------------------------------
log "Step 3: Bundling with esbuild (ESM, all deps inlined) ..."

# Use the Vite-built dist/index.js as entry point (not src/index.ts)
# because Vite resolves import.meta.glob and other Vite-specific transforms.
VITE_BUNDLE="$PROJECT_ROOT/dist/index.js"
if [ ! -f "$VITE_BUNDLE" ]; then
  echo "❌ Vite bundle not found: $VITE_BUNDLE"
  echo "   Run 'npm run build' first"
  exit 1
fi

npx esbuild "$VITE_BUNDLE" \
  --bundle \
  --platform=node \
  --target=node22 \
  --format=esm \
  --outfile="$SEA_BUNDLE" \
  --alias:@sengac/codelet-napi="$SEA_DIR/napi-sea-shim.mjs" \
  --alias:react-devtools-core="$SEA_DIR/devtools-shim.mjs" \
  --define:process.env.DEV='"false"' \
  --minify-syntax \
  --minify-whitespace \
  --legal-comments=none \
  2>&1 | { $VERBOSE && cat || tail -3; }

# ---------------------------------------------------------------------------
# Step 3b: Post-process the bundle for SEA compatibility
# ---------------------------------------------------------------------------
log "  Post-processing bundle for SEA compat ..."

node -e "
const fs = require('fs');
let code = fs.readFileSync('$SEA_BUNDLE', 'utf-8');

// 1. Remove shebang (SEA binary doesn't need it)
code = code.replace(/^#!\/usr\/bin\/env node\n/, '');

// 2. Rename esbuild's conflicting top-level __filename var declaration
//    esbuild reuses the name for its own variable, clashing with our shim
code = code.replace(
  /^(var [^;]*), __filename,/m,
  '\$1, __esbuild_fn,'
);

// 3. Inline jsdom's default-stylesheet.css (it uses readFileSync at load time)
const cssPath = '$PROJECT_ROOT/node_modules/jsdom/lib/jsdom/browser/default-stylesheet.css';
if (fs.existsSync(cssPath)) {
  const css = fs.readFileSync(cssPath, 'utf-8');
  const escaped = JSON.stringify(css);
  code = code.replace(
    /fs\w*\.readFileSync\(\s*path\w*\.resolve\(__dirname,\s*\"\.\.\/\.\.\/browser\/default-stylesheet\.css\"\)\s*,\s*\{[^}]*\}\s*\)/,
    escaped
  );
}

// 4. Neutralize jsdom's require.resolve for xhr-sync-worker.js
code = code.replace(
  /__require\.resolve\s*\?\s*__require\.resolve\(\s*\"\.\/xhr-sync-worker\.js\"\s*\)\s*:\s*null/g,
  'null'
);
code = code.replace(
  /require\.resolve\s*\?\s*require\.resolve\(\s*\"\.\/xhr-sync-worker\.js\"\s*\)\s*:\s*null/g,
  'null'
);

// 5. Prepend CJS compat shims (require, __dirname, __filename for ESM)
const shim = [
  'import { createRequire as __sea_cR } from \"module\";',
  'import { fileURLToPath as __sea_fU } from \"url\";',
  'import { dirname as __sea_dN } from \"path\";',
  'var require = __sea_cR(import.meta.url);',
  'var __filename = __sea_fU(import.meta.url);',
  'var __dirname = __sea_dN(__filename);',
  ''
].join('\n');

code = shim + code;

fs.writeFileSync('$SEA_BUNDLE', code);
console.log('  Post-processing complete');
"

BUNDLE_SIZE=$(du -h "$SEA_BUNDLE" | cut -f1 | xargs)
log "  Bundle: $BUNDLE_SIZE"

# ---------------------------------------------------------------------------
# Step 4: Generate sea-config.json
# ---------------------------------------------------------------------------
log "Step 4: Generating sea-config.json ..."

OUTPUT_NAME="fspec"
if [ "$PLATFORM" = "win32" ]; then
  OUTPUT_NAME="fspec.exe"
fi

cat > "$SEA_CONFIG" << EOF
{
  "main": "$SEA_BUNDLE",
  "mainFormat": "module",
  "output": "$SEA_DIR/$OUTPUT_NAME",
  "disableExperimentalSEAWarning": true,
  "useCodeCache": false,
  "useSnapshot": false,
  "assets": {
    "codelet-napi.node": "$NODE_FILE_PATH"
  }
}
EOF

# Also copy package.json next to the SEA binary for version detection
cp "$PROJECT_ROOT/package.json" "$SEA_DIR/package.json"
# And into dist/ since the Vite bundle reads from ../package.json relative to __dirname
cp "$PROJECT_ROOT/package.json" "$PROJECT_ROOT/dist/package.json" 2>/dev/null || true

vlog "  Config written"

# ---------------------------------------------------------------------------
# Step 5: Build the SEA binary
# ---------------------------------------------------------------------------
log "Step 5: Building SEA binary with node --build-sea ..."

node --build-sea "$SEA_CONFIG"

if [ ! -f "$SEA_DIR/$OUTPUT_NAME" ]; then
  echo "❌ SEA build failed — output not found: $SEA_DIR/$OUTPUT_NAME"
  exit 1
fi

# ---------------------------------------------------------------------------
# Step 6: Code sign (macOS only)
# ---------------------------------------------------------------------------
if [ "$PLATFORM" = "darwin" ]; then
  log "Step 6: Code signing (macOS ad-hoc) ..."
  codesign --sign - "$SEA_DIR/$OUTPUT_NAME" 2>&1 || {
    echo "⚠️  Code signing failed (binary may still work)"
  }
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
BINARY_SIZE=$(du -h "$SEA_DIR/$OUTPUT_NAME" | cut -f1 | xargs)

log ""
log "╔════════════════════════════════════════════════════════════════╗"
log "║              SEA Build Complete                                ║"
log "╠════════════════════════════════════════════════════════════════╣"
log "║  Binary:    $SEA_DIR/$OUTPUT_NAME"
log "║  Size:      $BINARY_SIZE"
log "║  Platform:  $PLATFORM-$ARCH"
log "║  Node.js:   $NODE_VERSION"
log "╚════════════════════════════════════════════════════════════════╝"
log ""
log "To run:     $SEA_DIR/$OUTPUT_NAME --version"
log "To install: cp $SEA_DIR/$OUTPUT_NAME /usr/local/bin/fspec"
