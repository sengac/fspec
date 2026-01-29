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
import { registerTestPrefix } from '../../test-helpers/work-unit-test-fixtures';

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

### 4. Full Environment (Recommended for most tests)
```typescript
import { setupFullTest, type FullTestSetup } from '../../test-helpers/universal-test-setup';

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

// Add these (choose based on your needs):
import { setupFullTest, type FullTestSetup } from '../../test-helpers/universal-test-setup';
// OR
import { setupWorkUnitTest, type WorkUnitTestSetup } from '../../test-helpers/universal-test-setup';

import { readJsonTestFile, writeJsonTestFile } from '../../test-helpers/test-file-operations';
import { registerTestPrefix } from '../../test-helpers/work-unit-test-fixtures';
```

### Step 2: Replace Variables
```typescript
// Replace these:
let testDir: string;
let specDir: string;
let workUnitsFile: string;
let prefixesFile: string;

// With this:
let setup: FullTestSetup; // or WorkUnitTestSetup
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
  setup = await setupFullTest('test-name'); // Use descriptive test name
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
epicsFile      → setup.epicsFile
specDir        → setup.specDir
foundationFile → setup.foundationFile

// Replace file operations:
await readFile(file, 'utf-8') → await readTextFile(file)
JSON.parse(await readFile(...)) → await readJsonTestFile(...)
await writeFile(file, JSON.stringify(...)) → await writeJsonTestFile(file, data)
await writeFile(file, content) → await writeTextFile(file, content)
```

### Step 5: Update Prefix Registration
```typescript
// Replace this:
const prefixes = JSON.parse(await readFile(prefixesFile, 'utf-8'));
prefixes.prefixes.AUTH = { description: 'Authentication features' };
await writeFile(prefixesFile, JSON.stringify(prefixes, null, 2));

// With this:
import { registerTestPrefix } from '../../test-helpers/work-unit-test-fixtures';
await registerTestPrefix(setup.testDir, 'AUTH', 'Authentication features');
```

## Common Migration Issues & Solutions

### Issue 1: "setup is not defined" Error
**Problem**: Mixed old/new variable references
```typescript
// Wrong:
await someCommand({ cwd: testDir }); // testDir undefined

// Right:
await someCommand({ cwd: setup.testDir });
```

### Issue 2: Missing Imports
**Problem**: Forgot to import required utilities
```typescript
// Add missing imports:
import { join } from 'path'; // If you still use join()
import { registerTestPrefix } from '../../test-helpers/work-unit-test-fixtures';
```

### Issue 3: File Operation Errors
**Problem**: Using old file operation patterns
```typescript
// Wrong:
const data = JSON.parse(await readFile(setup.workUnitsFile, 'utf-8'));

// Right:
const data = await readJsonTestFile(setup.workUnitsFile);
```

### Issue 4: Import Path Errors
```typescript
// Adjust relative paths based on file location:

// For files in src/commands/__tests__/:
import { setupFullTest } from '../../test-helpers/universal-test-setup';

// For files in src/test/:
import { setupFullTest } from '../test-helpers/universal-test-setup';

// For files in src/tui/__tests__/:
import { setupFullTest } from '../../test-helpers/universal-test-setup';
```

## Migration Status

### ✅ COMPLETED (315 tests):
- ✅ `src/test/system-reminder-preservation.test.ts` (5 tests)
- ✅ `src/test/fspec-session-interception.test.ts` (3 tests)  
- ✅ `src/commands/__tests__/work-unit.test.ts` (27 tests)
- ✅ `src/commands/__tests__/kanban-workflow.test.ts` (30 tests)
- ✅ `src/commands/__tests__/update-work-unit-status-done-sorting.test.ts` (6 tests)
- ✅ `src/commands/__tests__/list-tags-ensure.test.ts` (1 test)
- ✅ `src/commands/__tests__/event-storm-duplicate-detection.test.ts` (3 tests)
- ✅ `src/commands/__tests__/answer-question-preserves-text.test.ts` (3 tests)
- ✅ `src/commands/__tests__/query-work-units-advanced.test.ts` (4 tests)
- ✅ `src/commands/__tests__/update-work-unit-ensure.test.ts` (1 test)
- ✅ `src/commands/__tests__/generate-coverage-update-existing.test.ts` (2 tests)
- ✅ `src/commands/__tests__/query-dependency-stats.test.ts` (1 test)
- ✅ `src/commands/__tests__/show-event-storm.test.ts` (4 tests)
- ✅ `src/commands/__tests__/event-storm-skip-example-generation.test.ts` (3 tests)
- ✅ `src/commands/__tests__/show-epic.test.ts` (3 tests)
- ✅ `src/commands/__tests__/research-auto-attachment.test.ts` (5 tests)
- ✅ `src/commands/__tests__/conversational-review-prompt-before-done.test.ts` (5 tests)
- ✅ `src/commands/__tests__/review-agent-agnostic.test.ts` (4 tests)
- ✅ `src/commands/__tests__/update-work-unit-status-step-validation.test.ts` (4 tests)
- ✅ `src/commands/__tests__/virtual-hook-commands.test.ts` (9 tests)
- ✅ `src/commands/__tests__/add-foundation-bounded-context.test.ts` (4 tests)
- ✅ `src/commands/__tests__/generate-scenarios-naming.test.ts` (3 tests)
- ✅ `src/commands/__tests__/board.test.ts` (3 tests)
- ✅ `src/commands/__tests__/impact-analysis.test.ts` (1 test)
- ✅ `src/commands/__tests__/attachment-support.test.ts` (9 tests)
- ✅ `src/migrations/__tests__/migration-system.test.ts` (10 tests)
- ✅ `src/tui/components/__tests__/BoardView-exit-confirmation.test.tsx` (4 tests)
- ✅ `src/utils/__tests__/search-scenarios-bug-059.test.ts` (5 tests)
- ✅ `src/utils/__tests__/enhanced-research-tool-reminders.test.ts` (7 tests)
- ✅ `src/utils/__tests__/foundation-check.test.ts` (6 tests)
- ✅ `src/utils/__tests__/config-resolution.test.ts` (5 tests)
- ✅ `src/utils/__tests__/git-checkpoint-deleted-files.test.ts` (4 tests)
- ✅ `src/utils/__tests__/coverage-file-synchronization.test.ts` (6 tests)
- ✅ `src/utils/__tests__/system-reminder-research-tools.test.ts` (2 tests)
- ✅ `src/utils/__tests__/ensure-files.test.ts` (9 tests)
- ✅ `src/commands/__tests__/validate.test.ts` (16 tests)
- ✅ `src/commands/__tests__/list-features.test.ts` (10 tests)
- ✅ `src/commands/__tests__/create-feature.test.ts` (9 tests)
- ✅ `src/commands/__tests__/list-prefixes.test.ts` (3 tests)
- ✅ `src/commands/__tests__/format.test.ts` (17 tests)
- ✅ `src/commands/__tests__/query-orphans.test.ts` (1 test)
- ✅ `src/commands/__tests__/init-bundling.test.ts` (9 tests)
- ✅ `src/commands/__tests__/prevent-starting-blocked-work.test.ts` (3 tests)
- ✅ `src/commands/__tests__/hotspot-question-transformation.test.ts` (3 tests)
- ✅ `src/commands/__tests__/research-tool-visibility.integration.test.ts` (5 tests)
- ✅ `src/commands/__tests__/add-attachment-mermaid-validation.test.ts` (4 tests)
- ✅ `src/commands/__tests__/acdd-workflow-integration.test.ts` (3 tests)
- ✅ `src/commands/__tests__/architecture-notes-example-mapping.test.ts` (5 tests)
- ✅ `src/commands/__tests__/research-listing.test.ts` (4 tests)
- ✅ `src/commands/__tests__/dependency-bidirectional.test.ts` (2 tests)
- ✅ `src/commands/__tests__/virtual-hooks-reminders.test.ts` (5 tests)
- ✅ `src/commands/__tests__/validate.test.ts` (16 tests)
- ✅ `src/commands/__tests__/list-features.test.ts` (10 tests)
- ✅ `src/commands/__tests__/create-feature.test.ts` (9 tests)
- ✅ `src/commands/__tests__/list-prefixes.test.ts` (3 tests)
- ✅ `src/commands/__tests__/format.test.ts` (17 tests)
- ✅ `src/commands/__tests__/query-orphans.test.ts` (1 test)
- ✅ `src/commands/__tests__/init-bundling.test.ts` (9 tests)
- ✅ `src/commands/__tests__/prevent-starting-blocked-work.test.ts` (3 tests)
- ✅ `src/commands/__tests__/hotspot-question-transformation.test.ts` (3 tests)

### 🔄 STILL NEEDS MIGRATION (~84 files with ~160+ tests):

**High Priority (Command tests):**
- `src/commands/__tests__/validate.test.ts`
- `src/commands/__tests__/create-story.test.ts`
- `src/commands/__tests__/update-work-unit-status.test.ts`
- All other files in `/src/commands/__tests__/` that use `mkdtemp`

**Medium Priority:**
- Files in `/src/utils/__tests__/`
- Files in `/src/hooks/__tests__/`
- Files in `/src/tui/__tests__/` (some may already be migrated)

**To Find Remaining Files:**
```bash
# Find all test files still using legacy pattern:
grep -r "mkdtemp" src --include="*.test.ts" --include="*.test.tsx"

# Count remaining files:
grep -r "mkdtemp" src --include="*.test.ts" --include="*.test.tsx" | wc -l
```

## Testing Migration

After migrating a test file:

1. **Run the specific test**: `npm test -- path/to/test.test.ts`
2. **Check it uses temp directories**: Look for paths starting with OS temp dir  
3. **Verify cleanup**: No temp files should remain after test completion
4. **Check test logic**: All assertions should still pass
5. **Look for migration markers**: Tests should show temp directory creation in output

## Benefits After Migration

1. **No more manual file setup** - handled by utilities
2. **Consistent test patterns** - easier to understand and maintain
3. **Proper cleanup** - no leftover temp files
4. **Type safety** - setup objects provide typed file paths
5. **DRY principle** - no code duplication across test files
6. **Fast test execution** - isolated temp directories per test
7. **Easy to extend** - add new file types to setup utilities as needed
8. **Better debugging** - clearer separation between test setup and logic