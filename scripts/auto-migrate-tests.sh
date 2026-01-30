#!/bin/bash

# Automated Test Migration Script
# Uses sed for fast TypeScript transformations

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "=== Automated Test Migration Script ==="
echo "Using sed-based transformations for maximum compatibility"
echo ""

# Function to get files that need migration
get_files_to_migrate() {
    find src -name "*.test.ts" -exec bash -c 'grep -q "mkdtemp\|mkdirSync.*tmp.*\|mkdtempSync" "$1" && ! grep -q "setupTestDirectory\|setupWorkUnitTest\|setupFoundationTest\|setupFullTest" "$1" && echo "$1"' _ {} \;
}

# Function to determine setup type based on file content
determine_setup_type() {
    local file="$1"
    
    if grep -q "work-unit\|workUnit\|WorkUnit" "$file" && grep -q "foundation" "$file"; then
        echo "setupFullTest:FullTestSetup"
    elif grep -q "work-unit\|workUnit\|WorkUnit" "$file"; then
        echo "setupWorkUnitTest:WorkUnitTestSetup"
    elif grep -q "foundation" "$file"; then
        echo "setupFoundationTest:FoundationTestSetup"
    else
        echo "setupTestDirectory:TestDirectorySetup"
    fi
}

# Function to get relative path for imports
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

# Function to migrate a single file using targeted replacements
migrate_file_targeted() {
    local file="$1"
    local dry_run="${2:-false}"
    
    if [ "$dry_run" = "true" ]; then
        echo "    📋 Would migrate: $file"
        return 0
    fi
    
    echo "    🔄 Migrating: $file"
    
    # Create backup
    cp "$file" "$file.backup"
    
    local setup_info=$(determine_setup_type "$file")
    local setup_func="${setup_info%:*}"
    local setup_interface="${setup_info#*:}"
    local import_path=$(get_import_path "$file")
    
    # Step 1: Remove old imports using sed
    sed -i'' -e '/import.*mkdtemp.*rm.*mkdir.*writeFile.*from.*fs\/promises/d' "$file"
    sed -i'' -e '/import.*tmpdir.*from.*os/d' "$file"
    sed -i'' -e '/import { join } from.*path/d' "$file"
    
    # Step 2: Add new imports after the existing import block
    local temp_file="${file}.tmp"
    awk '
    /^import/ { imports[NR] = $0; next }
    /^$/ && !added_imports && NR > 1 { 
        print "import { join } from '\''path'\'';"
        print "import {"
        print "  '"$setup_func"',"
        print "  type '"$setup_interface"',"
        print "} from '\'''"$import_path"''\'';"
        if (need_file_ops) {
            print "import {"
            print "  writeTextFile,"
            print "  writeJsonTestFile,"
            print "  readJsonTestFile,"
            print "  ensureTestDirectory,"
            print "} from '\'''"${import_path%/*}/test-file-operations"''\'';"
        }
        print ""
        added_imports = 1
    }
    { if (!(NR in imports)) print }
    ' "$file" > "$temp_file"
    
    # Also print the imports that we kept
    awk '/^import/ && !/mkdtemp|tmpdir|fs\/promises/ { print }' "$file" >> "$temp_file.imports"
    
    # Combine imports and content
    {
        cat "$temp_file.imports" 2>/dev/null || true
        echo "import { join } from 'path';"
        echo "import {"
        echo "  $setup_func,"
        echo "  type $setup_interface,"
        echo "} from '$import_path';"
        
        if grep -q "JSON.parse\|JSON.stringify\|writeFile\|readFile\|mkdir" "$file"; then
            echo "import {"
            echo "  writeTextFile,"
            echo "  writeJsonTestFile,"
            echo "  readJsonTestFile,"
            echo "  ensureTestDirectory,"
            echo "} from '${import_path%/*}/test-file-operations';"
        fi
        
        echo ""
        
        # Get everything after imports
        awk '!/^import/ || /^import.*from.*\.\.[^\/]/ { print }' "$file"
        
    } > "$temp_file.final"
    
    # Step 3: Replace variable declarations
    sed -i'' \
        -e 's/let testDir: string;/let setup: '"$setup_interface"';/' \
        -e 's/let specDir: string;//' \
        -e 's/let workUnitsFile: string;//' \
        -e 's/let prefixesFile: string;//' \
        -e 's/let epicsFile: string;//' \
        -e 's/let featuresDir: string;//' \
        -e 's/let foundationFile: string;//' \
        -e 's/let tempDir: string;//' \
        -e 's/let tmpDir: string;//' \
        "$temp_file.final"
    
    # Step 4: Replace setup/teardown blocks
    sed -i'' \
        -e '/beforeEach.*{/,/});/{
            /testDir.*=.*mkdtemp\|tempDir.*=.*mkdtemp\|tmpDir.*=.*mkdtemp/c\    setup = await '"$setup_func"'('\'''"${file##*/}"'.'${file##*.}'-test'\'');
            /mkdir.*specDir\|mkdir.*spec\|mkdir.*features/d
            /writeFile.*workUnitsFile\|writeFile.*prefixes\|writeFile.*epics/d
        }' \
        -e '/afterEach.*{/,/});/{
            /rm.*testDir\|rmSync.*testDir\|rm.*tempDir\|rmSync.*tempDir\|rm.*tmpDir\|rmSync.*tmpDir/c\    await setup.cleanup();
        }' \
        "$temp_file.final"
    
    # Step 5: Replace variable references
    sed -i'' \
        -e 's/\btestDir\b/setup.testDir/g' \
        -e 's/\btempDir\b/setup.testDir/g' \
        -e 's/\btmpDir\b/setup.testDir/g' \
        -e 's/\bspecDir\b/setup.specDir/g' \
        -e 's/\bworkUnitsFile\b/setup.workUnitsFile/g' \
        -e 's/\bprefixesFile\b/setup.prefixesFile/g' \
        -e 's/\bepicsFile\b/setup.epicsFile/g' \
        -e 's/\bfeaturesDir\b/setup.featuresDir/g' \
        -e 's/\bfoundationFile\b/setup.foundationFile/g' \
        "$temp_file.final"
    
    # Step 6: Replace common file operations
    sed -i'' \
        -e 's/await writeFile(\([^,]*\), JSON\.stringify(\([^)]*\)[^)]*)/await writeJsonTestFile(\1, \2)/g' \
        -e 's/JSON\.parse(await readFile(\([^,]*\), [^)]*)/await readJsonTestFile(\1)/g' \
        -e 's/await mkdir(\([^,]*\), { recursive: true });/await ensureTestDirectory(\1);/g' \
        "$temp_file.final"
    
    # Replace original file
    mv "$temp_file.final" "$file"
    
    # Clean up temp files
    rm -f "$temp_file" "$temp_file.imports" 2>/dev/null || true
    
    # Quick syntax check
    if node -c "$file" 2>/dev/null; then
        echo "      ✅ Migration successful"
        rm "$file.backup" 2>/dev/null || true
        return 0
    else
        echo "      ⚠️  Syntax issues detected, restoring backup"
        mv "$file.backup" "$file"
        return 1
    fi
}

# Simple sed-based migration for straightforward cases
migrate_file_simple() {
    local file="$1"
    local dry_run="${2:-false}"
    
    if [ "$dry_run" = "true" ]; then
        echo "    📋 Would migrate: $file"
        return 0
    fi
    
    echo "    🔄 Migrating: $file (simple mode)"
    
    # Create backup
    cp "$file" "$file.backup"
    
    local setup_info=$(determine_setup_type "$file")
    local setup_func="${setup_info%:*}"
    local setup_interface="${setup_info#*:}"
    local import_path=$(get_import_path "$file")
    
    # Create a new file with proper structure
    {
        # Extract and preserve comment headers
        head -10 "$file" | grep '^\/\*\|^\s*\*\|^ \*' || true
        
        # Add imports
        echo "import { describe, it, expect, beforeEach, afterEach } from 'vitest';"
        echo "import { join } from 'path';"
        
        # Add existing non-filesystem imports
        grep "^import" "$file" | grep -v "mkdtemp\|rm\|mkdir\|writeFile\|tmpdir\|fs/promises" || true
        
        echo "import {"
        echo "  $setup_func,"
        echo "  type $setup_interface,"
        echo "} from '$import_path';"
        
        # Add file operations if needed
        if grep -q "JSON.parse\|JSON.stringify\|writeFile\|readFile\|mkdir" "$file"; then
            echo "import {"
            echo "  writeTextFile,"
            echo "  writeJsonTestFile,"
            echo "  readJsonTestFile,"
            echo "  ensureTestDirectory,"
            echo "} from '${import_path%/*}/test-file-operations';"
        fi
        
        echo ""
        
        # Extract the test content and transform it
        sed -n '/describe(/,$ p' "$file" | \
        sed 's/let testDir: string;/let setup: '"$setup_interface"';/' | \
        sed 's/let specDir: string;//' | \
        sed 's/let workUnitsFile: string;//' | \
        sed 's/let prefixesFile: string;//' | \
        sed 's/let epicsFile: string;//' | \
        sed 's/let featuresDir: string;//' | \
        sed 's/let foundationFile: string;//' | \
        sed 's/let tempDir: string;//' | \
        sed 's/let tmpDir: string;//' | \
        sed '/beforeEach.*{/,/});/{
            /testDir.*=.*mkdtemp\|tempDir.*=.*mkdtemp\|tmpDir.*=.*mkdtemp/c\    setup = await '"$setup_func"'('\''test-'"${file##*/}"'\'');
            /mkdir.*specDir\|mkdir.*spec\|mkdir.*features/d
            /writeFile.*workUnitsFile\|writeFile.*prefixes\|writeFile.*epics/d
        }' | \
        sed '/afterEach.*{/,/});/{
            /rm.*testDir\|rmSync.*testDir\|rm.*tempDir\|rmSync.*tempDir\|rm.*tmpDir\|rmSync.*tmpDir/c\    await setup.cleanup();
        }' | \
        sed 's/\btestDir\b/setup.testDir/g' | \
        sed 's/\btempDir\b/setup.testDir/g' | \
        sed 's/\btmpDir\b/setup.testDir/g' | \
        sed 's/\bspecDir\b/setup.specDir/g' | \
        sed 's/\bworkUnitsFile\b/setup.workUnitsFile/g' | \
        sed 's/\bprefixesFile\b/setup.prefixesFile/g' | \
        sed 's/\bepicsFile\b/setup.epicsFile/g' | \
        sed 's/\bfeaturesDir\b/setup.featuresDir/g' | \
        sed 's/\bfoundationFile\b/setup.foundationFile/g' | \
        sed 's/await writeFile(\([^,]*\), JSON\.stringify(\([^)]*\)[^)]*)/await writeJsonTestFile(\1, \2)/g' | \
        sed 's/JSON\.parse(await readFile(\([^,]*\), [^)]*)/await readJsonTestFile(\1)/g' | \
        sed 's/await mkdir(\([^,]*\), { recursive: true });/await ensureTestDirectory(\1);/g'
        
    } > "$file.new"
    
    # Replace original file
    mv "$file.new" "$file"
    
    # Test compilation
    if npm run build &>/dev/null; then
        echo "      ✅ Migration successful"
        rm "$file.backup" 2>/dev/null || true
        return 0
    else
        echo "      ⚠️  Build failed, restoring backup"
        mv "$file.backup" "$file"
        return 1
    fi
}

# Main function
main() {
    local dry_run=false
    local use_simple=false
    local files=()
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dry-run)
                dry_run=true
                shift
                ;;
            --simple)
                use_simple=true
                shift
                ;;
            --files)
                shift
                while [[ $# -gt 0 && ! "$1" =~ ^-- ]]; do
                    files+=("$1")
                    shift
                done
                ;;
            --help)
                echo "Usage: $0 [--dry-run] [--simple] [--files file1 file2 ...]"
                echo ""
                echo "Options:"
                echo "  --dry-run    Show what would be migrated without making changes"
                echo "  --simple     Use simple sed-based migration instead of ast-grep"
                echo "  --files      Specify specific files to migrate"
                echo "  --help       Show this help message"
                exit 0
                ;;
            *)
                echo "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Get files to migrate
    if [ ${#files[@]} -eq 0 ]; then
        mapfile -t files < <(get_files_to_migrate)
    fi
    
    if [ ${#files[@]} -eq 0 ]; then
        echo -e "${GREEN}✅ All files have been migrated!${NC}"
        ./scripts/verify-test-migration.sh
        exit 0
    fi
    
    echo "Found ${#files[@]} files to migrate"
    echo ""
    
    if [ "$dry_run" = "true" ]; then
        echo -e "${YELLOW}🔍 DRY RUN - No files will be modified${NC}"
        echo ""
    fi
    
    local successful=0
    local failed=0
    
    for file in "${files[@]}"; do
        if [ "$use_simple" = "true" ]; then
            if migrate_file_simple "$file" "$dry_run"; then
                ((successful++))
            else
                ((failed++))
            fi
        else
            if migrate_file_targeted "$file" "$dry_run"; then
                ((successful++))
            else
                ((failed++))
            fi
        fi
    done
    
    echo ""
    echo -e "${BLUE}📋 Migration Summary:${NC}"
    echo "  Successful: $successful"
    echo "  Failed: $failed"
    echo ""
    
    if [ "$dry_run" = "false" ] && [ "$successful" -gt 0 ]; then
        echo "🔍 Running verification script..."
        ./scripts/verify-test-migration.sh
    fi
}

main "$@"