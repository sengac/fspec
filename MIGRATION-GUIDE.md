/**
 * Migration Guide: Legacy Test to Universal Test Setup
 * 
 * This guide explains how to systematically migrate all legacy test files that use
 * manual mkdtemp/writeFile patterns to the new universal test setup utilities.
 */

## Migration Pattern

### BEFORE (Legacy Pattern):
```typescript
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';

describe('Feature: Some Test', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;
  // ... other file variables

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');
    
    await mkdir(specDir, { recursive: true });
    
    await writeFile(workUnitsFile, JSON.stringify({
      workUnits: {},
      states: { /* ... */ }
    }, null, 2));
    
    // ... more file setup
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  it('some test', async () => {
    // Test uses testDir, workUnitsFile, etc.
    await someCommand({ cwd: testDir });
    const data = JSON.parse(await readFile(workUnitsFile, 'utf-8'));
  });
});
```

### AFTER (Universal Setup Pattern):
```typescript
import { setupWorkUnitTest, type WorkUnitTestSetup } from '../../test-helpers/universal-test-setup';
import { readJsonTestFile, writeJsonTestFile } from '../../test-helpers/test-file-operations';

describe('Feature: Some Test', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('some-test');
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  it('some test', async () => {
    // Test uses setup.testDir, setup.workUnitsFile, etc.
    await someCommand({ cwd: setup.testDir });
    const data = await readJsonTestFile(setup.workUnitsFile);
  });
});
```

## Setup Function Selection Guide

Choose the appropriate setup function based on what your test needs:

### 1. Basic Directory Only
```typescript
import { setupTestDirectory } from '../../test-helpers/universal-test-setup';

// For tests that just need a temp directory
const setup = await setupTestDirectory('test-name');
// Provides: setup.testDir, setup.cleanup()
```

### 2. Work Unit Environment
```typescript
import { setupWorkUnitTest } from '../../test-helpers/universal-test-setup';

// For tests that need work units, prefixes, epics
const setup = await setupWorkUnitTest('test-name');
// Provides: setup.testDir, setup.workUnitsFile, setup.prefixesFile, 
//           setup.epicsFile, setup.specDir, setup.featuresDir, setup.cleanup()
```

### 3. Foundation Only
```typescript
import { setupFoundationTest } from '../../test-helpers/universal-test-setup';

// For tests that need foundation.json
const setup = await setupFoundationTest('test-name');
// Provides: setup.testDir, setup.foundationFile, setup.specDir, setup.cleanup()
```

### 4. Full Environment
```typescript
import { setupFullTest } from '../../test-helpers/universal-test-setup';

// For tests that need both foundation and work units
const setup = await setupFullTest('test-name');
// Provides: All of the above combined
```

## Common Migration Steps

### Step 1: Update Imports
```typescript
// Remove these:
import { mkdtemp, rm, readFile, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';

// Add these:
import { setupWorkUnitTest, type WorkUnitTestSetup } from '../../test-helpers/universal-test-setup';
import { readJsonTestFile, writeJsonTestFile } from '../../test-helpers/test-file-operations';
```

### Step 2: Replace Variables
```typescript
// Replace these:
let testDir: string;
let specDir: string;
let workUnitsFile: string;
let prefixesFile: string;

// With this:
let setup: WorkUnitTestSetup;
```

### Step 3: Update beforeEach/afterEach
```typescript
// Replace this:
beforeEach(async () => {
  testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  // ... file setup
});

afterEach(async () => {
  await rm(testDir, { recursive: true, force: true });
});

// With this:
beforeEach(async () => {
  setup = await setupWorkUnitTest('test-name');
});

afterEach(async () => {
  await setup.cleanup();
});
```

### Step 4: Update Variable References
```typescript
// Replace:
testDir        → setup.testDir
workUnitsFile  → setup.workUnitsFile
prefixesFile   → setup.prefixesFile
specDir        → setup.specDir

// Replace file operations:
await readFile(file, 'utf-8') → await readJsonTestFile(file)
JSON.parse(await readFile(...)) → await readJsonTestFile(...)
await writeFile(file, JSON.stringify(...)) → await writeJsonTestFile(file, data)
```

### Step 5: Update Prefix Registration
```typescript
// Replace this:
const prefixes = JSON.parse(await readFile(prefixesFile, 'utf-8'));
prefixes.prefixes.AUTH = { description: 'Authentication features' };
await writeFile(prefixesFile, JSON.stringify(prefixes, null, 2));

// With this:
await registerTestPrefix(setup.testDir, 'AUTH', 'Authentication features');
```

## Files to Migrate (Priority Order)

### High Priority (Core functionality):
- ✅ /src/test/system-reminder-preservation.test.ts (DONE)
- ✅ /src/test/fspec-session-interception.test.ts (DONE)  
- ✅ /src/commands/__tests__/work-unit.test.ts (PARTIALLY DONE)
- ✅ /src/commands/__tests__/kanban-workflow.test.ts (PARTIALLY DONE)
- ⏳ /src/commands/__tests__/update-work-unit-status-done-sorting.test.ts
- ⏳ /src/commands/__tests__/validate.test.ts

### Medium Priority (Command tests):
All other files in /src/commands/__tests__/ that use mkdtemp

### Lower Priority (Utility tests):
Files in /src/utils/__tests__/, /src/hooks/__tests__/, etc.

## Automation Script

You can use this pattern to create a migration script:

```bash
#!/bin/bash
# migrate-test-file.sh

file="$1"
if [ -z "$file" ]; then
  echo "Usage: $0 <test-file.ts>"
  exit 1
fi

# Backup original
cp "$file" "$file.backup"

# Basic replacements
sed -i.tmp 's/mkdtemp, rm, readFile, mkdir, writeFile/readFile/g' "$file"
sed -i.tmp 's/tmpdir/setupWorkUnitTest, type WorkUnitTestSetup/g' "$file"
# ... more replacements

echo "Migrated $file (backup saved as $file.backup)"
```

## Testing Migration

After migrating a test file:

1. **Run the specific test**: `npm test -- path/to/test.test.ts`
2. **Check it uses temp directories**: Look for paths starting with OS temp dir
3. **Verify cleanup**: No temp files should remain after test completion
4. **Check test logic**: All assertions should still pass

## Benefits After Migration

1. **No more manual file setup** - handled by utilities
2. **Consistent test patterns** - easier to understand and maintain
3. **Proper cleanup** - no leftover temp files
4. **Type safety** - setup objects provide typed file paths
5. **DRY principle** - no code duplication across test files
6. **Easy to extend** - add new file types to setup utilities as needed