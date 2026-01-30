#!/bin/bash

# Verification script to check test migration progress
# This script checks for files still using manual filesystem operations
# instead of the shared test setup utilities

echo "=== Test Migration Verification Script ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Find files that use manual filesystem operations but NOT the shared setup
echo "🔍 Checking for files still using manual filesystem operations..."
echo ""

LEGACY_FILES=$(find src -name "*.test.ts" -exec bash -c 'grep -q "mkdtemp\|mkdirSync.*tmp.*\|mkdtempSync" "$1" && ! grep -q "setupTestDirectory\|setupWorkUnitTest\|setupFoundationTest\|setupFullTest" "$1" && echo "$1"' _ {} \;)

if [ -z "$LEGACY_FILES" ]; then
    echo -e "${GREEN}✅ SUCCESS: All test files have been migrated to use shared test setup utilities!${NC}"
    echo ""
    echo "All test files are now using:"
    echo "  - setupTestDirectory()"
    echo "  - setupWorkUnitTest()"
    echo "  - setupFoundationTest()"
    echo "  - setupFullTest()"
    echo ""
    exit 0
else
    echo -e "${RED}❌ MIGRATION INCOMPLETE${NC}"
    echo ""
    echo "The following files still use manual filesystem operations:"
    echo ""
    
    COUNT=0
    while IFS= read -r file; do
        if [ ! -z "$file" ]; then
            COUNT=$((COUNT + 1))
            echo -e "${YELLOW}  $COUNT. $file${NC}"
        fi
    done <<< "$LEGACY_FILES"
    
    echo ""
    echo -e "${RED}Total files remaining: $COUNT${NC}"
    echo ""
    
    echo "Manual patterns still found:"
    echo "  - mkdtemp()"
    echo "  - mkdirSync() with temp directories"
    echo "  - mkdtempSync()"
    echo ""
    
    echo "These should be replaced with:"
    echo "  - setupTestDirectory() for basic tests"
    echo "  - setupWorkUnitTest() for work unit tests"
    echo "  - setupFoundationTest() for foundation tests"
    echo "  - setupFullTest() for comprehensive tests"
    echo ""
    
    exit 1
fi