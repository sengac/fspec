# TUI-079 Post-Fix Regression: BoardView Flickering

## Summary

Commit `a764bd46` ("fix: boardview state fixes") changed `globalStreamListener.ts` to call `loadData()` instead of `updateWorkUnitsFromWatcher()`. While this fixed data correctness gaps (ordering, missing fields, stale data), it introduced **severe flickering** in the BoardView due to lock contention, missing debouncing, and error-state oscillation.

## Evidence from `~/.fspec/fspec.log`

```
2026-03-15T05:25:49.611Z [error]: [ZUSTAND] loadData error: Lock file is already being held
2026-03-15T05:25:49.613Z [error]: [ZUSTAND] loadData error: Lock file is already being held
2026-03-15T05:25:50.132Z [error]: [ZUSTAND] loadData error: Lock file is already being held
2026-03-15T05:25:50.155Z [error]: [ZUSTAND] loadData error: Lock file is already being held
2026-03-15T05:25:50.155Z [error]: [ZUSTAND] loadData error: Lock file is already being held
2026-03-15T05:25:53.382Z [error]: [ZUSTAND] loadData error: Lock file is already being held
2026-03-15T05:25:55.342Z [error]: [ZUSTAND] loadData error: Lock file is already being held
2026-03-15T05:25:55.842Z [error]: [ZUSTAND] loadData error: Lock file is already being held
2026-03-15T05:25:56.927Z [error]: [ZUSTAND] loadData error: Lock file is already being held
2026-03-15T05:25:56.927Z [error]: [ZUSTAND] loadData error: Lock file is already being held
```

14 lock contention errors in ~7 seconds. Each one triggers the error UI, then the next successful load clears it — producing visible flash-to-error-and-back.

## Root Causes

### Root Cause 1: Lock Contention → Error State Oscillation (PRIMARY)

The flicker sequence:

1. AI agent or CLI writes to `work-units.json` (holds a **write lock** via `fileManager.transaction()`)
2. Rust `notify_debouncer_mini` fires after 100ms debounce
3. `globalStreamListener.handleStreamChunk()` calls `loadData()`
4. `loadData()` line 137 immediately does: `set(state => { state.error = null })` — clears any error
5. `loadData()` calls `ensureWorkUnitsFile()` → `fileManager.readJSON()` → tries to acquire filesystem lock
6. **Lock is still held by the writer** → `proper-lockfile` exhausts 10 retries (50–500ms backoff each) → throws `"Lock file is already being held"`
7. Error is NOT a JSON parse error → catch block sets: `set(state => { state.error = err.stack })` → **re-render to ErrorView** (lines 459–505 of BoardView.tsx)
8. Writer finishes, releases lock → next watcher event or concurrent `loadData()` succeeds → `set(error: null, workUnits: [...])` → **re-render back to normal board**
9. **Visible: board → error screen → board → error screen → board** in rapid succession

### Root Cause 2: No JavaScript-Side Debouncing → Concurrent loadData() Storms

Zero debouncing/throttling on the JavaScript side. Multiple triggers can fire `loadData()` simultaneously:

| Trigger | Source | When |
|---------|--------|------|
| Component mount | `BoardView.tsx:131` | Once on mount |
| File watcher | `globalStreamListener.ts:68` | Every `WorkUnitsUpdate` from Rust |
| IPC message | `BoardView.tsx:214` | `work-unit-changed` from AI agent process |
| Move up/down | `BoardView.tsx:571,582` | Explicit `await loadData()` after reorder |

A single `]` keypress (move work unit down) produces **3 concurrent `loadData()` calls**:
1. Explicit `await loadData()` on line 582
2. File write triggers Rust watcher → `globalStreamListener` → `loadData()` after ~100ms
3. If IPC bridge also fires → third `loadData()`

Each call acquires locks on 2 files (`work-units.json` + `epics.json`) = up to 6 lock acquisition attempts, causing contention cascades.

### Root Cause 3: `loadData()` Clears Error Unconditionally Before Reading

```typescript
loadData: async () => {
  set(state => {
    state.error = null;  // ← Clears error BEFORE trying to read (line 137-139)
  });
  try {
    // ... read files (may fail with ELOCKED)
```

Every `loadData()` call — even failed ones — first clears the error state, triggering a render to show the board, then potentially sets it again with a lock error, showing the error view. With 3+ concurrent calls, you get rapid oscillation between board and error states.

### Root Cause 4: Explicit `loadData()` After Move Operations Is Redundant

In `BoardView.tsx` `onMoveUp`/`onMoveDown`, after `moveWorkUnitUp/Down()` (which writes to disk), there is:
```typescript
await loadData();  // Lines 571, 582
```
This is **redundant** because the file write already triggers the Rust watcher → which triggers `loadData()`. But they run concurrently, causing the lock contention.

## Lock Acquisition Details

`fileManager.readJSON()` (src/utils/file-manager.ts:171):
- Uses `proper-lockfile` with `stale: 10000` (10s stale timeout)
- Retries: `{ retries: 10, minTimeout: 50, maxTimeout: 500 }` — up to ~5 seconds of backoff
- When all 10 retries exhausted → throws `"Lock file is already being held"`
- Each `loadData()` acquires locks on **2 files** sequentially: `work-units.json` then `epics.json`

## Recommended Fixes

### Fix 1: Debounce `loadData()` in globalStreamListener (Critical)

Add a JavaScript-side debounce so multiple rapid watcher events coalesce:

```typescript
let loadDataTimer: ReturnType<typeof setTimeout> | null = null;

if (chunk.type === 'WorkUnitsUpdate') {
  if (loadDataTimer) {
    clearTimeout(loadDataTimer);
  }
  loadDataTimer = setTimeout(() => {
    loadDataTimer = null;
    void useFspecStore.getState().loadData().then(() => { /* sync session context */ });
  }, 150); // 150ms debounce on top of Rust's 100ms
}
```

### Fix 2: Don't Set Error State for Lock Contention (Critical)

In `fspecStore.loadData()`, treat `ELOCKED` errors as transient — silently ignore them:

```typescript
} catch (error) {
  const err = error as Error;
  
  // Lock contention is transient — watcher will retry, don't flash error UI
  if (err.message.includes('Lock file is already being held')) {
    logger.debug('[ZUSTAND] loadData lock contention, will retry on next watcher event');
    return;
  }
  // ... existing error handling
}
```

### Fix 3: Guard Against Concurrent `loadData()` Calls (Critical)

Add an in-flight guard:

```typescript
let loadDataInFlight = false;

loadData: async () => {
  if (loadDataInFlight) {
    return;
  }
  loadDataInFlight = true;
  try {
    // ... existing logic
  } finally {
    loadDataInFlight = false;
  }
}
```

### Fix 4: Don't Clear Error Before Reading (Medium)

Move error clearing to success path only:

```typescript
// Remove the pre-read error clear (lines 137-139)
// Only clear error on success:
set(state => {
  state.workUnits = orderedWorkUnits;
  state.epics = Object.values(epicsData.epics);
  state.isLoaded = true;
  state.error = null; // Clear here instead
});
```

### Fix 5: Remove Redundant `loadData()` After Move Operations (Medium)

The explicit `await loadData()` after `moveWorkUnitUp/Down()` is redundant with the watcher-triggered one. Either:
- Remove the explicit call (watcher will handle it after debounce)
- Or keep it and let the in-flight guard prevent doubling

## Key Files

| File | Role |
|------|------|
| `src/tui/store/globalStreamListener.ts` | Receives watcher events, calls loadData() — needs debounce |
| `src/tui/store/fspecStore.ts` | Zustand store — loadData() needs concurrency guard + error handling |
| `src/tui/components/BoardView.tsx` | Board component — has redundant loadData() calls after moves |
| `src/utils/file-manager.ts` | Lock manager — source of ELOCKED errors |
