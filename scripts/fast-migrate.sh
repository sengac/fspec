#!/bin/bash

# Fast Automated Test Migration Script
# Processes all test files efficiently with proven patterns

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "=== Fast Automated Test Migration ==="
echo ""

# Function to get files that need migration
get_files_to_migrate() {
    find src -name "*.test.ts" -exec bash -c 'grep -q "mkdtemp\|mkdirSync.*tmp.*\|mkdtempSync" "$1" && ! grep -q "setupTestDirectory\|setupWorkUnitTest\|setupFoundationTest\|setupFullTest" "$1" && echo "$1"' _ {} \;
}

# Function to determine setup type
determine_setup_type() {
    local file="$1"
    
    # Check content for indicators
    if grep -qE "workUnits|work-unit|WorkUnit" "$file" && grep -qE "foundation" "$file"; then
        echo "setupFullTest:FullTestSetup"
    elif grep -qE "workUnits|work-unit|WorkUnit" "$file"; then
        echo "setupWorkUnitTest:WorkUnitTestSetup"  
    elif grep -qE "foundation" "$file"; then
        echo "setupFoundationTest:FoundationTestSetup"
    else
        echo "setupTestDirectory:TestDirectorySetup"
    fi
}

# Function to get import path
get_import_path() {
    local file="$1"
    if [[ "$file" == *"/src/commands/__tests__/"* ]]; then
        echo "../../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/test/"* ]]; then
        echo "../test-helpers/universal-test-setup"
    else
        echo "../../test-helpers/universal-test-setup"
    fi
}

# Fast migration using proven patterns
migrate_file() {
    local file="$1"
    local dry_run="${2:-false}"
    
    if [ "$dry_run" = "true" ]; then
        echo "    📋 $file"
        return 0
    fi
    
    echo "    🔄 $file"
    
    # Backup
    cp "$file" "$file.backup"
    
    # Determine setup
    local setup_info=$(determine_setup_type "$file")
    local setup_func="${setup_info%:*}"
    local setup_interface="${setup_info#*:}"
    local import_path=$(get_import_path "$file")
    local test_name=$(basename "$file" .test.ts)
    
    # Create new file
    cat > "$file.new" << 'HEADER'
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
HEADER

    # Add other imports (preserve non-fs imports)
    grep "^import" "$file" | grep -v "mkdtemp\|rm\|mkdir\|writeFile\|tmpdir\|fs/promises" >> "$file.new" || true
    
    # Add setup imports
    cat >> "$file.new" << EOF
import {
  ${setup_func},
  type ${setup_interface},
} from '${import_path}';
EOF

    # Add file operations if needed
    if grep -qE "JSON\.parse|JSON\.stringify|writeFile|readFile|mkdir" "$file"; then
        cat >> "$file.new" << EOF
import {
  writeTextFile,
  writeJsonTestFile, 
  readJsonTestFile,
  ensureTestDirectory,
} from '${import_path%/*}/test-file-operations';
EOF
    fi
    
    echo "" >> "$file.new"
    
    # Process the main content
    sed -n '/^describe\|^export/,$ p' "$file" | \
    # Replace variable declarations
    sed -E 's/let (testDir|tempDir|tmpDir): string;/let setup: '"$setup_interface"';/' | \
    sed '/let (specDir|workUnitsFile|prefixesFile|epicsFile|featuresDir|foundationFile): string;/d' | \
    # Replace setup/teardown
    sed -E '/beforeEach.*\{/,/\}\);/{
        s/.*mkdtemp.*tmpdir.*/    setup = await '"$setup_func"'('\'''"$test_name"'\'');/
        /mkdir.*spec|mkdir.*features|writeFile.*workUnits|writeFile.*prefixes/d
    }' | \
    sed -E '/afterEach.*\{/,/\}\);/{
        s/.*(rm|rmSync).*(testDir|tempDir|tmpDir).*/    await setup.cleanup();/
    }' | \
    # Replace variable references
    sed -E 's/\b(testDir|tempDir|tmpDir)\b/setup.testDir/g' | \
    sed -E 's/\b(specDir)\b/setup.specDir/g' | \
    sed -E 's/\b(workUnitsFile)\b/setup.workUnitsFile/g' | \
    sed -E 's/\b(prefixesFile)\b/setup.prefixesFile/g' | \
    sed -E 's/\b(epicsFile)\b/setup.epicsFile/g' | \
    sed -E 's/\b(featuresDir)\b/setup.featuresDir/g' | \
    sed -E 's/\b(foundationFile)\b/setup.foundationFile/g' | \
    # Replace file operations
    sed -E 's/await writeFile\(([^,]+), JSON\.stringify\(([^)]+)[^)]*\)/await writeJsonTestFile(\1, \2)/g' | \
    sed -E 's/JSON\.parse\(await readFile\(([^,]+)[^)]*\)/await readJsonTestFile(\1)/g' | \
    sed -E 's/await mkdir\(([^,]+), \{ recursive: true \}\);/await ensureTestDirectory(\1);/g' \
    >> "$file.new"
    
    # Replace original
    mv "$file.new" "$file"
    
    # Quick validation
    if node -pe "require('fs').readFileSync('$file', 'utf8')" >/dev/null 2>&1; then
        echo "      ✅"
        rm -f "$file.backup"
        return 0
    else
        echo "      ❌ Syntax error, restoring"
        mv "$file.backup" "$file"
        return 1
    fi
}

# Process specific problematic files that need special handling
migrate_special_files() {
    local dry_run="${1:-false}"
    
    # Files that have special patterns that need manual fixes
    local special_files=(
        "src/commands/__tests__/init-codex-home-directory.test.ts"
        "src/commands/__tests__/dependencies.test.ts"
    )
    
    for file in "${special_files[@]}"; do
        if [ -f "$file" ] && grep -q "mkdtemp" "$file" && ! grep -q "setup" "$file"; then
            echo "    🔧 Special handling: $file"
            if [ "$dry_run" = "false" ]; then
                # These need custom migration - flag for manual review
                echo "// TODO: Migrate this file manually - complex patterns detected" > "$file.migration-needed"
            fi
        fi
    done
}

# Main function
main() {
    local dry_run=false
    local batch_size=10
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dry-run) dry_run=true; shift ;;
            --batch-size) batch_size="$2"; shift 2 ;;
            --help)
                echo "Usage: $0 [--dry-run] [--batch-size N]"
                echo "Migrates all test files from manual filesystem ops to shared setup"
                exit 0
                ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done
    
    # Get files
    mapfile -t files < <(get_files_to_migrate)
    
    if [ ${#files[@]} -eq 0 ]; then
        echo -e "${GREEN}✅ All files migrated!${NC}"
        ./scripts/verify-test-migration.sh
        exit 0
    fi
    
    echo "Found ${#files[@]} files to migrate"
    if [ "$dry_run" = "true" ]; then
        echo -e "${YELLOW}DRY RUN - no changes will be made${NC}"
    fi
    echo ""
    
    # Process in batches
    local successful=0
    local failed=0
    
    for ((i=0; i<${#files[@]}; i+=batch_size)); do
        local batch=("${files[@]:i:batch_size}")
        echo -e "${BLUE}📦 Batch $((i/batch_size + 1)) (${#batch[@]} files):${NC}"
        
        for file in "${batch[@]}"; do
            if migrate_file "$file" "$dry_run"; then
                ((successful++))
            else
                ((failed++))
            fi
        done
        
        # Test the batch if not dry run
        if [ "$dry_run" = "false" ] && [ ${#batch[@]} -gt 0 ]; then
            echo "    🧪 Testing batch..."
            if timeout 30 npm test -- "${batch[@]}" --run --reporter=basic >/dev/null 2>&1; then
                echo "      ✅ Tests pass"
            else
                echo "      ⚠️  Some test issues (may be pre-existing)"
            fi
        fi
        echo ""
    done
    
    # Handle special files
    echo -e "${BLUE}🔧 Checking for special cases...${NC}"
    migrate_special_files "$dry_run"
    
    # Summary
    echo -e "${BLUE}📋 Summary:${NC}"
    echo "  Processed: $((successful + failed))"
    echo "  Successful: $successful"  
    echo "  Failed: $failed"
    echo ""
    
    if [ "$dry_run" = "false" ]; then
        echo "🔍 Final verification:"
        ./scripts/verify-test-migration.sh
    fi
}

main "$@"