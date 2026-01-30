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

### ✅ COMPLETED:
**Total migrated files**: All test files using the universal test setup utilities (`setupTestDirectory`, `setupWorkUnitTest`, `setupFoundationTest`, `setupFullTest`)

Files successfully using shared test setup include:
- All files importing from `'../../test-helpers/universal-test-setup'`
- All files using the standardized setup/cleanup pattern
- Tests that follow the DRY principle for temporary directory management

### 🔄 STILL NEEDS MIGRATION (61 files):

**High Priority (Command tests - 53 files):**
- `src/commands/__tests__/add-dependency-auto-block.test.ts`
- `src/commands/__tests__/add-example.test.ts`
- `src/commands/__tests__/audit-coverage.test.ts`
- `src/commands/__tests__/auto-checkpoint-cleanup.test.ts`
- `src/commands/__tests__/auto-checkpoint-on-status-transition.test.ts`
- `src/commands/__tests__/bug-078-dry-solid-work-unit-creation.test.ts`
- `src/commands/__tests__/create-feature-system-reminder.test.ts`
- `src/commands/__tests__/critical-path.test.ts`
- `src/commands/__tests__/dependencies.test.ts`
- `src/commands/__tests__/dependency-graph.test.ts`
- `src/commands/__tests__/draft-driven-discovery-feedback-loop.test.ts`
- `src/commands/__tests__/estimation-timing-clarification.test.ts`
- `src/commands/__tests__/example-mapping.test.ts`
- `src/commands/__tests__/generate-coverage.test.ts`
- `src/commands/__tests__/generate-example-mapping-from-event-storm-bug-092.test.ts`
- `src/commands/__tests__/generate-example-mapping-from-event-storm-exmap-014.test.ts`
- `src/commands/__tests__/generate-scenarios-bug-naming.test.ts`
- `src/commands/__tests__/generate-scenarios-comment-embedding.test.ts`
- `src/commands/__tests__/generate-scenarios-tag-placement.test.ts`
- `src/commands/__tests__/init-codex-home-directory.test.ts`
- `src/commands/__tests__/link-coverage.test.ts`
- `src/commands/__tests__/list-tags.test.ts`
- `src/commands/__tests__/list-work-units.test.ts`
- `src/commands/__tests__/parent-work-unit-validation.test.ts`
- `src/commands/__tests__/pm-remaining.test.ts`
- `src/commands/__tests__/prefill-workflow-blocking.test.ts`
- `src/commands/__tests__/preserve-comments-in-commands.test.ts`
- `src/commands/__tests__/query-blocked-work-units.test.ts`
- `src/commands/__tests__/query-bottlenecks.test.ts`
- `src/commands/__tests__/query-work-units-blocked-by.test.ts`
- `src/commands/__tests__/register-tag-ensure.test.ts`
- `src/commands/__tests__/register-tag.test.ts`
- `src/commands/__tests__/remove-question-display-bug.test.ts`
- `src/commands/__tests__/report-bug-to-github.test.ts`
- `src/commands/__tests__/research-error-handling.test.ts`
- `src/commands/__tests__/restore-checkpoint-terminology.test.ts`
- `src/commands/__tests__/review-ai-driven.test.ts`
- `src/commands/__tests__/scenario-deduplication.test.ts`
- `src/commands/__tests__/set-user-story.test.ts`
- `src/commands/__tests__/show-coverage.test.ts`
- `src/commands/__tests__/show-work-unit-dependencies.test.ts`
- `src/commands/__tests__/skip-step-validation-enforcement.test.ts`
- `src/commands/__tests__/stable-question-indices.test.ts`
- `src/commands/__tests__/suggest-dependencies.test.ts`
- `src/commands/__tests__/system-reminder-consolidation.test.ts`
- `src/commands/__tests__/unlink-coverage.test.ts`
- `src/commands/__tests__/update-work-unit-status-coverage-validation.test.ts`
- `src/commands/__tests__/update-work-unit-status-done-sorting.test.ts`
- `src/commands/__tests__/update-work-unit-status-one-to-one-enforcement.test.ts`
- `src/commands/__tests__/update-work-unit-status-val-005-per-feature.test.ts`
- `src/commands/__tests__/validate-tags-scenario-level.test.ts`
- `src/commands/__tests__/validate-tags-work-unit-placement.test.ts`
- `src/commands/__tests__/validate-tags.test.ts`
- `src/commands/__tests__/work-unit.test.ts`

**Medium Priority (Utils/Hooks/Other - 8 files):**
- `src/git/__tests__/diff-binary-and-truncation.test.ts`
- `src/hooks/__tests__/command-utils.test.ts`
- `src/hooks/__tests__/git-context.test.ts`
- `src/hooks/__tests__/script-generation.test.ts`
- `src/hooks/__tests__/virtual-hook-execution.test.ts`
- `src/research-tools/__tests__/registry-config-status.test.ts`
- `src/tui/__tests__/bug-065-checkpoint-integration.test.ts`
- `src/utils/__tests__/git-checkpoint-restore-deletes-new-files.test.ts`
- `src/utils/__tests__/provider-configuration-and-credentials-management.test.ts`

**To Find Remaining Files:**
```bash
# Find all test files still using legacy pattern:
grep -r "mkdtemp\|mkdirSync.*tmp.*\|mkdtempSync" src --include="*.test.ts" --include="*.test.tsx"

# Count remaining files:
grep -r "mkdtemp\|mkdirSync.*tmp.*\|mkdtempSync" src --include="*.test.ts" --include="*.test.tsx" | wc -l

# Find files that use manual filesystem operations but NOT the shared setup:
find src -name "*.test.ts" -exec bash -c 'grep -q "mkdtemp\|mkdirSync.*tmp.*\|mkdtempSync" "$1" && ! grep -q "setupTestDirectory\|setupWorkUnitTest\|setupFoundationTest\|setupFullTest" "$1" && echo "$1"' _ {} \;
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