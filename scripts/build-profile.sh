#!/bin/zsh
# build-profile.sh — build a debug fspec binary with embedded DWARF for
# macOS `sample` stack collection.
#
# debug=full: `sample` resolves symbols to file:line via embedded debug info,
# so the flame graph shows actual line numbers, not just function names.
#
# Usage: ./build-profile.sh   (from anywhere; runs from the workspace root)

set -euo pipefail
cd "$(dirname "$0")/../rust"

echo "==> Building debug build of fspec (DWARF: full)..."
cargo build -p codelet-fspec

BIN=target/debug/fspec
echo "==> Done: $BIN"
ls -lh "$BIN"
