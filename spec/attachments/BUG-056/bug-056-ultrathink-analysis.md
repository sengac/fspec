# ULTRATHINK Analysis: Work Unit ID Reuse Bug (BUG-056)

## Executive Summary

Work unit IDs (e.g., BUG-055, BUG-056) are being reused after deletion, violating the immutability principle. When BUG-055 and BUG-056 were deleted in commit `5f7885e`, the next bug created got ID BUG-055 instead of BUG-057. This contradicts fspec's stable ID philosophy and creates ambiguity in git history, documentation, and human communication.

## Problem Statement

### What Happened

1. **Original BUG-055 & BUG-056**: Created to track test failures for deprecated BOARD-003 functionality
2. **Deleted**: Both removed in commit `5f7885e` (2025-11-02 16:27:38)
3. **Reused**: New BUG-055 created in commit `646c347` (2025-11-02 16:30:14) for a **completely different issue** (attachment file duplication)

### Expected Behavior

Work unit IDs should **NEVER** be reused, even after deletion. The ID counter for each prefix should maintain a "high water mark" that only increases, similar to database auto-increment primary keys.

**Correct sequence after deletion:**
- BUG-001 through BUG-056 created (high water mark: 56)
- BUG-055 and BUG-056 deleted (high water mark: still 56)
- Next bug gets ID: **BUG-057** ✓

### Actual Behavior

The ID generation logic recalculates the next ID by scanning **currently existing** work units, ignoring deleted ones.

**Incorrect sequence (current bug):**
- BUG-001 through BUG-056 created (max existing ID: 56)
- BUG-055 and BUG-056 deleted (max existing ID: 54)
- Next bug gets ID: **BUG-055** ❌

## Root Cause Analysis

### The Buggy Code

Located in THREE files (identical implementation):
- `src/commands/create-bug.ts:169-177`
- `src/commands/create-story.ts:168-176`
- `src/commands/create-task.ts:164-172`

```typescript
function generateNextId(workUnitsData: WorkUnitsData, prefix: string): string {
  const existingIds = Object.keys(workUnitsData.workUnits)  // ❌ PROBLEM LINE
    .filter(id => id.startsWith(`${prefix}-`))
    .map(id => parseInt(id.split('-')[1]))
    .filter(num => !isNaN(num));

  const nextNumber =
    existingIds.length === 0 ? 1 : Math.max(...existingIds) + 1;
  return `${prefix}-${String(nextNumber).padStart(3, '0')}`;
}
```

**The Problem:**
- Line 1: `Object.keys(workUnitsData.workUnits)` only returns **currently existing** work units
- When a work unit is deleted, it's removed from `workUnitsData.workUnits`
- The deleted ID is no longer considered when calculating `Math.max(...existingIds)`
- Result: ID counter "rewinds" to the highest remaining ID

### Why This Is Critical

**1. Historical Ambiguity**
- Git history contains commits referencing "BUG-055" (old bug)
- New commits also reference "BUG-055" (new bug)
- Future developers: "Which BUG-055 are we talking about?"

**2. Documentation Confusion**
- Feature files, test files, or markdown docs may mention "BUG-055"
- After deletion and reuse, these references become ambiguous

**3. Communication Breakdown**
- Human conversation: "Remember BUG-055?"
- Which one? The deleted one or the new one?

**4. Breaks Stable ID Philosophy**
- IDX-001 and IDX-002 established stable indices for items **within** work units
- Work unit IDs themselves should follow the same principle
- Inconsistency: items within work units have stable IDs, but work units themselves don't

## Technical Deep Dive

### Parallel with Stable Indices (IDX-001, IDX-002)

fspec already solved this problem for items WITHIN work units:

**From `src/types/index.ts:73-77`:**
```typescript
export interface WorkUnit {
  // ... other fields
  nextRuleId?: number;      // High water mark for rules
  nextExampleId?: number;   // High water mark for examples
  nextQuestionId?: number;  // High water mark for questions
  nextNoteId?: number;      // High water mark for architecture notes
}
```

These counters:
- Auto-increment from 0
- **Never decrease**, even when items are soft-deleted
- Stored directly in each work unit

**We need the same pattern for work unit IDs.**

### Current Data Structure

**From `src/types/index.ts:90-97`:**
```typescript
export interface WorkUnitsData {
  meta?: {
    version: string;
    lastUpdated: string;
  };
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}
```

**Missing:** No field to track high water marks per prefix!

## Solution Design

### Proposed Changes

**1. Extend WorkUnitsData Type**

```typescript
export interface WorkUnitsData {
  meta?: {
    version: string;
    lastUpdated: string;
  };
  prefixCounters?: Record<string, number>; // ← ADD THIS
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}
```

**2. Update ID Generation Logic**

```typescript
function generateNextId(
  workUnitsData: WorkUnitsData,
  prefix: string
): string {
  // Initialize prefixCounters if missing (backward compatibility)
  if (!workUnitsData.prefixCounters) {
    workUnitsData.prefixCounters = {};
  }

  // Get high water mark from prefixCounters (if exists)
  const storedHighWaterMark = workUnitsData.prefixCounters[prefix] || 0;

  // Calculate high water mark from existing IDs (for backward compat)
  const existingIds = Object.keys(workUnitsData.workUnits)
    .filter(id => id.startsWith(`${prefix}-`))
    .map(id => parseInt(id.split('-')[1]))
    .filter(num => !isNaN(num));

  const calculatedHighWaterMark =
    existingIds.length === 0 ? 0 : Math.max(...existingIds);

  // Use the maximum of both (handles migration case)
  const highWaterMark = Math.max(storedHighWaterMark, calculatedHighWaterMark);

  // Next number is always high water mark + 1
  const nextNumber = highWaterMark + 1;

  // Update the high water mark in prefixCounters
  workUnitsData.prefixCounters[prefix] = nextNumber;

  return `${prefix}-${String(nextNumber).padStart(3, '0')}`;
}
```

**3. Update All Three Commands**

Apply the fix to:
- `src/commands/create-bug.ts`
- `src/commands/create-story.ts`
- `src/commands/create-task.ts`

**4. Create Migration**

Add migration to initialize `prefixCounters` for existing projects:

```typescript
// src/migrations/prefix-counters.ts
export async function migratePrefixCounters(
  workUnitsData: WorkUnitsData
): Promise<WorkUnitsData> {
  if (!workUnitsData.prefixCounters) {
    workUnitsData.prefixCounters = {};
  }

  // Calculate high water marks from all existing IDs
  const prefixMap = new Map<string, number>();

  for (const workUnitId of Object.keys(workUnitsData.workUnits)) {
    const match = workUnitId.match(/^([A-Z]+)-(\d+)$/);
    if (match) {
      const prefix = match[1];
      const number = parseInt(match[2]);
      const current = prefixMap.get(prefix) || 0;
      prefixMap.set(prefix, Math.max(current, number));
    }
  }

  // Store high water marks
  for (const [prefix, highWaterMark] of prefixMap.entries()) {
    workUnitsData.prefixCounters[prefix] = highWaterMark;
  }

  return workUnitsData;
}
```

**5. Update Deletion Commands**

Deletion commands (if they exist) should **NOT** modify `prefixCounters`. The high water mark should remain unchanged when work units are deleted.

### Migration Strategy

**Why backward compatibility is critical:**
- Existing projects have work-units.json without `prefixCounters`
- Migration must calculate correct high water marks from existing IDs
- Must handle edge case: deleted IDs in git history (not in current file)

**Migration approach:**
1. Check if `prefixCounters` field exists
2. If missing, scan all work unit IDs and calculate max per prefix
3. Store in `prefixCounters`
4. Going forward, `generateNextId` maintains the high water mark

**Edge case handling:**
- If user manually deleted BUG-055 and BUG-056 from JSON without using a command
- Migration only sees BUG-054 as highest
- Next bug would get BUG-055 (same bug!)
- **Solution**: Documentation should strongly discourage manual JSON editing
- **Better solution**: Add `fspec delete-work-unit` command that preserves counters

## Testing Strategy

### Unit Tests

**Test cases to add:**

1. **ID generation with empty work units**
   - No work units exist
   - Next ID should be PREFIX-001

2. **ID generation with existing work units**
   - BUG-001, BUG-002, BUG-003 exist
   - Next ID should be BUG-004

3. **ID generation after deletion** (THE KEY TEST)
   - BUG-001, BUG-002, BUG-003 exist (high water mark: 3)
   - Delete BUG-003
   - Next ID should be BUG-004 (NOT BUG-003)

4. **Migration from old format**
   - Load work-units.json without `prefixCounters`
   - Run migration
   - Verify `prefixCounters` has correct high water marks

5. **Multiple prefixes**
   - BUG-005, FEAT-010, TASK-003 exist
   - Next BUG should be BUG-006
   - Next FEAT should be FEAT-011
   - Next TASK should be TASK-004

### Integration Tests

1. **End-to-end workflow**
   - Create BUG-001, BUG-002, BUG-003
   - Delete BUG-002 and BUG-003
   - Create new bug
   - Verify ID is BUG-004

2. **Concurrent creation** (if supported)
   - Multiple processes creating bugs simultaneously
   - Verify no ID collisions
   - (Requires file locking from LOCK-002)

## Related Work

### IDX-001: Stable Indices Implementation
- Established pattern for stable IDs within work units
- Items have `id` field that never decreases
- Soft-delete with `deleted: true` flag
- This bug fix extends that pattern to work unit IDs themselves

### LOCK-002: File Locking System
- Uses `fileManager.transaction()` for atomic writes
- Prevents race conditions when multiple processes write work-units.json
- Should work with `prefixCounters` updates (already uses transactions)

### MIG-001: Build Migration System
- Provides infrastructure for running migrations
- Can be used to add prefix-counters migration
- Version bumping strategy already established

## Impact Assessment

### Severity: **HIGH**

**Reasons:**
1. Violates core fspec principle (stable IDs)
2. Creates ambiguity in git history (hard to debug later)
3. Affects all three work unit types (bug, story, task)
4. Will get worse over time (more deletions = more reuse)

### Affected Components

- ✓ `create-bug.ts` - Uses buggy `generateNextId`
- ✓ `create-story.ts` - Uses buggy `generateNextId`
- ✓ `create-task.ts` - Uses buggy `generateNextId`
- ? Deletion commands (if they exist) - Should not touch counters
- ✓ `src/types/index.ts` - Needs `prefixCounters` field
- ✓ Migration system - Needs new migration
- ✓ Tests - Need comprehensive test coverage

### Risk of Breaking Changes

**Low risk** if migration is done correctly:
- New field `prefixCounters` is optional (backward compat)
- Old files without `prefixCounters` handled via migration
- Existing work unit IDs unchanged (only affects NEW IDs)

## Implementation Checklist

- [ ] Update `WorkUnitsData` type in `src/types/index.ts`
- [ ] Refactor `generateNextId` in `create-bug.ts`
- [ ] Refactor `generateNextId` in `create-story.ts`
- [ ] Refactor `generateNextId` in `create-task.ts`
- [ ] Create migration `prefix-counters.ts`
- [ ] Register migration in migration system
- [ ] Add unit tests for ID generation with deletion
- [ ] Add integration tests for end-to-end workflow
- [ ] Update JSON schema (if exists) to include `prefixCounters`
- [ ] Update documentation about stable IDs
- [ ] Consider adding `fspec delete-work-unit` command (future enhancement)

## Alternative Solutions Considered

### Alternative 1: Never Delete Work Units
- Mark as deleted with `deleted: true` flag (like items within work units)
- Keep in `workUnitsData.workUnits` forever
- **Pros**: Simple, no counter needed
- **Cons**: `work-units.json` grows forever, pollutes queries

### Alternative 2: Store Deleted IDs
- Keep array of deleted IDs: `deletedWorkUnitIds: string[]`
- Check both existing and deleted when generating IDs
- **Pros**: Explicit tracking of deleted IDs
- **Cons**: Array grows forever, more complex logic

### Alternative 3: High Water Mark (CHOSEN)
- Store `prefixCounters: Record<string, number>`
- Simple integer per prefix
- **Pros**: Clean, minimal storage, matches IDX-001 pattern
- **Cons**: Requires migration

## Conclusion

This bug violates fspec's stable ID principle and will cause increasing confusion as more work units are deleted. The fix is straightforward: add `prefixCounters` to track high water marks per prefix, similar to how `nextRuleId`, `nextExampleId`, etc. work for items within work units.

The solution requires:
1. Type extension (`WorkUnitsData`)
2. Logic update (`generateNextId` in 3 files)
3. Migration (calculate initial high water marks)
4. Tests (ID generation with deletion)

**Estimated complexity**: 3-5 story points
- Type changes: trivial
- Logic changes: straightforward (3 files)
- Migration: moderate (need to scan all IDs)
- Tests: comprehensive (multiple scenarios)

**Priority**: High (data integrity issue)

---

*Generated for BUG-056 on 2025-11-02*
