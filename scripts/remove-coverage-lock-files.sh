#!/bin/sh
# remove-coverage-lock-files.sh — delete leftover coverage lock files (BUG-551)
#
# fspec used to lock each coverage file with a sibling
# <name>.feature.coverage.lock that was never removed. Coverage writes now
# hold the project's one spec write lock (spec/.fspec-write.lock), so the
# per-feature lock files are leftovers. This deletes them and nothing else.
#
# Usage:
#   sh scripts/remove-coverage-lock-files.sh [project-dir]   # default: current directory

set -eu

project="${1:-.}"
features="$project/spec/features"

if [ ! -d "$features" ]; then
  echo "No spec/features directory in $project" >&2
  exit 1
fi

count=$(find "$features" -type f -name '*.feature.coverage.lock' | wc -l | tr -d ' ')
find "$features" -type f -name '*.feature.coverage.lock' -exec rm -f {} +
echo "Removed $count coverage lock file(s) from $features"
