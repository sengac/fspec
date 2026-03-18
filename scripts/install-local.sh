#!/bin/bash
# Install fspec CLI locally as a Single Executable Application (SEA)

set -e

echo "🔧 Installing fspec CLI locally (SEA binary)..."

# Ensure we're in the project root
if [ ! -f "package.json" ]; then
    echo "❌ Error: Please run this script from the project root directory"
    exit 1
fi

INSTALL_DIR="/usr/local/bin"
BINARY_NAME="fspec"

# Install dependencies
echo "📦 Installing dependencies..."
npm install --legacy-peer-deps

# Build the SEA binary (includes Vite build + NAPI + esbuild + SEA)
echo "🔨 Building SEA binary..."
npm run build:sea

# Verify SEA binary was produced
SEA_BINARY="dist/sea/$BINARY_NAME"
if [ ! -f "$SEA_BINARY" ]; then
    echo "❌ SEA build failed — binary not found: $SEA_BINARY"
    exit 1
fi

# Remove any existing npm-linked fspec first
if command -v fspec &>/dev/null; then
    EXISTING=$(which fspec 2>/dev/null || true)
    if [ -n "$EXISTING" ] && [ -L "$EXISTING" ]; then
        # It's a symlink (likely from npm link) — check if it points into an nvm/npm path
        LINK_TARGET=$(readlink "$EXISTING" 2>/dev/null || true)
        if echo "$LINK_TARGET" | grep -q "node_modules\|nvm"; then
            echo "🧹 Removing old npm-linked fspec at $EXISTING..."
            npm unlink -g fspec 2>/dev/null || rm -f "$EXISTING"
        fi
    fi
fi

# Create wrapper script instead of symlink
# (Node.js SEA binaries don't work correctly when invoked via symlink —
#  the binary needs to be called by its real path)
SEA_BINARY_ABS="$(cd "$(dirname "$SEA_BINARY")" && pwd)/$(basename "$SEA_BINARY")"
echo "📋 Installing wrapper script $INSTALL_DIR/$BINARY_NAME → $SEA_BINARY_ABS..."

WRAPPER_SCRIPT="#!/bin/sh
exec \"$SEA_BINARY_ABS\" \"\$@\"
"

# Remove any existing file/symlink at the target
if [ -e "$INSTALL_DIR/$BINARY_NAME" ] || [ -L "$INSTALL_DIR/$BINARY_NAME" ]; then
    if [ -w "$INSTALL_DIR" ]; then
        rm -f "$INSTALL_DIR/$BINARY_NAME"
    else
        echo "   (requires sudo for $INSTALL_DIR)"
        sudo rm -f "$INSTALL_DIR/$BINARY_NAME"
    fi
fi

if [ -w "$INSTALL_DIR" ]; then
    printf '%s' "$WRAPPER_SCRIPT" > "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
else
    echo "   (requires sudo for $INSTALL_DIR)"
    printf '%s' "$WRAPPER_SCRIPT" | sudo tee "$INSTALL_DIR/$BINARY_NAME" > /dev/null
    sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"
fi

# Verify installation
INSTALLED_VERSION=$("$INSTALL_DIR/$BINARY_NAME" --version 2>&1 || true)

echo ""
echo "✅ fspec SEA binary installed successfully!"
echo ""
echo "   Wrapper: $INSTALL_DIR/$BINARY_NAME → $SEA_BINARY_ABS"
echo "   Version: $INSTALLED_VERSION"
echo "   Type:    Single Executable Application (Node.js embedded)"
echo ""
echo "You can now use the 'fspec' command globally:"
echo "  fspec --help"
echo ""
echo "To uninstall, run:"
echo "  rm $INSTALL_DIR/$BINARY_NAME"
