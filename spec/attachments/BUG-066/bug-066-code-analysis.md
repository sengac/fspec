# BUG-066: Auto-Checkpoints Not Cleaned Up - Code Analysis

## Executive Summary

When a work unit transitions to `done` status, automatic checkpoints should be deleted but manual checkpoints preserved. Currently, **no checkpoints are deleted at all** due to three distinct bugs in the implementation.

## Problem Statement

**Expected Behavior:**
- When work unit moves to `done` status
- Delete ALL auto-checkpoints for that work unit (pattern: `{workUnitId}-auto-{state}`)
- Preserve ALL manual checkpoints (custom names)

**Current Behavior:**
- No checkpoints are deleted (automatic or manual)
- Checkpoints accumulate indefinitely, causing repository bloat

## Root Cause Analysis

### Bug #1: Stub Implementation (CRITICAL)

**Location:** `src/utils/git-checkpoint.ts:494-523`

**Issue:** The `cleanupCheckpoints()` function is a **non-functional stub**

```typescript
export async function cleanupCheckpoints(
  workUnitId: string,
  cwd: string,
  keepLast: number
): Promise<{...}> {
  const checkpoints = await listCheckpoints(workUnitId, cwd);

  // Sort by timestamp (newest first)
  checkpoints.sort((a, b) => {
    return new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime();
  });

  const preserved = checkpoints.slice(0, keepLast);
  const deleted = checkpoints.slice(keepLast);

  // In real implementation, would delete the old stash entries
  // For now, just return the split  ← THIS IS THE PROBLEM!

  return {
    deletedCount: deleted.length,
    preservedCount: preserved.length,
    deleted,
    preserved,
  };
}
```

**Impact:** Function calculates what would be deleted but **doesn't actually delete anything**.

### Bug #2: Missing Integration (CRITICAL)

**Location:** `src/commands/update-work-unit-status.ts:475-489`

**Issue:** Checkpoint cleanup is **never called** when work unit moves to `done`

```typescript
// BUG FIX (IDX-002): Auto-compact when moving to done status
if (newStatus === 'done') {
  await compactWorkUnit({
    workUnitId: options.workUnitId,
    force: true,
    cwd,
  });

  // Re-read work units data after compaction
  workUnitsData = await ensureWorkUnitsFile(cwd);
  workUnit = workUnitsData.workUnits[options.workUnitId];

  // ← NO CHECKPOINT CLEANUP HAPPENS HERE!
}
```

**Impact:** Even if the cleanup function worked, it would never be invoked.

### Bug #3: Design Flaw (MODERATE)

**Location:** `src/utils/git-checkpoint.ts:494` (function signature)

**Issue:** Current API uses `keepLast: number` parameter which would delete BOTH manual and automatic checkpoints

```typescript
export async function cleanupCheckpoints(
  workUnitId: string,
  cwd: string,
  keepLast: number  ← WRONG: Should filter by isAutomatic flag instead
)
```

**Impact:** If implemented as-is, would incorrectly delete manual checkpoints.

## Existing Infrastructure (Can Leverage)

### ✅ Checkpoint Identification Already Works

**Location:** `src/utils/git-checkpoint.ts:465-467`

```typescript
const isAutomatic = parsed.checkpointName.startsWith(
  `${workUnitId}-auto-`
);
```

The `listCheckpoints()` function correctly identifies automatic checkpoints:
- Returns array of `Checkpoint` objects
- Each has `isAutomatic: boolean` field
- Auto-checkpoints have pattern: `{workUnitId}-auto-{fromState}`
- Examples: `AUTH-001-auto-specifying`, `BUG-027-auto-testing`

### ✅ Checkpoint Storage Locations Known

**Git Refs:**
- Path: `.git/refs/fspec-checkpoints/{workUnitId}/{checkpointName}`
- Each checkpoint stored as a git reference
- Can be deleted using `fs.unlink(refPath)`

**Index Files:**
- Path: `.git/fspec-checkpoints-index/{workUnitId}.json`
- JSON structure:
  ```json
  {
    "checkpoints": [
      { "name": "checkpoint-name", "message": "fspec-checkpoint:..." }
    ]
  }
  ```
- Needs to be updated to remove deleted checkpoint entries

## Proposed Fix Strategy

### Step 1: Create New Function `cleanupAutoCheckpoints()`

**Location:** `src/utils/git-checkpoint.ts` (add new export)

**Signature:**
```typescript
export async function cleanupAutoCheckpoints(
  workUnitId: string,
  cwd: string
): Promise<{
  deletedCount: number;
  deletedCheckpoints: string[];
}>
```

**Implementation:**
1. Call `listCheckpoints(workUnitId, cwd)` to get all checkpoints
2. Filter where `isAutomatic === true`
3. For each auto-checkpoint:
   - Delete git ref file: `.git/refs/fspec-checkpoints/{workUnitId}/{checkpointName}`
   - Remove entry from index file: `.git/fspec-checkpoints-index/{workUnitId}.json`
4. Return count and names of deleted checkpoints

### Step 2: Integrate Into `update-work-unit-status.ts`

**Location:** `src/commands/update-work-unit-status.ts:489` (after auto-compact)

**Code:**
```typescript
if (newStatus === 'done') {
  await compactWorkUnit({
    workUnitId: options.workUnitId,
    force: true,
    cwd,
  });

  // Re-read work units data after compaction
  workUnitsData = await ensureWorkUnitsFile(cwd);
  workUnit = workUnitsData.workUnits[options.workUnitId];

  // ADDITION: Cleanup auto-checkpoints for completed work unit
  try {
    const cleanupResult = await gitCheckpoint.cleanupAutoCheckpoints(
      options.workUnitId,
      cwd
    );

    if (cleanupResult.deletedCount > 0) {
      console.log(
        chalk.gray(
          `🧹 Auto-cleanup: ${cleanupResult.deletedCount} checkpoint(s) deleted`
        )
      );
    }
  } catch (error) {
    // Silently skip cleanup if git operations fail
    if (process.env.DEBUG) {
      console.warn(chalk.yellow(`⚠️ Checkpoint cleanup skipped: ${error.message}`));
    }
  }
}
```

### Step 3: Deprecate Old `cleanupCheckpoints()` (Optional)

**Options:**
1. **Keep it** - Still useful for manual cleanup with `fspec cleanup-checkpoints --keep-last N`
2. **Fix the stub** - Implement actual deletion logic
3. **Mark deprecated** - Add JSDoc comment pointing to new function

## Test Coverage Needed

### Test Scenarios:

1. **Work unit with only auto-checkpoints**
   - Given: Work unit has 3 auto-checkpoints
   - When: Move to done
   - Then: All 3 auto-checkpoints deleted

2. **Work unit with only manual checkpoints**
   - Given: Work unit has 2 manual checkpoints
   - When: Move to done
   - Then: No checkpoints deleted (all preserved)

3. **Work unit with mixed checkpoints**
   - Given: Work unit has 2 auto + 1 manual checkpoint
   - When: Move to done
   - Then: 2 auto-checkpoints deleted, 1 manual preserved

4. **Work unit with no checkpoints**
   - Given: Work unit has no checkpoints
   - When: Move to done
   - Then: No errors, cleanup succeeds silently

## Files to Modify

1. ✏️ `src/utils/git-checkpoint.ts` - Add `cleanupAutoCheckpoints()` function
2. ✏️ `src/commands/update-work-unit-status.ts` - Call cleanup after auto-compact
3. ✏️ `src/commands/__tests__/auto-checkpoint-cleanup.test.ts` - New test file

## Complexity Estimate

**Story Points:** 3-5 points

**Breakdown:**
- New function implementation: ~50 lines
- Integration into status update: ~20 lines
- Test coverage: ~150 lines
- Total: ~220 lines of code

**Risk Factors:**
- File system operations (git refs)
- JSON parsing/writing (index files)
- Error handling (git repo not initialized)
- Need to avoid breaking existing checkpoint features

## References

**Code Locations:**
- `src/utils/git-checkpoint.ts:494-523` - Stub cleanup function
- `src/utils/git-checkpoint.ts:441-489` - List checkpoints (working)
- `src/utils/git-checkpoint.ts:465-467` - Auto-checkpoint detection (working)
- `src/commands/update-work-unit-status.ts:475-489` - Auto-compact on done
- `src/commands/cleanup-checkpoints.ts` - Manual cleanup command

**Related Work Units:**
- AUTO-001: Intelligent checkpoint system (implemented)
- GIT-002: Auto-checkpoint on status transition (implemented)
- BUG-066: This bug (fixing checkpoint cleanup)
