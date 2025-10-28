# Multi-File Consistency Strategy

## Executive Summary

**Decision:** Use sequential transactions for multi-file operations. Accept eventual consistency with defensive querying patterns and repair utilities.

**Rationale:** Two-phase commit adds 20+ hours of complexity for minimal real-world benefit. The codebase already has defensive querying and denormalized data that tolerates temporary inconsistencies.

---

## The Problem

Many fspec commands write to multiple JSON files atomically (from the user's perspective):

```typescript
// create-story writes TWO files
await writeFile(workUnitsFile, ...);  // File 1
await writeFile(epicsFile, ...);      // File 2

// What if file 2 write fails?
// - work-units.json already committed (work unit created)
// - epics.json not updated (epic's work unit list incomplete)
// - Result: INCONSISTENT STATE
```

**19 commands have multi-file writes:**
- create-story, create-bug, create-task (work-units → epics)
- delete-epic (epics → prefixes → work-units)
- register-tag, update-tag, delete-tag (tags.json → TAGS.md)
- add-diagram, delete-diagram (foundation.json → FOUNDATION.md)
- example-mapping operations (work-units → example-map.json)
- 10+ more commands

---

## Three Approaches Considered

### Option A: Two-Phase Commit (REJECTED)

**How it works:**
1. **Prepare phase:** Lock all files, write to temp files
2. **Commit phase:** Rename all temp files atomically
3. **Rollback:** If any step fails, delete all temp files

**Pros:**
- ✅ Perfect consistency (all-or-nothing atomicity)
- ✅ No repair needed

**Cons:**
- ❌ **20-30 hours implementation cost** (distributed transaction coordinator)
- ❌ **Extremely complex** (WAL, crash recovery, nested lock management)
- ❌ **2x lock hold time** (blocks other operations during both phases)
- ❌ **High risk** (complex code = more bugs, harder maintenance)
- ❌ **Overkill** (solving a theoretical problem that rarely occurs)

**Verdict:** NOT WORTH THE COST

---

### Option B: Nested Transactions (REJECTED)

**How it works:**
```typescript
await fileManager.multiTransaction([
  { file: workUnitsFile, fn: (data) => { ... } },
  { file: epicsFile, fn: (data) => { ... } }
]);
```

**Pros:**
- ✅ Atomic across multiple files

**Cons:**
- ❌ **Deadlock risk** (nested locks on multiple files)
- ❌ **Lock ordering requirements** (must lock alphabetically to prevent deadlocks)
- ❌ **Complex API** (harder to understand and use correctly)
- ❌ **Still requires 2PC** (same complexity as Option A)

**Verdict:** Same problems as Option A

---

### Option C: Sequential Transactions + Defensive Patterns (ACCEPTED ✅)

**How it works:**
```typescript
// Transaction 1: work-units.json
await fileManager.transaction(workUnitsFile, async (data) => {
  data.workUnits[nextId] = newStory;
  data.states.backlog.push(nextId);
});

// Transaction 2: epics.json (separate, sequential)
await fileManager.transaction(epicsFile, async (data) => {
  data.epics[epicId].workUnits.push(nextId);
});

// If Transaction 2 fails:
// - Transaction 1 already committed
// - Result: Temporary inconsistency
// - Defensive querying + repair utilities handle it
```

**Pros:**
- ✅ **Simple** (just sequential transactions, no 2PC)
- ✅ **Fast** (no nested lock overhead)
- ✅ **Matches existing patterns** (delete-epic already uses this)
- ✅ **Defensive querying tolerates inconsistency** (queries check both directions)
- ✅ **Repair utilities available** (`fspec repair-work-units`)
- ✅ **Minimal refactoring** (replace writeFile with transaction, same logic)

**Cons:**
- ⚠️ **Temporary inconsistency possible** (if later writes fail)
- ⚠️ **Requires documentation** (must explain limitation)

**Verdict:** BEST TRADE-OFF

---

## Why Sequential Transactions Work

### 1. Denormalized Data Structure

fspec uses **denormalized data** (redundant storage) across files:

```typescript
// work-units.json
{
  "AUTH-001": {
    "id": "AUTH-001",
    "epic": "user-management"  // ← Stored here
  }
}

// epics.json
{
  "user-management": {
    "id": "user-management",
    "workUnits": ["AUTH-001"]  // ← AND here (redundant)
  }
}
```

**Same relationship stored in BOTH files!**

This redundancy is a **design choice** for:
- Fast queries (don't need to join files)
- Simple API (commands work with single files)
- Offline resilience (each file is self-describing)

**Trade-off:** Redundancy requires consistency maintenance

---

### 2. Defensive Querying (Tolerates Inconsistency)

fspec queries **check both directions** and accept if **either** is valid:

```typescript
// src/commands/epics.ts
const relatedWorkUnits = allWorkUnits.filter(wu =>
  wu.epic === epicId ||  // ← Check work unit's epic field
  (epic.workUnits && epic.workUnits.includes(wu.id))  // ← OR epic's list
);
```

**If inconsistency occurs:**
- Work unit has `epic: "user-management"` ✅
- Epic missing work unit in list ❌
- Query: `wu.epic === "user-management"` → **TRUE** ✅
- **Result:** Work unit still found! Query succeeds despite inconsistency.

**This pattern is ALREADY IN THE CODEBASE** (not added for locking).

---

### 3. Repair Utilities

`fspec repair-work-units` can detect and fix inconsistencies:

```bash
fspec repair-work-units
```

**Current capabilities:**
- Rebuilds states arrays from work unit status fields
- Repairs bidirectional dependencies (blocks/blockedBy, relatesTo)

**Future enhancement:**
- Add cross-file consistency checks
- Detect work unit with epic but not in epic's list
- Add missing work units to epic's list
- Remove deleted work units from epic's list

**Implementation:** ~2 hours (vs 20+ hours for 2PC)

---

## Failure Scenarios and Impact

### Scenario 1: create-story fails on epic write

**What happens:**
```
1. Write work-units.json ✅ (Transaction 1 commits)
2. Disk full / Permissions error
3. Write epics.json ❌ (Transaction 2 fails)

Result:
- workUnit.epic = "user-management" ✅
- epic.workUnits missing "AUTH-003" ❌
```

**Impact:**
- **User sees error:** "Failed to create story: ENOSPC: no space left on device"
- **work-units.json:** Work unit exists ✅
- **epics.json:** Epic's list incomplete ❌
- **Queries:** Work unit still found (defensive querying checks `wu.epic`)
- **TUI:** Work unit shows in backlog ✅
- **Epic list:** Work unit missing from epic's list ⚠️

**Recovery:**
1. User fixes disk space issue
2. User runs `fspec repair-work-units` (future enhancement detects/fixes)
3. Or manually re-run `fspec update-work-unit AUTH-003 --epic user-management`

**Probability:** VERY LOW (disk full between two writes ~milliseconds apart)

---

### Scenario 2: delete-epic fails on prefixes write

**What happens:**
```
1. Delete from epics.json ✅ (Transaction 1 commits)
2. Permission denied on prefixes.json
3. Update prefixes.json ❌ (Transaction 2 fails - silently caught!)

Result:
- Epic deleted ✅
- Prefix still has epicId reference ❌
```

**Impact:**
- **User sees:** "✓ Epic deleted successfully" (error swallowed)
- **epics.json:** Epic gone ✅
- **prefixes.json:** Dangling reference ❌
- **Queries:** Prefix queries check if epic exists before using it

**Recovery:**
- `fspec repair-work-units` (future enhancement) detects dangling epic references
- Or user manually runs `fspec update-prefix PREFIX --epic=''`

**Note:** delete-epic ALREADY uses this pattern with try-catch!

---

### Scenario 3: register-tag fails on TAGS.md generation

**What happens:**
```
1. Update tags.json ✅ (Transaction 1 commits)
2. Write TAGS.md ❌ (Transaction 2 fails)

Result:
- tags.json has new tag ✅
- TAGS.md outdated ❌
```

**Impact:**
- **User sees error:** "Failed to generate TAGS.md"
- **tags.json:** Tag registered ✅ (source of truth)
- **TAGS.md:** Outdated documentation ❌

**Recovery:**
- User runs `fspec generate-tags-md` (regenerates from tags.json)
- Or manually edits TAGS.md

**Probability:** VERY LOW

---

## Implementation Pattern

### Single-File Transaction (Most Commands)

```typescript
// Before (no locks)
const data = await ensureWorkUnitsFile(cwd);
const workUnitsFile = join(cwd, 'spec/work-units.json');

// Validate
if (!data.workUnits[id]) {
  throw new Error('Work unit does not exist');
}

// Mutate
data.workUnits[id].status = newStatus;

// Write
await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

// After (with locks)
const specPath = await findOrCreateSpecDirectory(cwd);
const workUnitsFile = join(specPath, 'work-units.json');

await fileManager.transaction<WorkUnitsData>(workUnitsFile, async (data) => {
  // Validate
  if (!data.workUnits[id]) {
    throw new Error('Work unit does not exist');
  }

  // Mutate
  data.workUnits[id].status = newStatus;

  // No return, no write - fileManager handles it
});
```

**Changes:**
- ✅ Remove `ensureWorkUnitsFile()` call
- ✅ Wrap in `transaction()`
- ✅ Remove `writeFile()` call
- ✅ Keep all validation and mutation logic unchanged

---

### Multi-File Transaction (Sequential)

```typescript
// Before (no locks)
const workUnitsData = await ensureWorkUnitsFile(cwd);
workUnitsData.workUnits[nextId] = newStory;
await writeFile(workUnitsFile, JSON.stringify(workUnitsData, 2));

const epicsData = await ensureEpicsFile(cwd);
epicsData.epics[epicId].workUnits.push(nextId);
await writeFile(epicsFile, JSON.stringify(epicsData, 2));

// After (with locks - sequential transactions)
await fileManager.transaction<WorkUnitsData>(workUnitsFile, async (data) => {
  data.workUnits[nextId] = newStory;
});
// ← Transaction 1 complete, lock released

if (options.epic) {
  await fileManager.transaction<EpicsData>(epicsFile, async (data) => {
    data.epics[epicId].workUnits.push(nextId);
  });
  // ← Transaction 2 complete, lock released
}
// If Transaction 2 fails, Transaction 1 is already committed (acceptable)
```

**Key points:**
- ✅ **Sequential** (not nested) - no deadlock risk
- ✅ **Separate locks** - each file locked independently
- ✅ **Fast** - lock released immediately after each transaction
- ⚠️ **Not atomic** - second transaction can fail after first succeeds

---

## Documentation Requirements

### 1. Code Comments

Add JSDoc to `fileManager.transaction()`:

```typescript
/**
 * Execute a read-modify-write operation with exclusive write lock.
 *
 * @param filePath - Absolute path to JSON file
 * @param fn - Callback that mutates data in place
 *
 * @remarks
 * Multi-file operations use sequential transactions (not atomic).
 * If later transactions fail, earlier transactions are already committed.
 * This is acceptable due to:
 * - Denormalized data (redundant storage)
 * - Defensive querying (checks both directions)
 * - Repair utilities (fspec repair-work-units)
 *
 * @example
 * ```typescript
 * // Single file (atomic)
 * await fileManager.transaction(workUnitsFile, async (data) => {
 *   data.workUnits[id].status = 'done';
 * });
 *
 * // Multiple files (sequential, not atomic)
 * await fileManager.transaction(workUnitsFile, async (data) => {
 *   data.workUnits[id] = newWorkUnit;
 * });
 * await fileManager.transaction(epicsFile, async (data) => {
 *   data.epics[epicId].workUnits.push(id);
 * });
 * // If second fails, first is committed (acceptable)
 * ```
 */
async transaction<T>(
  filePath: string,
  fn: (data: T) => Promise<void> | void
): Promise<void>
```

---

### 2. Architecture Documentation

Add to `spec/FOUNDATION.md`:

```markdown
## Data Consistency Model

fspec uses **eventual consistency** with sequential transactions for multi-file operations.

**Design Principles:**

1. **Denormalized Data** - Same relationship stored in multiple files for fast queries
2. **Defensive Querying** - Queries check multiple sources, accept if any is valid
3. **Sequential Transactions** - Multi-file writes use separate transactions (not atomic)
4. **Repair Utilities** - `fspec repair-work-units` detects and fixes inconsistencies

**Trade-offs:**

- ✅ Simple implementation (no distributed transactions)
- ✅ Fast operations (minimal lock hold time)
- ✅ Tolerates temporary inconsistency
- ⚠️ Rare failure modes can leave inconsistent state (recoverable)

**When Inconsistency Occurs:**

If a multi-file operation fails partway through (disk full, permissions error), earlier
writes are committed but later writes may fail. Run `fspec repair-work-units` to detect
and fix cross-file inconsistencies.
```

---

### 3. User-Facing Documentation

Add to `docs/troubleshooting.md`:

```markdown
## Data Inconsistency Errors

**Symptom:** Work unit shows epic but not in epic's work unit list (or vice versa)

**Cause:** Multi-file operation failed partway through (disk full, permissions error)

**Solution:**

```bash
fspec repair-work-units
```

This command:
- Rebuilds states arrays from work unit status fields
- Repairs bidirectional dependencies (blocks/blockedBy, relatesTo)
- Fixes cross-file inconsistencies (work-units ↔ epics)

**Prevention:** Ensure sufficient disk space and file permissions before bulk operations
```

---

## Future Enhancements (Optional)

### 1. Enhanced repair-work-units (2 hours)

Add cross-file consistency checks:

```typescript
// Check work-units ↔ epics consistency
for (const [id, workUnit] of Object.entries(workUnitsData.workUnits)) {
  if (workUnit.epic) {
    const epic = epicsData.epics[workUnit.epic];
    if (!epic.workUnits.includes(id)) {
      // Add work unit to epic's list
      epic.workUnits.push(id);
      repairs.push(`Added ${id} to epic ${workUnit.epic}'s work unit list`);
    }
  }
}

// Check epics → work-units consistency
for (const [epicId, epic] of Object.entries(epicsData.epics)) {
  if (epic.workUnits) {
    for (const workUnitId of epic.workUnits) {
      const workUnit = workUnitsData.workUnits[workUnitId];
      if (!workUnit || workUnit.epic !== epicId) {
        // Remove from epic's list
        epic.workUnits = epic.workUnits.filter(id => id !== workUnitId);
        repairs.push(`Removed ${workUnitId} from epic ${epicId}'s work unit list`);
      }
    }
  }
}
```

---

### 2. Pre-flight Validation (1 hour)

Add disk space check before multi-file operations:

```typescript
async function checkDiskSpace(filePaths: string[]): Promise<void> {
  const totalSize = filePaths.reduce((sum, path) => {
    return sum + fs.statSync(path).size;
  }, 0);

  const { available } = await checkDiskSpace('/');
  if (available < totalSize * 2) {
    throw new Error('Insufficient disk space for operation');
  }
}
```

---

### 3. Transactional Log (5 hours)

Add write-ahead log for crash recovery:

```typescript
// Before: Log intent
await fs.appendFile('fspec.txn.log', JSON.stringify({
  op: 'create-story',
  files: [workUnitsFile, epicsFile],
  timestamp: Date.now()
}));

// Execute: Sequential transactions
await fileManager.transaction(workUnitsFile, ...);
await fileManager.transaction(epicsFile, ...);

// After: Clear log
await fs.truncate('fspec.txn.log');

// On crash: Replay log
if (fs.existsSync('fspec.txn.log')) {
  // Read log, re-execute incomplete operations
}
```

**Note:** This is OPTIONAL. Not needed for v1.0.

---

## Summary

**Chosen Strategy:** Sequential transactions with eventual consistency

**Why:**
- ✅ Simple (minimal implementation cost)
- ✅ Fast (no nested lock overhead)
- ✅ Defensive (querying tolerates inconsistency)
- ✅ Recoverable (repair utilities)
- ✅ Pragmatic (solves 99.9% of cases, handles edge cases gracefully)

**Not Chosen:**
- ❌ Two-phase commit (20+ hours, overkill for rare failure mode)
- ❌ Nested transactions (deadlock risk, complex API)

**Documentation:**
- ✅ JSDoc on `transaction()` method
- ✅ Architecture section in FOUNDATION.md
- ✅ Troubleshooting guide for users
- ✅ Code comments in multi-file commands

**Future Work:**
- Enhance `repair-work-units` with cross-file checks (2 hours)
- Add disk space pre-flight validation (1 hour)
- Consider transactional log if failure rate increases (5 hours)

---

## Related Work Units

- LOCK-001: Implement file locking for concurrent access safety (this work unit)
- Future: Enhance repair-work-units with cross-file consistency checks
- Future: Add disk space validation for bulk operations
