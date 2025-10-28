# Lock Lifecycle Analysis - WHERE and WHEN to Lock

## Executive Summary

**The Problem:** Race conditions occur during read-modify-write operations on JSON files when multiple fspec instances run concurrently.

**The Solution:** Three-layer locking architecture with centralized lock management in `LockedFileManager` singleton.

**The Scope:** 66 files use ensure* functions, ~50 commands need refactoring, TUI store needs read lock integration.

---

## 1. FILES THAT NEED LOCKING

All JSON files in `spec/` directory:

| File | Contention Level | Primary Operations | Notes |
|------|------------------|-------------------|-------|
| `work-units.json` | **HIGHEST** | 66 files use ensureWorkUnitsFile() | 230+ work units, most commands modify this |
| `tags.json` | Medium | tag commands (register, update, delete) | Modified when tags change |
| `foundation.json` | Low | foundation commands (update, add-diagram) | Infrequent updates |
| `prefixes.json` | Low | prefix commands (create, update) | Infrequent updates |
| `epics.json` | Medium | epic commands (create, update) | Modified when epics change |
| `fspec-hooks.json` | Low | hook commands (add, remove) | Infrequent updates |
| `example-map.json` | Medium | Example Mapping commands | Export/import operations |

---

## 2. OPERATION TYPES AND LOCK PATTERNS

### Type 1: READ-ONLY Operations

**Pattern:**
```typescript
// Current (no locks)
const data = await ensureWorkUnitsFile(cwd);
// Use data in memory (no write back)
```

**Examples:**
- `list-work-units` - Lists all work units
- `show-work-unit` - Shows single work unit details
- `display-board` - Renders Kanban board
- TUI store `loadData()` - Refreshes every 2 seconds

**Lock Needed:** READ lock (shared - multiple concurrent readers allowed)

**Lock Lifecycle:**
```
ACQUIRE READ LOCK
  ├─ Call fs.readFile()
  ├─ Parse JSON
  └─ Store in memory
RELEASE READ LOCK ← immediately after parse, data is in memory now
```

**Why Release Early?**
- Data is in memory, no longer accessing file
- Allows other readers to proceed
- Allows writers to proceed when no readers remain

**Refactored Pattern:**
```typescript
// New (with locks)
const data = await fileManager.readJSON<WorkUnitsData>(
  join(specPath, 'work-units.json'),
  defaultWorkUnitsData
);
// Lock acquired before read, released after parse
// LockedFileManager handles acquisition/release internally
```

---

### Type 2: READ-MODIFY-WRITE Operations

**Pattern:**
```typescript
// Current (no locks - RACE CONDITION!)
const data = await ensureWorkUnitsFile(cwd);        // READ
data.workUnits[id].status = newStatus;              // MODIFY (in memory)
await writeFile(filePath, JSON.stringify(data, 2)); // WRITE

// Problem: Another process can read/write between these steps!
// Result: Lost updates, corrupted JSON, inconsistent state
```

**Examples:**
- `update-work-unit-status` - Changes work unit status
- `create-story` - Adds new work unit
- `delete-work-unit` - Removes work unit
- `add-rule`, `add-example`, `answer-question` - Modifies example mapping
- ~50 commands total

**Lock Needed:** WRITE lock (exclusive - blocks ALL readers and ALL writers)

**Lock Lifecycle:**
```
ACQUIRE WRITE LOCK ← BEFORE ensureWorkUnitsFile()
  ├─ Wait for all active readers to finish
  ├─ Wait for any active writer to finish
  ├─ Block new readers/writers from starting
  ├─ Call fs.readFile()
  ├─ Parse JSON
  ├─ Modify data in memory
  ├─ Call fs.writeFile() with atomic pattern
  │   ├─ Write to temp file (.tmp.abc123)
  │   └─ Rename to original file (atomic operation)
  └─ RELEASE WRITE LOCK (in finally block)
       └─ Even if error occurs, lock MUST be released
```

**Why Hold Lock Throughout?**
- Ensures atomicity of entire read-modify-write sequence
- Prevents interleaved operations from other processes
- Guarantees consistency (no partial updates)

**Refactored Pattern:**
```typescript
// New (with locks)
await fileManager.transaction<WorkUnitsData>(
  join(specPath, 'work-units.json'),
  async (data) => {
    // WRITE LOCK ACQUIRED HERE

    // Modify data
    data.workUnits[id].status = newStatus;

    // Return modified data (will be written atomically)
    return data;

    // WRITE LOCK RELEASED HERE (in finally block)
  }
);
```

---

### Type 3: WRITE-ONLY Operations (Initial File Creation)

**Pattern:**
```typescript
// Current (in ensure* functions)
try {
  const content = await readFile(filePath, 'utf-8');
  return JSON.parse(content);
} catch (error) {
  if (error.code === 'ENOENT') {
    // File doesn't exist, create it
    await writeFile(filePath, JSON.stringify(initialData, null, 2));
    return initialData;
  }
  throw error;
}
```

**Examples:**
- First time running fspec
- ensure* functions creating missing files

**Lock Needed:** WRITE lock

**Lock Lifecycle:**
```
TRY:
  ACQUIRE READ LOCK
    ├─ Attempt fs.readFile()
    └─ RELEASE READ LOCK
CATCH (ENOENT):
  ACQUIRE WRITE LOCK
    ├─ Check again if file exists (another process may have created it)
    ├─ If still missing: Create with initial data
    └─ RELEASE WRITE LOCK
```

---

## 3. WHERE TO ACQUIRE LOCKS

### Decision: Centralize in `LockedFileManager` Singleton

**Why?**
- Single source of truth for all file locking
- Prevents duplicate lock acquisition logic
- Easier to maintain and test
- Ensures consistency across codebase

**Implementation Location:**
```
src/utils/file-manager.ts (NEW FILE)
  └─ LockedFileManager class (singleton)
      ├─ readJSON<T>(filePath, defaultData) → Promise<T>
      ├─ writeJSON<T>(filePath, data) → Promise<void>
      └─ transaction<T>(filePath, fn) → Promise<void>
```

**Singleton Pattern:**
```typescript
// file-manager.ts
class LockedFileManager {
  private static instance: LockedFileManager;
  private readLocks: Map<string, number> = new Map(); // file → reader count
  private writeLocks: Set<string> = new Set();        // file → is locked
  private waitingReaders: Map<string, Array<() => void>> = new Map();
  private waitingWriters: Map<string, Array<() => void>> = new Map();

  private constructor() {}

  public static getInstance(): LockedFileManager {
    if (!LockedFileManager.instance) {
      LockedFileManager.instance = new LockedFileManager();
    }
    return LockedFileManager.instance;
  }

  // Methods here...
}

export const fileManager = LockedFileManager.getInstance();
```

---

## 4. WHEN TO RELEASE LOCKS

### Rule 1: ALWAYS Release in Finally Block

**Why?**
- Prevents lock leaks if error occurs
- Guarantees release even if exception thrown
- Critical for system stability

**Pattern:**
```typescript
async acquireReadLock(file: string): Promise<void> {
  // Acquire logic...
}

async releaseReadLock(file: string): Promise<void> {
  // Release logic...
}

async readJSON<T>(filePath: string, defaultData: T): Promise<T> {
  const lockfile = require('proper-lockfile');
  let release: (() => Promise<void>) | undefined;

  try {
    // Acquire inter-process lock
    release = await lockfile.lock(filePath, {
      stale: 10000,
      retries: { retries: 10, minTimeout: 50, maxTimeout: 500 }
    });

    // Acquire in-process read lock
    await this.acquireReadLock(filePath);

    try {
      const content = await readFile(filePath, 'utf-8');
      return JSON.parse(content);
    } finally {
      // CRITICAL: Release in-process read lock
      await this.releaseReadLock(filePath);
    }
  } finally {
    // CRITICAL: Release inter-process lock
    if (release) {
      await release();
    }
  }
}
```

### Rule 2: Release READ Locks Immediately After Parse

**Why?**
- Data is in memory, file access no longer needed
- Allows other readers to proceed concurrently
- Minimizes lock hold time

**Pattern:**
```typescript
const data = await fileManager.readJSON(filePath, defaultData);
// ← READ LOCK RELEASED HERE (inside readJSON method)
// Now use data in memory...
```

### Rule 3: Release WRITE Locks After Atomic Write Completes

**Why?**
- Ensures atomic write-replace pattern completes
- Prevents partial writes visible to readers
- Guarantees consistency

**Pattern:**
```typescript
async transaction<T>(filePath: string, fn: (data: T) => Promise<T>): Promise<void> {
  let release: (() => Promise<void>) | undefined;

  try {
    // Acquire inter-process write lock
    release = await lockfile.lock(filePath, lockOptions);

    // Acquire in-process write lock
    await this.acquireWriteLock(filePath);

    try {
      // Read current data
      const data = await this.readJSON<T>(filePath, {} as T);

      // Execute user function (modify data)
      const modifiedData = await fn(data);

      // Write atomically
      const tempFile = `${filePath}.tmp.${randomUUID()}`;
      await writeFile(tempFile, JSON.stringify(modifiedData, null, 2));
      await rename(tempFile, filePath); // ← ATOMIC operation

      // ← WRITE COMPLETES HERE
    } finally {
      // CRITICAL: Release in-process write lock
      await this.releaseWriteLock(filePath);
    }
  } finally {
    // CRITICAL: Release inter-process write lock
    if (release) {
      await release();
    }
  }
}
```

---

## 5. AFFECTED FILES - COMPLETE LIST

### Core Utilities (MUST REFACTOR)

**src/utils/file-manager.ts** (NEW FILE)
- LockedFileManager class
- readJSON(), writeJSON(), transaction() methods
- Singleton pattern

**src/utils/ensure-files.ts** (REFACTOR ALL 5 FUNCTIONS)
- ensureWorkUnitsFile() → Use fileManager.readJSON()
- ensurePrefixesFile() → Use fileManager.readJSON()
- ensureEpicsFile() → Use fileManager.readJSON()
- ensureTagsFile() → Use fileManager.readJSON()
- ensureFoundationFile() → Use fileManager.readJSON()

### Commands (66 FILES - REFACTOR WRITE OPERATIONS)

**Read-Modify-Write Operations:**
- update-work-unit.ts
- update-work-unit-status.ts
- update-prefix.ts
- set-user-story.ts
- remove-rule.ts
- remove-question.ts
- remove-example.ts
- remove-dependency.ts
- remove-attachment.ts
- create-story.ts
- create-bug.ts
- create-task.ts
- create-prefix.ts
- delete-work-unit.ts
- add-rule.ts
- add-question.ts
- add-example.ts
- add-dependency.ts
- add-attachment.ts
- add-assumption.ts
- add-architecture-note.ts
- answer-question.ts
- prioritize-work-unit.ts
- register-tag.ts (tags.json)
- update-foundation.ts (foundation.json)
- add-diagram.ts (foundation.json)
- ... (~40 more commands with write operations)

**Read-Only Operations:**
- list-work-units.ts
- show-work-unit.ts
- display-board.ts
- query-*.ts commands
- show-*.ts commands
- list-*.ts commands
- ... (~20 more read-only commands)

### TUI Store (REFACTOR FOR READ LOCKS)

**src/tui/store/fspecStore.ts**
- loadData() → Calls ensureWorkUnitsFile(), ensureEpicsFile()
- Currently read-only (no writes)
- Needs READ lock integration
- Refreshes every 2 seconds

---

## 6. LOCK LIFECYCLE EXAMPLES

### Example 1: TUI Board Refresh (READ-ONLY)

**Before (no locks):**
```typescript
// TUI refreshes every 2 seconds
loadData: async () => {
  const workUnitsData = await ensureWorkUnitsFile(cwd); // No lock
  const epicsData = await ensureEpicsFile(cwd);         // No lock
  // Update UI
}
```

**After (with locks):**
```typescript
loadData: async () => {
  // ACQUIRE READ LOCK on work-units.json
  const workUnitsData = await fileManager.readJSON(
    join(cwd, 'spec/work-units.json'),
    defaultWorkUnitsData
  );
  // RELEASE READ LOCK immediately after parse

  // ACQUIRE READ LOCK on epics.json
  const epicsData = await fileManager.readJSON(
    join(cwd, 'spec/epics.json'),
    defaultEpicsData
  );
  // RELEASE READ LOCK immediately after parse

  // Update UI (data in memory, no locks needed)
}
```

**Timeline:**
```
0ms:  TUI refresh starts
1ms:  Acquire read lock on work-units.json
2ms:  Read file (3ms duration)
5ms:  Parse JSON
6ms:  Release read lock on work-units.json ← Other readers can start now
7ms:  Acquire read lock on epics.json
8ms:  Read file (1ms duration)
9ms:  Parse JSON
10ms: Release read lock on epics.json
11ms: Update UI (no locks needed)
```

---

### Example 2: Concurrent Read and Write (update-work-unit-status)

**Scenario:** TUI refreshing while user runs `fspec update-work-unit-status BOARD-001 done`

**Before (no locks - RACE CONDITION!):**
```
Time  TUI (Read)                     CLI (Write)
----  --------------------------     ---------------------------
0ms   Read work-units.json
1ms   Parse JSON                     Read work-units.json
2ms                                  Parse JSON
3ms                                  Modify data (status = done)
4ms                                  Write work-units.json ← Overwrites TUI's read
5ms   Update UI with stale data ← UI shows old status!
```

**After (with locks - SAFE!):**
```
Time  TUI (Read)                     CLI (Write)
----  --------------------------     ---------------------------
0ms   Acquire READ lock
1ms   Read work-units.json           Attempt WRITE lock → BLOCKED (reader active)
2ms   Parse JSON
3ms   Release READ lock ← TUI done
4ms                                  Acquire WRITE lock ← Now allowed
5ms                                  Read work-units.json
6ms                                  Parse JSON
7ms                                  Modify data (status = done)
8ms                                  Write atomically
9ms                                  Release WRITE lock
10ms  Next refresh starts
11ms  Acquire READ lock ← CLI done
12ms  Read work-units.json
13ms  Parse JSON (sees updated data) ← UI shows correct status!
14ms  Release READ lock
```

---

### Example 3: Multiple Concurrent Reads (readers-writer pattern)

**Scenario:** Three commands run concurrently:
1. `fspec list-work-units` (reads)
2. `fspec show-work-unit BOARD-001` (reads)
3. `fspec update-work-unit-status BOARD-002 done` (writes)

**Timeline:**
```
Time  Command 1 (Read)         Command 2 (Read)         Command 3 (Write)
----  -----------------------  -----------------------  -------------------------
0ms   Acquire READ lock
1ms   Read file                Acquire READ lock ← Allowed (both read)
2ms                            Read file
3ms   Parse JSON               Parse JSON               Attempt WRITE lock → BLOCKED
4ms   Release READ lock        Release READ lock
5ms                                                     Acquire WRITE lock ← Now allowed
6ms                                                     Read file
7ms                                                     Parse JSON
8ms                                                     Modify data
9ms                                                     Write atomically
10ms                                                    Release WRITE lock
```

**Key Points:**
- Commands 1 and 2 read CONCURRENTLY (readers-writer pattern allows this)
- Command 3 WAITS for both readers to finish
- Command 3 acquires exclusive write lock after readers release
- Total time: 10ms (vs 15ms if sequential)

---

## 7. KEY QUESTIONS ANSWERED

### Q1: Where exactly should locks be acquired?

**Answer:** In `LockedFileManager` methods, BEFORE any file I/O operations.

**Details:**
```typescript
class LockedFileManager {
  async readJSON<T>(filePath: string, defaultData: T): Promise<T> {
    // ← ACQUIRE READ LOCK HERE (before readFile)

    // Inter-process lock (proper-lockfile)
    const release = await lockfile.lock(filePath, options);

    // In-process lock (readers-writer pattern)
    await this.acquireReadLock(filePath);

    try {
      const content = await readFile(filePath, 'utf-8');
      return JSON.parse(content);
    } finally {
      await this.releaseReadLock(filePath);
      await release();
    }
  }

  async transaction<T>(filePath: string, fn: (data: T) => Promise<T>): Promise<void> {
    // ← ACQUIRE WRITE LOCK HERE (before any I/O)

    const release = await lockfile.lock(filePath, options);
    await this.acquireWriteLock(filePath);

    try {
      // Read-modify-write sequence here
    } finally {
      await this.releaseWriteLock(filePath);
      await release();
    }
  }
}
```

---

### Q2: Where exactly should locks be released?

**Answer:** In `finally` blocks, AFTER file operations complete.

**Details:**
- READ locks: Release immediately after JSON.parse() completes
- WRITE locks: Release after atomic write completes (rename operation)
- ALWAYS use finally blocks to guarantee release even on error

**Why finally blocks?**
```typescript
try {
  await acquireLock();
  await doWork(); // ← Might throw error
} finally {
  await releaseLock(); // ← ALWAYS executes, even if error
}
```

---

### Q3: What about nested operations (reading multiple files)?

**Answer:** Acquire separate locks for each file, DO NOT hold multiple locks simultaneously.

**Pattern:**
```typescript
// CORRECT: Sequential locking (one file at a time)
const workUnits = await fileManager.readJSON('work-units.json', defaultData);
// ← work-units.json READ LOCK RELEASED HERE

const epics = await fileManager.readJSON('epics.json', defaultData);
// ← epics.json READ LOCK RELEASED HERE

// WRONG: Holding multiple locks (deadlock risk)
await fileManager.transaction('work-units.json', async (wu) => {
  await fileManager.transaction('epics.json', async (ep) => {
    // ← Both locks held simultaneously - DEADLOCK RISK!
  });
});
```

**Why?**
- Holding multiple locks increases deadlock risk
- Lock order must be consistent (alphabetical) to prevent deadlocks
- Better: Lock one file, release, lock next file

---

### Q4: What about the TUI refreshing every 2 seconds?

**Answer:** TUI uses READ locks, which are concurrent and non-blocking for other readers.

**Details:**
- TUI refresh acquires READ lock
- READ locks are SHARED (multiple readers allowed)
- Other commands reading concurrently are NOT blocked
- Only WRITE operations wait for TUI to finish
- TUI releases lock immediately after parse (< 10ms)

**Impact on Writers:**
```
Worst case: Writer waits for TUI refresh to complete
- TUI acquires READ lock
- Writer attempts WRITE lock → BLOCKED
- TUI releases READ lock (< 10ms)
- Writer acquires WRITE lock → PROCEEDS
- Total delay: < 10ms (negligible)
```

---

## 8. MIGRATION STRATEGY

### Phase 1: Create LockedFileManager (NEW)

**File:** `src/utils/file-manager.ts`

**Contents:**
- LockedFileManager class (singleton)
- readJSON<T>(filePath, defaultData) method
- writeJSON<T>(filePath, data) method
- transaction<T>(filePath, fn) method
- Internal lock tracking (Map, Set, queues)
- proper-lockfile integration
- Readers-writer pattern implementation

**Estimated Effort:** 4-6 hours (complex logic, needs careful design)

---

### Phase 2: Refactor ensure-files.ts (MODIFY)

**File:** `src/utils/ensure-files.ts`

**Changes:**
- Import fileManager singleton
- Replace all `readFile` + `JSON.parse` with `fileManager.readJSON()`
- Replace all `writeFile` + `JSON.stringify` with `fileManager.writeJSON()`
- Keep function signatures unchanged (transparent to callers)

**Example:**
```typescript
// BEFORE
export async function ensureWorkUnitsFile(cwd: string): Promise<WorkUnitsData> {
  const specPath = await findOrCreateSpecDirectory(cwd);
  const filePath = join(specPath, 'work-units.json');

  try {
    const content = await readFile(filePath, 'utf-8');
    return JSON.parse(content);
  } catch (error) {
    if (error.code === 'ENOENT') {
      await writeFile(filePath, JSON.stringify(initialData, null, 2));
      return initialData;
    }
    throw error;
  }
}

// AFTER
export async function ensureWorkUnitsFile(cwd: string): Promise<WorkUnitsData> {
  const specPath = await findOrCreateSpecDirectory(cwd);
  const filePath = join(specPath, 'work-units.json');

  return await fileManager.readJSON<WorkUnitsData>(filePath, {
    meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
    workUnits: {},
    states: { backlog: [], specifying: [], testing: [], implementing: [], validating: [], done: [], blocked: [] }
  });
}
```

**Estimated Effort:** 2-3 hours (5 functions to refactor)

---

### Phase 3: Refactor Commands with Write Operations (MODIFY ~50 FILES)

**Files:** All commands that do read-modify-write

**Pattern:**
```typescript
// BEFORE
const workUnitsData = await ensureWorkUnitsFile(cwd);
const filePath = join(cwd, 'spec/work-units.json');

// Modify data
workUnitsData.workUnits[id].status = newStatus;

// Write back
await writeFile(filePath, JSON.stringify(workUnitsData, null, 2));

// AFTER
const specPath = await findOrCreateSpecDirectory(cwd);
const filePath = join(specPath, 'work-units.json');

await fileManager.transaction<WorkUnitsData>(filePath, async (data) => {
  // Modify data
  data.workUnits[id].status = newStatus;
  return data;
});
```

**Estimated Effort:** 8-12 hours (50 commands × 10-15 minutes each)

---

### Phase 4: Update TUI Store (MODIFY)

**File:** `src/tui/store/fspecStore.ts`

**Changes:**
- Replace ensureWorkUnitsFile() with fileManager.readJSON()
- Replace ensureEpicsFile() with fileManager.readJSON()
- No write operations (TUI is read-only)

**Estimated Effort:** 1 hour

---

### Phase 5: Add Concurrency Tests (NEW)

**Files:** `src/utils/__tests__/file-manager.test.ts` (NEW)

**Test Cases:**
1. Multiple concurrent readers (no blocking)
2. Reader and writer coordination (writer waits)
3. Multiple writers (sequential execution)
4. Stale lock detection (crashed process recovery)
5. Atomic write-replace (crash during write)
6. Lock leak prevention (error handling)
7. Transaction rollback (error in callback)

**Estimated Effort:** 6-8 hours (comprehensive test suite)

---

### Total Estimated Effort: 20-30 hours

**Breakdown:**
- Phase 1 (LockedFileManager): 4-6 hours
- Phase 2 (ensure-files): 2-3 hours
- Phase 3 (Commands): 8-12 hours
- Phase 4 (TUI): 1 hour
- Phase 5 (Tests): 6-8 hours

---

## 9. CRITICAL SUCCESS FACTORS

### Must Work 100% Perfectly the First Time

**Why?**
- All-at-once migration (not incremental)
- Cannot have partial locking (some files locked, others not)
- One corrupted JSON file = entire fspec broken
- No gradual rollout possible

**How to Ensure Success:**
1. **Comprehensive tests FIRST** - Write all concurrency tests before implementation
2. **Test in isolation** - Test LockedFileManager independently before integration
3. **Incremental integration** - Integrate ensure-files.ts first, then commands one-by-one
4. **Rollback plan** - Git checkpoints before refactoring each component
5. **Validation** - Run full test suite after each phase

---

## 10. RISKS AND MITIGATION

### Risk 1: Lock Leaks (Locks Never Released)

**Impact:** Permanent deadlock, fspec unusable

**Mitigation:**
- ALWAYS use finally blocks
- Comprehensive error handling tests
- Timeout on lock acquisition (10 second stale detection)
- Add lock leak detection in LockedFileManager

---

### Risk 2: Deadlocks (Circular Waiting)

**Impact:** Commands hang indefinitely

**Mitigation:**
- Lock ordering: Alphabetical file order
- Avoid holding multiple locks simultaneously
- Timeout on lock acquisition
- Retry with exponential backoff

---

### Risk 3: Performance Degradation

**Impact:** Commands slow down, TUI laggy

**Mitigation:**
- Readers-writer pattern (concurrent reads)
- Release locks immediately after I/O
- Profile lock hold times
- Optimize hot paths (work-units.json operations)

---

### Risk 4: Test Flakiness (Timing-Dependent Tests)

**Impact:** CI/CD fails intermittently

**Mitigation:**
- Use deterministic test patterns
- Mock time/delays in tests
- Increase timeouts for concurrency tests
- Run tests multiple times to detect flakiness

---

## 11. NEXT STEPS

1. **Review this analysis** - Confirm scope and approach
2. **Update Example Mapping** - Add implementation insights to LOCK-001
3. **Write comprehensive tests** - Start with LockedFileManager tests
4. **Implement LockedFileManager** - Core locking logic
5. **Refactor ensure-files.ts** - First integration point
6. **Refactor commands** - One by one, with tests for each
7. **Validate** - Full test suite + manual testing

---

## 12. OPEN QUESTIONS FOR HUMAN

1. **Lock hold time limits:** Should we add warnings if locks are held > 1 second?
2. **Metrics collection:** Should we log lock acquisition times (FSPEC_DEBUG_LOCKS env var)?
3. **Fallback strategy:** If locking fails, should we retry or fail immediately?
4. **Network filesystems:** Any specific testing needed for NFS/SMB?
5. **Migration plan approval:** Does this all-at-once approach make sense, or prefer incremental?
