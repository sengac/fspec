#!/bin/bash
# Hook: Run linter and enforce code quality
#
# This hook runs a linter (e.g., eslint) and fails if any errors are found.
# Use with blocking: true to prevent command execution on lint errors.

echo "Running linter..."

# Run linter (example: npm run lint)
# Capture output and exit code
if npm run lint --silent 2>&1; then
  # Lint passed
  echo "✓ No lint errors found"
  exit 0
else
  # Lint failed
  lint_exit_code=$?

  # Write errors to stderr
  echo "✗ Lint errors found" >&2
  echo "Run 'npm run lint' to see details" >&2

  # Exit with non-zero code for lint errors
  exit 1
fi
