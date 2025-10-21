#!/bin/bash
# Verification script for BUG-023 fix
# Tests all 20 affected help commands to ensure COMMON PATTERNS display correctly

set -e

echo "========================================="
echo "Verifying COMMON PATTERNS Fix (BUG-023)"
echo "========================================="
echo ""

# Array of commands to test
commands=(
  "add-background"
  "add-dependencies"
  "add-diagram"
  "add-hook"
  "add-virtual-hook"
  "audit-coverage"
  "clear-virtual-hooks"
  "copy-virtual-hooks"
  "delete-features-by-tag"
  "delete-scenarios-by-tag"
  "delete-work-unit"
  "dependencies"
  "link-coverage"
  "list-virtual-hooks"
  "prioritize-work-unit"
  "query-bottlenecks"
  "query-orphans"
  "remove-dependency"
  "remove-virtual-hook"
  "show-coverage"
)

passed=0
failed=0
total=${#commands[@]}

for cmd in "${commands[@]}"; do
  echo -n "Testing: fspec $cmd --help ... "

  # Run command and capture output
  output=$(fspec "$cmd" --help 2>&1 || true)

  # Check for [object Object] in COMMON PATTERNS section
  if echo "$output" | grep -q "COMMON PATTERNS"; then
    if echo "$output" | grep -A 20 "COMMON PATTERNS" | grep -q "\[object Object\]"; then
      echo "❌ FAILED - Found [object Object]"
      failed=$((failed + 1))
    else
      echo "✅ PASSED"
      passed=$((passed + 1))
    fi
  else
    echo "⚠️  SKIPPED - No COMMON PATTERNS section"
  fi
done

echo ""
echo "========================================="
echo "Results:"
echo "  Passed: $passed"
echo "  Failed: $failed"
echo "  Total:  $total"
echo "========================================="

if [ $failed -gt 0 ]; then
  echo ""
  echo "❌ VERIFICATION FAILED - Some commands still show [object Object]"
  exit 1
else
  echo ""
  echo "✅ ALL TESTS PASSED - COMMON PATTERNS display correctly"
  exit 0
fi
