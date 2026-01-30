#!/bin/bash

# Batch migration script for converting test files to use shared test setup
# This script automates the migration pattern from MIGRATION-GUIDE.md

echo "=== Batch Test Migration Script ==="
echo ""

# Function to migrate a single test file
migrate_test_file() {
    local file="$1"
    echo "🔄 Migrating: $file"
    
    # Skip if file doesn't exist or already migrated
    if [ ! -f "$file" ]; then
        echo "   ⚠️  File not found, skipping"
        return 1
    fi
    
    if grep -q "setupTestDirectory\|setupWorkUnitTest\|setupFoundationTest\|setupFullTest" "$file"; then
        echo "   ✅ Already migrated, skipping"
        return 0
    fi
    
    # Create backup
    cp "$file" "$file.backup"
    
    # Determine which setup function to use based on file content and imports
    local setup_type="setupTestDirectory"
    local setup_interface="TestDirectorySetup"
    
    if grep -q "work-unit\|workUnit\|WorkUnit" "$file"; then
        setup_type="setupWorkUnitTest"
        setup_interface="WorkUnitTestSetup"
    elif grep -q "foundation" "$file"; then
        setup_type="setupFoundationTest" 
        setup_interface="FoundationTestSetup"
    fi
    
    # If it uses both work units and foundation, use setupFullTest
    if grep -q "work-unit\|workUnit\|WorkUnit" "$file" && grep -q "foundation" "$file"; then
        setup_type="setupFullTest"
        setup_interface="FullTestSetup"
    fi
    
    # Determine the relative path to test-helpers based on file location
    local relative_path="../../test-helpers/universal-test-setup"
    if [[ "$file" == *"/src/test/"* ]]; then
        relative_path="../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/tui/"* ]]; then
        relative_path="../../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/utils/"* ]]; then
        relative_path="../../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/hooks/"* ]]; then
        relative_path="../../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/git/"* ]]; then
        relative_path="../../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/research-tools/"* ]]; then
        relative_path="../../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/migrations/"* ]]; then
        relative_path="../../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/generators/"* ]]; then
        relative_path="../../test-helpers/universal-test-setup"
    elif [[ "$file" == *"/src/validators/"* ]]; then
        relative_path="../../test-helpers/universal-test-setup"
    fi
    
    # Start migration
    cat > "$file.tmp" << 'EOF'
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
EOF
    
    # Add specific imports based on original file content
    grep "^import.*from '\.\." "$file" | grep -v "mkdtemp\|rm\|readFile\|writeFile\|mkdir\|tmpdir" >> "$file.tmp"
    
    # Add universal test setup imports
    echo "import {" >> "$file.tmp"
    echo "  $setup_type," >> "$file.tmp"
    echo "  type $setup_interface," >> "$file.tmp"
    echo "} from '$relative_path';" >> "$file.tmp"
    
    # Add test file operations import if needed
    if grep -q "JSON.parse\|JSON.stringify\|writeFile\|readFile" "$file"; then
        echo "import {" >> "$file.tmp"
        echo "  writeTextFile," >> "$file.tmp"
        echo "  writeJsonTestFile," >> "$file.tmp"
        echo "  readJsonTestFile," >> "$file.tmp"
        echo "  ensureTestDirectory," >> "$file.tmp"
        echo "} from '${relative_path%/*}/test-file-operations';" >> "$file.tmp"
    fi
    
    echo "" >> "$file.tmp"
    
    # Extract the describe block and modify variables
    sed -n '/describe(/,/afterEach/p' "$file" | \
    sed 's/let testDir: string;/let setup: '"$setup_interface"';/' | \
    sed 's/let specDir: string;//' | \
    sed 's/let workUnitsFile: string;//' | \
    sed 's/let prefixesFile: string;//' | \
    sed 's/let epicsFile: string;//' | \
    sed 's/let featuresDir: string;//' | \
    sed 's/let foundationFile: string;//' | \
    sed '/beforeEach/,/});/{
        s/testDir = await mkdtemp.*/setup = await '"$setup_type"'('\''test-name'\'');/
        /mkdir.*specDir\|mkdir.*spec/d
        /writeFile.*workUnitsFile/d
        /await rm(testDir/c\    await setup.cleanup();
    }' >> "$file.tmp"
    
    # Extract and modify the rest of the file
    sed -n '/describe.*Scenario/,$p' "$file" | \
    sed 's/testDir/setup.testDir/g' | \
    sed 's/specDir/setup.specDir/g' | \
    sed 's/workUnitsFile/setup.workUnitsFile/g' | \
    sed 's/prefixesFile/setup.prefixesFile/g' | \
    sed 's/epicsFile/setup.epicsFile/g' | \
    sed 's/featuresDir/setup.featuresDir/g' | \
    sed 's/foundationFile/setup.foundationFile/g' | \
    sed 's/await writeFile(\([^,]*\), JSON.stringify(\([^)]*\)))/await writeJsonTestFile(\1, \2)/g' | \
    sed 's/JSON.parse(await readFile(\([^,]*\), .utf-8.))/await readJsonTestFile(\1)/g' | \
    sed 's/await mkdir(\([^,]*\), { recursive: true });/await ensureTestDirectory(\1);/g' >> "$file.tmp"
    
    # Replace original file
    mv "$file.tmp" "$file"
    
    echo "   ✅ Migrated successfully"
    return 0
}

# Test the migration on one file first
echo "🧪 Testing migration on one file first..."

if migrate_test_file "src/commands/__tests__/dependencies.test.ts"; then
    echo ""
    echo "✅ Test migration successful!"
    echo ""
    echo "Would you like to continue with all remaining files? (y/N)"
    read -r response
    
    if [[ "$response" =~ ^[Yy]$ ]]; then
        echo ""
        echo "🚀 Starting batch migration of all remaining files..."
        echo ""
        
        # Get all files that need migration
        files_to_migrate=$(find src -name "*.test.ts" -exec bash -c 'grep -q "mkdtemp\|mkdirSync.*tmp.*\|mkdtempSync" "$1" && ! grep -q "setupTestDirectory\|setupWorkUnitTest\|setupFoundationTest\|setupFullTest" "$1" && echo "$1"' _ {} \;)
        
        total_files=$(echo "$files_to_migrate" | wc -l)
        current=0
        
        echo "Found $total_files files to migrate"
        echo ""
        
        for file in $files_to_migrate; do
            current=$((current + 1))
            echo "[$current/$total_files] Migrating: $file"
            migrate_test_file "$file"
        done
        
        echo ""
        echo "🎉 Batch migration completed!"
        echo ""
        echo "Running verification script..."
        ./scripts/verify-test-migration.sh
    else
        echo "Migration cancelled."
    fi
else
    echo ""
    echo "❌ Test migration failed. Please check the dependencies.test.ts file and fix any issues."
    echo "You can restore from backup: mv src/commands/__tests__/dependencies.test.ts.backup src/commands/__tests__/dependencies.test.ts"
fi