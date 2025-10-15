#!/usr/bin/env python3
"""
Hook: Run pytest for work unit tests

This hook reads the context from stdin, extracts the workUnitId,
and runs pytest on the corresponding test file.
"""

import json
import sys
import subprocess
from pathlib import Path

# Read JSON context from stdin
context_json = sys.stdin.read()

try:
    context = json.loads(context_json)
except json.JSONDecodeError as e:
    print(f"Failed to parse JSON context: {e}", file=sys.stderr)
    sys.exit(1)

# Extract workUnitId from context
work_unit_id = context.get('workUnitId')

if not work_unit_id:
    print("No workUnitId in context", file=sys.stderr)
    sys.exit(1)

# Find test file for work unit
# AUTH-001 -> src/**/__tests__/*auth-001*.test.ts
test_pattern = work_unit_id.lower().replace('-', '-')
test_files = list(Path('src').glob(f'**/__tests__/*{test_pattern}*.test.ts'))

if not test_files:
    print(f"No test files found for {work_unit_id}")
    sys.exit(0)

# Run pytest on test files
print(f"Running tests for {work_unit_id}...")

result = subprocess.run(
    ['npm', 'test', '--', str(test_files[0])],
    capture_output=True,
    text=True
)

# Display output
if result.stdout:
    print(result.stdout)

if result.stderr:
    print(result.stderr, file=sys.stderr)

# Exit with pytest's exit code
sys.exit(result.returncode)
