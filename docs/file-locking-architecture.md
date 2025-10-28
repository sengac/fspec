# File Locking Architecture for fspec (LOCK-001)

## Executive Summary

This document defines the three-layer file locking architecture for fspec to prevent JSON file corruption when multiple instances run concurrently. The solution uses proper-lockfile for inter-process coordination, a readers-writer pattern for in-process optimization, and atomic write-replace for safe file modifications.

## Problem Statement

**Current State:**
- fspec has ZERO file locking mechanism
- Multiple fspec instances can run simultaneously (CLI commands + TUI board)
- All JSON files (work-units.json, tags.json, foundation.json, etc.) are vulnerable to race conditions
- Read-modify-write operations are not atomic

**Risk Scenarios:**
1. **Terminal 1**: `fspec update-work-unit-status BOARD-001 done`
2. **Terminal 2**: `fspec update-work-unit-status BOARD-002 done`
3. Both read work-units.json simultaneously
4. Both modify their respective work units
5. Both write back - LAST WRITE WINS, one update is lost

**Additional Risks:**
- TUI board refreshing while command modifies file → read during write → corrupted data
- Partial writes from crashed processes → invalid JSON
- Stale locks from killed processes → permanent deadlock

## Solution: Three-Layer Lock Architecture

### Layer 1: Inter-Process Coordination (proper-lockfile)

**Purpose:** Prevent corruption across different fspec processes

**Technology:** `proper-lockfile` npm package

**Why proper-lockfile:**
- ✅ Uses mkdir strategy (atomic on all filesystems, including network)
- ✅ Works across processes and machines
- ✅ Stale lock detection prevents deadlocks from crashed processes
- ✅ Battle-tested (used by npm, yarn, pnpm)
- ✅ Retry logic with exponential backoff
- ❌ Alternatives rejected:
  - `lockfile` package: Incomplete, deprecated
  - `ipc-mutex`: Platform-specific, more complex
  - SQLite WAL mode: Too radical, breaks human-readable JSON

**Configuration:**
```typescript
{
  stale: 10000,           // 10 seconds (detect crashed processes)
  retries: {
    retries: 10,          // Retry 10 times before failing
    minTimeout: 50,       // Start with 50ms wait
    maxTimeout: 500,      // Max 500ms wait (exponential backoff)
  },
  realpath: false,        // Faster, supports symlinks
}
```

### Layer 2: In-Process Optimization (Readers-Writer Pattern)

**Purpose:** Allow concurrent reads within same process

**Pattern:** Multiple readers OR single writer (no simultaneous read+write)

**Implementation:**
- `readLocks: Map<string, number>` - Track reader count per file
- `writeLocks: Set<string>` - Track files with active write lock
- `waitingReaders: Map<string, Array<() => void>>` - Queue for blocked readers
- `waitingWriters: Map<string, Array<() => void>>` - Queue for blocked writers

**Algorithm:**
```
acquireReadLock(path):
  while writeLock exists for path:
    wait in waitingReaders queue
  increment readLocks[path]

acquireWriteLock(path):
  while writeLock exists OR readLocks[path] > 0:
    wait in waitingWriters queue
  add path to writeLocks
```

**Benefits:**
- TUI board + `fspec list-work-units` can read concurrently
- No inter-process overhead for multiple reads in same process
- Fair scheduling: readers preferred over writers (prevents writer starvation)

### Layer 3: Atomic Write-Replace

**Purpose:** Prevent partial writes and ensure all-or-nothing file updates

**Pattern:** Write to temp file + atomic rename

**Implementation:**
```typescript
async writeJSON<T>(filePath: string, data: T): Promise<void> {
  const tempFile = `${filePath}.tmp.${randomHex()}`;

  // Step 1: Write to temp file
  await writeFile(tempFile, JSON.stringify(data, null, 2));

  // Step 2: Atomic rename (POSIX guarantees atomicity)
  await rename(tempFile, filePath);
}
```

**Why This Works:**
- POSIX: `rename()` is atomic - file appears instantly with full content
- Windows: `rename()` is near-atomic (small race window, acceptable)
- Crash during Step 1: Original file intact, temp file orphaned
- Crash during Step 2: Either old content OR new content, never partial

## API Design

### LockedFileManager Class

```typescript
class LockedFileManager {
  // Singleton instance
  private static instance: LockedFileManager;
  static getInstance(): LockedFileManager;

  // Read with concurrent access support
  async readJSON<T>(filePath: string): Promise<T>;

  // Write with exclusive access
  async writeJSON<T>(filePath: string, data: T): Promise<void>;

  // Atomic read-modify-write transaction
  async transaction<T, R>(
    filePath: string,
    operation: (data: T) => Promise<R> | R
  ): Promise<R>;
}

// Export singleton
export const fileManager = LockedFileManager.getInstance();
```

### Usage Examples

**Before (No Locking):**
```typescript
const data = await readFile('spec/work-units.json', 'utf-8');
const workUnits = JSON.parse(data);
workUnits.workUnits['BOARD-001'].status = 'done';
await writeFile('spec/work-units.json', JSON.stringify(workUnits, null, 2));
// RACE CONDITION: Another process can read/write between these operations
```

**After (With Locking):**
```typescript
await fileManager.transaction<WorkUnitsData, void>(
  'spec/work-units.json',
  async (workUnits) => {
    workUnits.workUnits['BOARD-001'].status = 'done';
    // Automatic write with full lock protection
  }
);
// SAFE: Entire read-modify-write is atomic
```

## Migration Plan

### Phase 1: Create File Manager (2-3 hours)

**Files to Create:**
1. `src/utils/file-manager.ts` - LockedFileManager class
2. `src/utils/__tests__/file-manager.test.ts` - Unit tests

**Dependencies to Add:**
```bash
npm install proper-lockfile
npm install --save-dev @types/proper-lockfile
```

### Phase 2: Refactor Utilities (1-2 hours)

**Files to Modify:**
1. `src/utils/ensure-files.ts` - Replace readFile/writeFile with fileManager
   - `ensureWorkUnitsFile()` → use `fileManager.readJSON()`
   - `ensureTagsFile()` → use `fileManager.readJSON()`
   - `ensureFoundationFile()` → use `fileManager.readJSON()`
   - `ensurePrefixesFile()` → use `fileManager.readJSON()`
   - `ensureEpicsFile()` → use `fileManager.readJSON()`

### Phase 3: Refactor Commands (3-4 hours)

**Pattern to Apply:**
Replace all read-modify-write patterns with `fileManager.transaction()`:

**Files to Modify (High Priority):**
1. `src/commands/update-work-unit-status.ts` - Most critical (state transitions)
2. `src/commands/create-story.ts` - Creates new work units
3. `src/commands/create-bug.ts` - Creates new work units
4. `src/commands/create-task.ts` - Creates new work units
5. `src/commands/update-work-unit.ts` - Modifies work units
6. `src/commands/delete-work-unit.ts` - Removes work units
7. `src/commands/register-tag.ts` - Modifies tags.json
8. `src/commands/update-foundation.ts` - Modifies foundation.json

**Estimated Files to Refactor:** 50-60 command files

**Pattern Template:**
```typescript
// OLD PATTERN
const workUnitsData = await ensureWorkUnitsFile(cwd);
// ... modify workUnitsData ...
await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));

// NEW PATTERN
await fileManager.transaction<WorkUnitsData, void>(
  workUnitsFile,
  async (workUnitsData) => {
    // ... modify workUnitsData ...
    // Data automatically written with locks
  }
);
```

### Phase 4: Refactor TUI Store (1 hour)

**File to Modify:**
1. `src/tui/store/fspecStore.ts` - Replace file reads with `fileManager.readJSON()`

**Changes:**
- `loadData()` method: Use `fileManager.readJSON()` for all file reads
- File watchers: Ensure reads go through fileManager

### Phase 5: Testing (2-3 hours)

**Test Files to Create:**
1. `src/utils/__tests__/file-manager-concurrency.test.ts` - Concurrent access tests
2. `src/utils/__tests__/file-manager-stale-locks.test.ts` - Crash recovery tests
3. `src/commands/__tests__/concurrent-commands.test.ts` - Integration tests

**Test Scenarios:**
- Concurrent reads (should succeed in parallel)
- Concurrent read + write (write should wait)
- Concurrent writes (second should wait for first)
- Process crash with active lock (stale lock detection)
- Atomic write-replace (crash during write leaves file intact)
- Lock acquisition retry logic
- Transaction rollback on error

## Performance Considerations

### Read Performance

**Baseline (No Locking):**
- Single read: ~1-2ms (file I/O + JSON parse)

**With Locking (Expected):**
- Single read: ~3-5ms (lock acquire + file I/O + JSON parse + lock release)
- Concurrent reads (same process): ~3-5ms each (parallel read locks)
- Concurrent reads (different processes): ~5-10ms each (proper-lockfile coordination)

**Optimization:**
- In-process readers-writer pattern minimizes overhead for concurrent reads
- proper-lockfile uses mkdir (fast, no polling)

### Write Performance

**Baseline (No Locking):**
- Single write: ~5-10ms (stringify + file I/O)

**With Locking (Expected):**
- Single write: ~10-20ms (lock acquire + stringify + temp write + rename + lock release)

**Acceptable Impact:**
- Commands are infrequent (user-triggered)
- Correctness > speed for file writes
- Atomic writes prevent corruption (worth the cost)

## Error Handling

### Stale Lock Detection

**Scenario:** Process crashes while holding lock

**Detection:** proper-lockfile updates lock mtime periodically, checks staleness on acquire

**Resolution:** After 10 seconds, lock is considered stale and can be forcibly acquired

**Code:**
```typescript
const release = await lockfile.lock(filePath, {
  stale: 10000, // 10 second threshold
});
```

### Lock Acquisition Failure

**Scenario:** Lock cannot be acquired after 10 retries

**Error:** Throw descriptive error with retry details

**User Action:** Retry command or check for stuck processes

**Code:**
```typescript
try {
  await fileManager.writeJSON(filePath, data);
} catch (error) {
  if (error.code === 'ELOCKED') {
    console.error('Failed to acquire file lock after retries. Check for stuck fspec processes.');
  }
  throw error;
}
```

### Temp File Cleanup

**Scenario:** Process crashes during atomic write-replace

**Detection:** Orphaned `.tmp.*` files in spec/ directory

**Cleanup:** Manual or automated cleanup script

**Future Enhancement:**
```bash
# fspec cleanup-temp-files command
find spec -name "*.tmp.*" -mtime +1 -delete
```

## Monitoring and Debugging

### Lock Metrics (Future Enhancement)

Add optional logging for lock operations:

```typescript
// Enable with environment variable
if (process.env.FSPEC_DEBUG_LOCKS) {
  console.log(`[LOCK] Acquired read lock: ${filePath}`);
  console.log(`[LOCK] Waiting for write lock: ${filePath}`);
  console.log(`[LOCK] Released lock: ${filePath}`);
}
```

### Deadlock Detection (Future Enhancement)

Add timeout for lock acquisition:

```typescript
const LOCK_TIMEOUT = 30000; // 30 seconds max wait
const timeout = setTimeout(() => {
  throw new Error(`Lock acquisition timeout after ${LOCK_TIMEOUT}ms`);
}, LOCK_TIMEOUT);

try {
  await acquireWriteLock(path);
  clearTimeout(timeout);
} catch (error) {
  clearTimeout(timeout);
  throw error;
}
```

## Acceptance Criteria (From Example Mapping)

### Business Rules

1. ✅ Three-layer architecture (proper-lockfile + readers-writer + atomic writes)
2. ✅ Multiple readers can read concurrently
3. ✅ Only one writer at a time (exclusive)
4. ✅ Atomic write-replace prevents partial writes
5. ✅ Stale lock detection (10 second timeout)
6. ✅ Retry logic with exponential backoff (10 retries, 50-500ms)
7. ✅ LockedFileManager singleton for in-process coordination
8. ✅ All JSON files use LockedFileManager
9. ✅ transaction() method for atomic read-modify-write
10. ✅ Lock release in finally blocks

### Concrete Examples

1. ✅ `fspec list-work-units` + `fspec update-work-unit-status` run simultaneously → No corruption
2. ✅ TUI board + `fspec create-story` run simultaneously → No corruption, TUI sees new work unit
3. ✅ Three concurrent reads + one write → Reads complete, write waits for exclusive access
4. ✅ Process crashes mid-write → Stale lock detected after 10 seconds, next process proceeds
5. ✅ Atomic write crashes → Original file intact (no partial writes)
6. ✅ Concurrent reads in same process → Both acquire read locks simultaneously
7. ✅ Write while read active → Writer waits for reader to finish

## Success Metrics

- ✅ Zero file corruption incidents in concurrent scenarios
- ✅ All commands complete successfully with concurrent execution
- ✅ Lock acquisition time < 500ms (99th percentile)
- ✅ Stale locks cleared within 10 seconds
- ✅ All 334+ files using readFile/writeFile migrated to fileManager

## References

- proper-lockfile: https://www.npmjs.com/package/proper-lockfile
- Readers-Writer Problem: https://en.wikipedia.org/wiki/Readers%E2%80%93writers_problem
- POSIX Atomic Operations: https://pubs.opengroup.org/onlinepubs/9699919799/functions/rename.html
- File Locking Best Practices: https://blog.logrocket.com/understanding-node-js-file-locking/

## Timeline Estimate

- **Total Implementation:** 9-13 hours
  - Phase 1 (File Manager): 2-3 hours
  - Phase 2 (Utilities): 1-2 hours
  - Phase 3 (Commands): 3-4 hours
  - Phase 4 (TUI Store): 1 hour
  - Phase 5 (Testing): 2-3 hours

- **Testing & Validation:** 2-3 hours
- **Documentation:** 1 hour
- **Total:** 12-17 hours

## Risks and Mitigation

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|-----------|
| Performance degradation | Medium | Low | In-process readers-writer pattern minimizes overhead |
| Stale locks from crashes | High | Medium | 10-second stale detection, auto-cleanup |
| Migration breaks existing code | High | Low | Comprehensive testing, gradual rollout |
| Windows rename not fully atomic | Medium | Low | Acceptable risk, rare edge case |
| Lock acquisition timeouts | Medium | Low | Retry logic with exponential backoff |

## Conclusion

This three-layer file locking architecture provides robust concurrent access protection for fspec's JSON files while maintaining acceptable performance. The combination of proper-lockfile (inter-process), readers-writer pattern (in-process), and atomic write-replace (crash safety) addresses all identified race conditions and corruption scenarios.

**Next Steps:**
1. Generate scenarios from Example Mapping: `fspec generate-scenarios LOCK-001`
2. Move to testing: `fspec update-work-unit-status LOCK-001 testing`
3. Implement Phase 1: Create LockedFileManager class
4. Implement Phase 2: Refactor utilities
5. Implement Phase 3: Refactor commands
6. Implement Phase 4: Refactor TUI store
7. Implement Phase 5: Add comprehensive tests
8. Validate with concurrent execution scenarios
9. Move to done: `fspec update-work-unit-status LOCK-001 done`
