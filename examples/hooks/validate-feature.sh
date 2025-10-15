#!/bin/bash
# Hook: Validate that work unit has a linked feature file
#
# This hook reads the context from stdin, extracts the workUnitId,
# and checks if a corresponding feature file exists.

# Read JSON context from stdin
read -r context

# Parse workUnitId from JSON context
workUnitId=$(echo "$context" | grep -o '"workUnitId":"[^"]*"' | cut -d'"' -f4)

if [ -z "$workUnitId" ]; then
  echo "No workUnitId in context" >&2
  exit 1
fi

# Convert work unit ID to kebab-case feature file name
# AUTH-001 -> auth-001
featureId=$(echo "$workUnitId" | tr '[:upper:]' '[:lower:]')

# Check if feature file exists
featureFile="spec/features/${featureId}.feature"

if [ -f "$featureFile" ]; then
  echo "✓ Feature file exists: $featureFile"
  exit 0
else
  echo "✗ Feature file not found: $featureFile" >&2
  echo "Work unit $workUnitId must have a feature file before moving to testing" >&2
  exit 1
fi
