# TUI-016 Incomplete Implementation - Bug Report

## Executive Summary

**Status**: TUI-016 is 50% complete. The IPC infrastructure is fully built and tested, but the TUI integration layer is NOT connected. The checkpoint display in `CheckpointPanel.tsx` is still using **chokidar file-watching** which MUST BE REMOVED and replaced with Zustand + IPC.

---

## Critical Issue: CheckpointPanel Using Outdated File-Watching

### Current Broken Implementation

**File**: `src/tui/components/CheckpointPanel.tsx`

**Problems**:
1. ❌ **Line 8**: Imports chokidar (MUST BE REMOVED)
2. ❌ **Line 61**: Uses local `useState` for counts (WRONG - should use Zustand store)
3. ❌ **Lines 64-92**: Sets up chokidar file watcher (ENTIRE BLOCK MUST BE DELETED)
4. ❌ Not connected to IPC message system
5. ❌ Not connected to Zustand state management

### What MUST Happen

**REMOVE ALL OF THIS**:
```typescript
import chokidar from 'chokidar';  // ❌ DELETE THIS LINE

const [counts, setCounts] = useState<CheckpointCounts>({  // ❌ DELETE - use store instead
  manual: 0,
  auto: 0,
});

// ❌ DELETE ENTIRE useEffect BLOCK (lines 64-92) - no more file watching!
useEffect(() => {
  const watcher = chokidar.watch(checkpointIndexDir, { ... });
  // ... all this code must be removed
}, [checkpointIndexDir]);
```

**REPLACE WITH THIS**:
```typescript
// Read from Zustand store instead
const checkpointCounts = useFspecStore(state => state.checkpointCounts);
const loadCheckpointCounts = useFspecStore(state => state.loadCheckpointCounts);

// Load on mount
useEffect(() => {
  void loadCheckpointCounts();
}, [loadCheckpointCounts]);

// Use store state in render
const displayText = checkpointCounts.manual === 0 && checkpointCounts.auto === 0
  ? 'Checkpoints: None'
  : `Checkpoints: ${checkpointCounts.manual} Manual, ${checkpointCounts.auto} Auto`;
```

---

## Three Missing Components

### 1. Zustand Store Missing Checkpoint State

**File**: `src/tui/store/fspecStore.ts`

**What's Missing**:
- No `checkpointCounts` state property
- No `loadCheckpointCounts()` action method

**What to Add**:
```typescript
interface FspecState {
  // ... existing state properties
  checkpointCounts: { manual: number; auto: number };  // ADD THIS
  loadCheckpointCounts: () => Promise<void>;           // ADD THIS
}

// In createFspecStore() initialization:
const useFspecStore = create<FspecState>((set, get) => ({
  // ... existing state

  checkpointCounts: { manual: 0, auto: 0 },  // ADD THIS

  loadCheckpointCounts: async () => {         // ADD THIS
    const gitDir = path.join(process.cwd(), '.git');
    const checkpointIndexDir = path.join(gitDir, 'fspec-checkpoints-index');

    try {
      const files = await fs.readdir(checkpointIndexDir);
      const manual = files.filter(f => !f.includes('-auto-')).length;
      const auto = files.filter(f => f.includes('-auto-')).length;

      set({ checkpointCounts: { manual, auto } });
    } catch (error) {
      // Directory doesn't exist or is empty
      set({ checkpointCounts: { manual: 0, auto: 0 } });
    }
  },
}));
```

---

### 2. BoardView Missing IPC Server

**File**: `src/tui/components/BoardView.tsx`

**What's Missing**:
- No IPC server initialization on component mount
- No event listeners for 'checkpoint-changed' messages
- No cleanup on component unmount

**What to Add** (after existing useEffects around line 137):
```typescript
import { createIPCServer, cleanupIPCServer, getIPCPath } from '../../utils/ipc';
import type { Server } from 'net';

// Add this useEffect
useEffect(() => {
  let server: Server | null = null;

  try {
    server = createIPCServer((message) => {
      if (message.type === 'checkpoint-changed') {
        // Trigger store reload when checkpoint command runs
        void useFspecStore.getState().loadCheckpointCounts();
      }
      // Add more message handlers here as needed for future IPC events
    });

    const ipcPath = getIPCPath();
    server.listen(ipcPath);
  } catch (error) {
    // IPC server failed to start (non-fatal - TUI still works)
  }

  return () => {
    if (server) {
      cleanupIPCServer(server);
    }
  };
}, []);
```

---

### 3. CheckpointPanel Refactor

**File**: `src/tui/components/CheckpointPanel.tsx`

**Current Issues**:
- Line 8: Imports chokidar (DELETE)
- Line 61: Uses local useState (DELETE)
- Lines 64-92: File watcher setup (DELETE ENTIRE BLOCK)

**Complete Refactor Required**:

```typescript
// REMOVE chokidar import completely
// import chokidar from 'chokidar';  ❌ DELETE THIS

// ADD Zustand store import
import { useFspecStore } from '../store/fspecStore';

export const CheckpointPanel: React.FC = () => {
  // REMOVE local state
  // const [counts, setCounts] = useState<CheckpointCounts>({ ... });  ❌ DELETE

  // USE store state instead
  const checkpointCounts = useFspecStore(state => state.checkpointCounts);
  const loadCheckpointCounts = useFspecStore(state => state.loadCheckpointCounts);

  // REMOVE entire chokidar watcher useEffect (lines 64-92)  ❌ DELETE

  // ADD simple load on mount
  useEffect(() => {
    void loadCheckpointCounts();
  }, [loadCheckpointCounts]);

  // Update display text to use store state
  const displayText = checkpointCounts.manual === 0 && checkpointCounts.auto === 0
    ? 'Checkpoints: None'
    : `Checkpoints: ${checkpointCounts.manual} Manual, ${checkpointCounts.auto} Auto`;

  // ... rest of component remains the same
};
```

---

## Why Chokidar MUST Be Removed

1. **Architectural Mismatch**: File-watching is the OLD approach. TUI-016 implemented IPC-based updates specifically to replace this.

2. **Performance**: Chokidar polls filesystem continuously, wasting resources.

3. **Reliability**: File-watching can miss events, have race conditions, and behave differently across platforms.

4. **Maintenance**: Two update mechanisms (file-watching + IPC) creates confusion and bugs.

5. **Already Built**: The IPC infrastructure is DONE and TESTED. We just need to connect it.

---

## Current Architecture vs Required Architecture

### Current (WRONG) ❌

```
Terminal A: fspec checkpoint TUI-016
    ↓
Writes to .git/fspec-checkpoints-index/
    ↓
Terminal B (TUI): CheckpointPanel chokidar detects file change
    ↓
Updates local state
    ↓
Display updates
```

### Required (CORRECT) ✅

```
Terminal A: fspec checkpoint TUI-016
    ↓
Writes to .git/fspec-checkpoints-index/
    ↓
Sends IPC message { type: 'checkpoint-changed' }
    ↓
Terminal B (TUI): BoardView IPC server receives message
    ↓
Calls store.loadCheckpointCounts()
    ↓
Store reads filesystem and updates checkpointCounts state
    ↓
CheckpointPanel re-renders from store
    ↓
Display updates
```

---

## What's Already Complete (Don't Touch)

✅ **IPC Utility** (`src/utils/ipc.ts`):
- `getIPCPath()` - Cross-platform socket/pipe paths
- `createIPCServer()` - Server creation with message handling
- `sendIPCMessage()` - Client message sending
- `cleanupIPCServer()` - Proper resource cleanup

✅ **Commands Sending IPC Messages**:
- `src/commands/checkpoint.ts` (line 41)
- `src/commands/update-work-unit-status.ts` (auto checkpoints)
- `src/commands/cleanup-checkpoints.ts`

✅ **IPC Integration Tests**:
- `src/utils/__tests__/ipc-integration.test.ts`
- 6 comprehensive test scenarios
- All passing

✅ **Architecture Documentation**:
- `tui-016-ipc-architecture.md` (complete implementation guide)

---

## Implementation Checklist

### Phase 1: Add Zustand Store State
- [ ] Edit `src/tui/store/fspecStore.ts`
- [ ] Add `checkpointCounts` state property
- [ ] Add `loadCheckpointCounts()` action method
- [ ] Test: Import store in CheckpointPanel and verify state exists

### Phase 2: Add IPC Server to BoardView
- [ ] Edit `src/tui/components/BoardView.tsx`
- [ ] Import IPC utilities
- [ ] Add useEffect to create IPC server on mount
- [ ] Add message handler for 'checkpoint-changed' events
- [ ] Add cleanup on unmount
- [ ] Test: Run TUI, verify socket/pipe created at `/tmp/fspec-tui.sock` or `\\.\pipe\fspec-tui`

### Phase 3: Refactor CheckpointPanel
- [ ] Edit `src/tui/components/CheckpointPanel.tsx`
- [ ] **REMOVE chokidar import** (line 8)
- [ ] **REMOVE useState for counts** (line 61)
- [ ] **REMOVE entire chokidar watcher useEffect** (lines 64-92)
- [ ] Add Zustand store hooks
- [ ] Add simple mount effect to call `loadCheckpointCounts()`
- [ ] Update display text to read from store state
- [ ] Test: Verify no TypeScript errors, component renders

### Phase 4: Integration Testing
- [ ] Start TUI in one terminal
- [ ] Run `fspec checkpoint <work-unit-id> <name>` in another terminal
- [ ] Verify checkpoint display updates immediately
- [ ] Test with manual and auto checkpoints
- [ ] Test with `fspec cleanup-checkpoints` command
- [ ] Verify no regression in existing TUI functionality

### Phase 5: Cleanup
- [ ] Remove chokidar from package.json dependencies if no longer used elsewhere
- [ ] Update TUI-016 feature file coverage
- [ ] Mark work unit as complete

---

## Files to Modify

1. **`src/tui/store/fspecStore.ts`** (add checkpoint state + action)
2. **`src/tui/components/BoardView.tsx`** (add IPC server lifecycle)
3. **`src/tui/components/CheckpointPanel.tsx`** (REMOVE chokidar, use store)

## Files That Are Already Correct (Don't Touch)

1. **`src/utils/ipc.ts`** ✅
2. **`src/commands/checkpoint.ts`** ✅
3. **`src/commands/update-work-unit-status.ts`** ✅
4. **`src/commands/cleanup-checkpoints.ts`** ✅
5. **`src/utils/__tests__/ipc-integration.test.ts`** ✅

---

## Estimated Effort

**Story Points**: 3 (1-2 hours)
- Simple state addition to Zustand store (15 min)
- IPC server setup in BoardView (20 min)
- CheckpointPanel refactor to remove chokidar (30 min)
- Testing and validation (30 min)

---

## Priority

**HIGH** - This is a half-completed feature where:
- Infrastructure is done but not connected
- Old implementation (chokidar) still in use and needs removal
- Affects user experience (checkpoint display may be stale)
- Creates technical debt if left incomplete

---

## Related Work Units

- **TUI-016** - Original work unit that implemented IPC infrastructure (status: done but incomplete)
- Consider creating follow-up tasks for other IPC message types if needed

---

## References

- **Architecture Doc**: `tui-016-ipc-architecture.md`
- **IPC Utility**: `src/utils/ipc.ts`
- **IPC Tests**: `src/utils/__tests__/ipc-integration.test.ts`
- **Zustand Store**: `src/tui/store/fspecStore.ts`
- **CheckpointPanel**: `src/tui/components/CheckpointPanel.tsx`
- **BoardView**: `src/tui/components/BoardView.tsx`
