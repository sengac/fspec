#!/bin/bash

# Semi-automated test migration helper script
# Processes test files in batches based on their complexity and type

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "=== Semi-Automated Test Migration Helper ==="
echo ""

# Function to categorize a test file
categorize_test_file() {
    local file="$1"
    
    if grep -q "work-unit\|workUnit\|WorkUnit" "$file" && grep -q "foundation" "$file"; then
        echo "full"
    elif grep -q "work-unit\|workUnit\|WorkUnit" "$file"; then
        echo "workunit"
    elif grep -q "foundation" "$file"; then
        echo "foundation"
    else
        echo "directory"
    fi
}

# Function to determine the relative path based on file location
get_relative_path() {
    local file="$1"
    
    if [[ "$file" == *"/src/commands/__tests__/"* ]]; then
        echo "../../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/test/"* ]]; then
        echo "../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/tui/"* ]]; then
        echo "../../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/utils/"* ]]; then
        echo "../../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/hooks/"* ]]; then
        echo "../../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/git/"* ]]; then
        echo "../../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/research-tools/"* ]]; then
        echo "../../test-helpers/universal-test-setup"
    else
        echo "../../test-helpers/universal-test-setup"
    fi
}

# Function to migrate a single file
migrate_file() {
    local file="$1"
    local category="$2"
    local dry_run="${3:-false}"
    
    if [ "$dry_run" = "true" ]; then
        echo "    📋 Would migrate: $file (type: $category)"
        return 0
    fi
    
    echo "    🔄 Migrating: $file (type: $category)"
    
    # Create backup
    cp "$file" "$file.backup"
    
    local setup_func=""
    local setup_interface=""
    local relative_path=$(get_relative_path "$file")
    
    case "$category" in
        "full")
            setup_func="setupFullTest"
            setup_interface="FullTestSetup"
            ;;
        "workunit")
            setup_func="setupWorkUnitTest"
            setup_interface="WorkUnitTestSetup"
            ;;
        "foundation")
            setup_func="setupFoundationTest"
            setup_interface="FoundationTestSetup"
            ;;
        *)
            setup_func="setupTestDirectory"
            setup_interface="TestDirectorySetup"
            ;;
    esac
    
    # Create temporary file for migration
    local temp_file="${file}.migrating"
    
    # Step 1: Update imports section
    {
        # Keep initial imports, but filter out filesystem ones
        head -1 "$file"  # Keep the first line (usually a comment or import)
        echo "import { describe, it, expect, beforeEach, afterEach } from 'vitest';"
        echo "import { join } from 'path';"
        
        # Extract non-filesystem imports
        grep "^import.*from '\.\." "$file" | grep -v "mkdtemp\|rm\|readFile\|writeFile\|mkdir\|tmpdir\|fs/promises" || true
        
        # Add universal test setup imports
        echo "import {"
        echo "  $setup_func,"
        echo "  type $setup_interface,"
        echo "} from '$relative_path';"
        
        # Add file operations if needed
        if grep -q "JSON.parse\|JSON.stringify\|writeFile\|readFile\|mkdir" "$file"; then
            echo "import {"
            echo "  writeTextFile,"
            echo "  writeJsonTestFile,"
            echo "  readJsonTestFile,"
            echo "  ensureTestDirectory,"
            echo "} from '${relative_path%/*}/test-file-operations';"
        fi
        
        echo ""
    } > "$temp_file"
    
    # Step 2: Process the describe block and setup/teardown
    {
        # Find the describe line and extract test content
        sed -n '/describe(/,$ p' "$file" | \
        # Replace variable declarations
        sed 's/let testDir: string;/let setup: '"$setup_interface"';/' | \
        sed 's/let specDir: string;//' | \
        sed 's/let workUnitsFile: string;//' | \
        sed 's/let prefixesFile: string;//' | \
        sed 's/let epicsFile: string;//' | \
        sed 's/let featuresDir: string;//' | \
        sed 's/let foundationFile: string;//' | \
        sed 's/let tempDir: string;//' | \
        sed 's/let tmpDir: string;//' | \
        # Replace setup/teardown
        sed '/beforeEach.*{/,/});/{
            /testDir.*=.*mkdtemp/c\    setup = await '"$setup_func"'('\''test-name'\'');
            /tempDir.*=.*mkdtemp/c\    setup = await '"$setup_func"'('\''test-name'\'');
            /tmpDir.*=.*mkdtemp/c\    setup = await '"$setup_func"'('\''test-name'\'');
            /mkdir.*specDir\|mkdir.*spec/d
            /mkdir.*featuresDir\|mkdir.*features/d
            /writeFile.*workUnitsFile/d
            /writeFile.*prefixesFile/d
            /writeFile.*epicsFile/d
        }' | \
        sed '/afterEach.*{/,/});/{
            /rm.*testDir\|rmSync.*testDir/c\    await setup.cleanup();
            /rm.*tempDir\|rmSync.*tempDir/c\    await setup.cleanup();
            /rm.*tmpDir\|rmSync.*tmpDir/c\    await setup.cleanup();
        }' | \
        # Replace variable references
        sed 's/\btestDir\b/setup.testDir/g' | \
        sed 's/\btempDir\b/setup.testDir/g' | \
        sed 's/\btmpDir\b/setup.testDir/g' | \
        sed 's/\bspecDir\b/setup.specDir/g' | \
        sed 's/\bworkUnitsFile\b/setup.workUnitsFile/g' | \
        sed 's/\bprefixesFile\b/setup.prefixesFile/g' | \
        sed 's/\bepicsFile\b/setup.epicsFile/g' | \
        sed 's/\bfeaturesDir\b/setup.featuresDir/g' | \
        sed 's/\bfoundationFile\b/setup.foundationFile/g' | \
        # Replace common file operations
        sed 's/await writeFile(\([^,]*\), JSON\.stringify(\([^)]*\)[^)]*)/await writeJsonTestFile(\1, \2)/g' | \
        sed 's/JSON\.parse(await readFile(\([^,]*\), [^)]*)/await readJsonTestFile(\1)/g' | \
        sed 's/await mkdir(\([^,]*\), { recursive: true });/await ensureTestDirectory(\1);/g'
    } >> "$temp_file"
    
    # Replace original file
    mv "$temp_file" "$file"
    
    # Test if the migration worked by attempting to parse the TypeScript
    if ! npx tsc --noEmit "$file" 2>/dev/null; then
        echo "      ⚠️  TypeScript errors detected, restoring backup"
        mv "$file.backup" "$file"
        return 1
    fi
    
    # Run a quick test to see if it compiles
    echo "      ✅ Migration completed successfully"
    rm "$file.backup" 2>/dev/null || true
    return 0
}

# Get list of files to migrate
get_files_to_migrate() {
    find src -name "*.test.ts" -exec bash -c 'grep -q "mkdtemp\|mkdirSync.*tmp.*\|mkdtempSync" "$1" && ! grep -q "setupTestDirectory\|setupWorkUnitTest\|setupFoundationTest\|setupFullTest" "$1" && echo "$1"' _ {} \;
}

# Main migration logic
main() {
    local dry_run=false
    local category_filter=""
    local batch_size=5
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dry-run)
                dry_run=true
                shift
                ;;
            --category)
                category_filter="$2"
                shift 2
                ;;
            --batch-size)
                batch_size="$2"
                shift 2
                ;;
            --help)
                echo "Usage: $0 [--dry-run] [--category <type>] [--batch-size <num>]"
                echo ""
                echo "Options:"
                echo "  --dry-run          Show what would be migrated without making changes"
                echo "  --category <type>  Only migrate files of type: directory|foundation|workunit|full"
                echo "  --batch-size <num> Number of files to process in each batch (default: 5)"
                echo "  --help             Show this help message"
                exit 0
                ;;
            *)
                echo "Unknown option: $1"
                echo "Use --help for usage information"
                exit 1
                ;;
        esac
    done
    
    # Get all files that need migration
    files_to_migrate=($(get_files_to_migrate))
    
    if [ ${#files_to_migrate[@]} -eq 0 ]; then
        echo -e "${GREEN}✅ All files have been migrated!${NC}"
        ./scripts/verify-test-migration.sh
        exit 0
    fi
    
    echo "Found ${#files_to_migrate[@]} files that need migration"
    echo ""
    
    # Categorize files
    declare -A categories
    for file in "${files_to_migrate[@]}"; do
        category=$(categorize_test_file "$file")
        categories[$category]+="$file "
    done
    
    # Show categories
    echo -e "${BLUE}📊 File categorization:${NC}"
    for category in directory foundation workunit full; do
        count=$(echo "${categories[$category]}" | wc -w)
        if [ "$count" -gt 0 ]; then
            echo "  $category: $count files"
        fi
    done
    echo ""
    
    # If category filter is specified, only process that category
    if [ -n "$category_filter" ]; then
        if [ -z "${categories[$category_filter]}" ]; then
            echo -e "${YELLOW}⚠️  No files found in category '$category_filter'${NC}"
            exit 0
        fi
        
        files_to_process=(${categories[$category_filter]})
        echo -e "${BLUE}🎯 Processing only '$category_filter' category (${#files_to_process[@]} files)${NC}"
        echo ""
    else
        files_to_process=("${files_to_migrate[@]}")
    fi
    
    # Process files in batches
    local processed=0
    local successful=0
    local failed=0
    
    for ((i=0; i<${#files_to_process[@]}; i+=batch_size)); do
        local batch=("${files_to_process[@]:i:batch_size}")
        
        echo -e "${BLUE}📦 Batch $((i/batch_size + 1)): Processing ${#batch[@]} files${NC}"
        
        for file in "${batch[@]}"; do
            category=$(categorize_test_file "$file")
            
            if migrate_file "$file" "$category" "$dry_run"; then
                ((successful++))
            else
                ((failed++))
            fi
            ((processed++))
        done
        
        if [ "$dry_run" = "false" ] && [ ${#batch[@]} -gt 0 ]; then
            echo "      🧪 Testing batch..."
            if npm test -- "${batch[@]}" --run --reporter=basic 2>/dev/null; then
                echo -e "      ${GREEN}✅ Batch tests passed${NC}"
            else
                echo -e "      ${YELLOW}⚠️  Some tests in batch may have issues (check manually)${NC}"
            fi
        fi
        
        echo ""
    done
    
    # Summary
    echo -e "${BLUE}📋 Migration Summary:${NC}"
    echo "  Total processed: $processed"
    echo "  Successful: $successful"
    echo "  Failed: $failed"
    echo ""
    
    if [ "$dry_run" = "false" ]; then
        echo "🔍 Running verification script..."
        ./scripts/verify-test-migration.sh
    else
        echo "💡 Run without --dry-run to perform the actual migration"
    fi
}

main "$@"