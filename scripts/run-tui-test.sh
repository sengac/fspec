#!/usr/bin/env bash
#
# scripts/run-tui-test.sh
#
# Wrapper around `@microsoft/tui-test` that works around its transform
# pipeline: tui-test copies every non-hidden, non-node_modules directory
# under CWD into `.tui-test/cache/` and runs swc on every `.js`/`.ts`
# file it finds. The gitignored `tmp/` directory at the repo root
# contains external repositories cloned for AST indexing
# (`tmp/<repo-name>/…`) — some of them (e.g. `tmp/scala-scalafmt/`) ship
# JSX code in plain `.js` files, which swc rejects because it parses
# `.js` with `jsx: false`.
#
# The fix: move `tmp/` aside before invoking tui-test, then always
# restore it on exit (even if tests fail or the user Ctrl-C's).
#
# Usage:
#   ./scripts/run-tui-test.sh                 # run all e2e tests
#   ./scripts/run-tui-test.sh prov-095        # filter to matching tests
#   ./scripts/run-tui-test.sh --trace prov-095

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

TMP_DIR="$REPO_ROOT/tmp"
STASH_DIR=""

cleanup() {
  # Restore tmp/ if we moved it
  if [[ -n "$STASH_DIR" && -d "$STASH_DIR" ]]; then
    if [[ ! -e "$TMP_DIR" ]]; then
      mv "$STASH_DIR" "$TMP_DIR"
      echo "[run-tui-test] restored $TMP_DIR from $STASH_DIR" >&2
    else
      # tmp/ came back during the run — merge stash contents in.
      # This should be rare; warn loudly so the user can resolve.
      echo "[run-tui-test] WARNING: $TMP_DIR reappeared during run." >&2
      echo "[run-tui-test] Stashed contents left at: $STASH_DIR" >&2
    fi
  fi
}
trap cleanup EXIT INT TERM

# Move tmp/ out of CWD only if it exists and is non-empty
if [[ -d "$TMP_DIR" ]]; then
  STASH_DIR="$(mktemp -d "${TMPDIR:-/tmp}/fspec-tui-test-stash-XXXXXX")/tmp"
  mv "$TMP_DIR" "$STASH_DIR"
  echo "[run-tui-test] moved $TMP_DIR aside to $STASH_DIR" >&2
fi

# Also clear any stale cache from previous failed runs so swc re-transforms
rm -rf "$REPO_ROOT/.tui-test/cache"

# Forward all CLI args through to tui-test. Do NOT use `exec` — that
# would replace this shell and skip the `trap cleanup EXIT` handler,
# leaving tmp/ stashed indefinitely.
set +e
npx @microsoft/tui-test "$@"
TUI_EXIT=$?
set -e
exit "$TUI_EXIT"
