#!/bin/bash
# Based on cage's install-local.sh, simplified for single package

set -e

echo "🔧 Installing fspec CLI locally..."

# Ensure we're in the project root
if [ ! -f "package.json" ]; then
    echo "❌ Error: Please run this script from the project root directory"
    exit 1
fi

# Install dependencies
echo "📦 Installing dependencies..."
npm install

# Build the CLI
echo "🔨 Building CLI..."
npm run build

# Make CLI executable (redundant with build script, but ensures it)
echo "🔑 Making CLI executable..."
chmod +x dist/index.js

# Link the CLI globally
echo "🔗 Linking CLI globally..."
npm link

echo "✅ fspec CLI installed successfully!"
echo ""
echo "You can now use the 'fspec' command globally:"
echo "  fspec --help"
echo ""
echo "To uninstall, run:"
echo "  npm unlink -g fspec"
