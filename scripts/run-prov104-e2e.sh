#!/usr/bin/env bash
#
# scripts/run-prov104-e2e.sh
#
# Run the PROV-104 /model-navigation e2e (tui-test) without letting
# tui-test's blanket transform-copy duplicate the gigantic Rust build
# tree (codelet/target ~170G, codelet/patches ~6G, codelet/napi ~1G)
# into .tui-test/cache/ (which fills the disk).
#
# Strategy:
#   1. Copy the built fspec binary to a stable /tmp path (FSPEC_BIN).
#   2. Move the huge non-source dirs OUT of CWD (same-filesystem rename =
#      instant) so tui-test never walks them.
#   3. Run tui-test pointed at the copied binary.
#   4. Always restore the moved dirs on exit.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

BIN_SRC="$REPO_ROOT/codelet/target/debug/fspec"
BIN_DST="/tmp/fspec-prov104-e2e"
STASH_ROOT="/tmp/prov104-stash-$$"
BIG_DIRS=("codelet/target" "codelet/patches" "codelet/napi" "tmp")

restore() {
  for d in "${BIG_DIRS[@]}"; do
    base="$(basename "$d")"
    parent="$(dirname "$d")"
    if [[ -d "$STASH_ROOT/$base" && ! -e "$REPO_ROOT/$d" ]]; then
      mkdir -p "$REPO_ROOT/$parent"
      mv "$STASH_ROOT/$base" "$REPO_ROOT/$d"
      echo "[run-prov104] restored $d" >&2
    fi
  done
  rmdir "$STASH_ROOT" 2>/dev/null || true
}
trap restore EXIT

if [[ ! -x "$BIN_SRC" ]]; then
  echo "[run-prov104] missing binary: $BIN_SRC (build with: cd codelet && cargo build -p codelet-fspec --bin fspec)" >&2
  exit 1
fi

echo "[run-prov104] copying binary -> $BIN_DST" >&2
cp -f "$BIN_SRC" "$BIN_DST"
chmod +x "$BIN_DST"

mkdir -p "$STASH_ROOT"
for d in "${BIG_DIRS[@]}"; do
  if [[ -e "$REPO_ROOT/$d" ]]; then
    mv "$REPO_ROOT/$d" "$STASH_ROOT/$(basename "$d")"
    echo "[run-prov104] stashed $d" >&2
  fi
done

export FSPEC_BIN="$BIN_DST"
npx tui-test "${@:-prov-104-model-nav}"
