# TUI-079: BoardView Update Gaps Investigation

## Summary

When `work-units.json` is modified externally (e.g. by an AI agent via the Fspec tool, or by the fspec CLI), the BoardView does not fully reflect the changes. The root cause is that **TUI-060** migrated the file watcher from `useWorkUnitsWatcher` (which called `loadData()`) to `globalStreamListener` (which calls the lossy `updateWorkUnitsFromWatcher()` path).

## Associated Card

**TUI-060** — "Session Header Work Unit Status Display" — introduced the `globalStreamListener` architecture and moved the work units watcher from BoardView to a global singleton. The comment in `BoardView.tsx` line 136 reads:

```
// TUI-060: Work units watcher is now handled by globalStreamListener at TUI startup
```

## Architecture

There are **two update paths** for work units in the TUI:

### Path 1: `loadData()` (TypeScript — full read)

Called on mount and after move operations. Reads `work-units.json` via `ensureWorkUnitsFile()`, which provides the **complete** data:

- `workUnits` — full `WorkUnit` objects with all 20+ fields
- `states` — ordered arrays per column controlling display priority
- Runs auto-migrations if needed

Then builds an `orderedWorkUnits` array by iterating `states` arrays column-by-column (`fspecStore.ts` lines 146-166), preserving priority order.

### Path 2: `updateWorkUnitsFromWatcher()` (Rust watcher — partial data)

The Rust `work_units_watcher.rs` watches the `spec/` directory for file changes and emits `StreamChunk::WorkUnitsUpdate` with a `Vec<WorkUnitInfo>`. The `globalStreamListener.ts` receives these and calls `updateWorkUnitsFromWatcher()`.

The Rust `WorkUnitsFile` struct (line 58 of `work_units_watcher.rs`) only deserializes:

```rust
struct WorkUnitsFile {
    work_units: HashMap<String, WorkUnit>,
}
```

It **ignores** the `states` field entirely. And `WorkUnitInfo` only carries 7 fields:

```rust
pub struct WorkUnitInfo {
    pub id: String,
    pub title: String,
    pub work_type: String,
    pub status: String,
    pub description: Option<String>,
    pub estimate: Option<i32>,
    pub epic: Option<String>,
}
```

## Identified Gaps

### Gap 1: Ordering is lost — `states` arrays not read by Rust watcher

The `states` field in `work-units.json` controls display priority order within each column. The Rust `WorkUnitsFile` struct doesn't deserialize it. When `updateWorkUnitsFromWatcher()` runs:

1. It maps over `state.workUnits` (the existing flat array) and updates fields in-place
2. New units are **appended to the end** of the array
3. The `states` array order is never consulted

**Result:** Work units that move between columns appear in the wrong position. Priority reordering done externally is not reflected.

`loadData()` fixes this by building the array from `states` arrays, but the watcher path bypasses it.

### Gap 2: `stateHistory` not sent — "last changed" indicator (⏩) stale

`UnifiedBoardLayout` computes `lastChangedWorkUnit` from `stateHistory` timestamps (lines 168-181). The Rust `WorkUnit`/`WorkUnitInfo` structs don't include `stateHistory`. After a watcher update, the `...existingUnit` spread preserves the **stale** `stateHistory` from the initial `loadData()`. The ⏩ emoji never moves to the newly-changed work unit.

### Gap 3: `attachments` not sent — attachment indicators stale

`UnifiedBoardLayout` shows attachment info in the details panel. `WorkUnitInfo` has no `attachments` field. After watcher updates, attachments added externally won't appear until a full `loadData()`.

### Gap 4: Deleted work units never removed

`updateWorkUnitsFromWatcher()` (lines 364-402 of `fspecStore.ts`) maps existing units and adds new ones, but **never removes** units absent from the watcher data. Deleted work units remain as ghosts on the board.

### Gap 5: Priority order within columns incorrect after status changes

Even when `status` is correctly updated, the flat array order is never reconciled with the `states` arrays. `BoardView` groups via `filter(wu => wu.status === status)` which works for column assignment, but the **order within each column** is determined by the flat array position, not the `states` array priority.

### Gap 6: Unused `useWorkUnitsWatcher` hook had the correct behavior

`src/tui/hooks/useWorkUnitsWatcher.ts` exists and does the right thing — on `WorkUnitsUpdate` events it calls `loadData()` (line 112). But **BoardView no longer uses this hook** since TUI-060 moved to `globalStreamListener`, which uses the lossy path.

## Key Files

| File | Role |
|------|------|
| `codelet/napi/src/work_units_watcher.rs` | Rust file watcher — reads file, emits `WorkUnitsUpdate` chunks |
| `codelet/napi/src/types.rs` (line 182) | `WorkUnitInfo` struct — only 7 fields |
| `src/tui/store/globalStreamListener.ts` | Receives watcher events, calls `updateWorkUnitsFromWatcher()` |
| `src/tui/store/fspecStore.ts` | Zustand store — both `loadData()` and `updateWorkUnitsFromWatcher()` |
| `src/tui/hooks/useWorkUnitsWatcher.ts` | Unused hook that correctly calls `loadData()` on change |
| `src/tui/components/BoardView.tsx` | Board component — delegates to `globalStreamListener` |
| `src/tui/components/UnifiedBoardLayout.tsx` | Board renderer — uses `stateHistory`, `attachments`, ordering |

## Recommended Fix

**Make `globalStreamListener` call `loadData()` instead of `updateWorkUnitsFromWatcher()`** when it receives a `WorkUnitsUpdate` event. This is what the unused `useWorkUnitsWatcher` hook already does. The file is already written to disk when the watcher fires, so the TypeScript re-read is essentially free.

This fixes all 6 gaps in one change:
- ✅ Ordering comes from `states` arrays
- ✅ Full `stateHistory` available for last-changed indicator
- ✅ Full `attachments` available for details panel
- ✅ Deleted work units removed (they won't be in `states` arrays)
- ✅ Priority order within columns matches `states`
- ✅ No Rust-side changes needed

The `updateWorkUnitsFromWatcher()` function and the partial data from the Rust watcher become unnecessary — the watcher event serves purely as a "file changed" signal.

## Additional Findings (Post-Investigation)

### Gap 7: `fspecStore.WorkUnit` interface missing `attachments` field

The `fspecStore.ts` `WorkUnit` interface (lines 37-46) does not declare an `attachments` field, yet `UnifiedBoardLayout.tsx` and `BoardView.tsx` both access `workUnit.attachments`. This works at runtime because `loadData()` pushes the raw JSON object which carries extra fields. However, `updateWorkUnitsFromWatcher()` builds objects with only the declared fields, so new work units created externally have `attachments` as `undefined`. The recommended fix (switching to `loadData()`) resolves this since the full JSON object is used.

### Gap 8: Session context not cleared when current work unit is deleted externally

In `globalStreamListener.ts` lines 68-86, after the watcher update, the code checks `chunk.workUnits` for the current work unit to sync session status. If the current work unit is **deleted** externally:

- `chunk.workUnits.find(wu => wu.id === currentWorkUnitId)` returns `undefined`
- The code does nothing — `sessionStore.currentWorkUnitId` still points to the deleted unit
- The session header continues showing the deleted work unit

**This gap is NOT fixed by switching to `loadData()`.** After `loadData()` completes, the session sync logic still uses `chunk.workUnits` from the Rust watcher. The fix: after `loadData()`, check if `currentWorkUnitId` still exists in the store's `workUnits` array. If not, clear session context via `setCurrentWorkUnit(null, null)`.

### Gap 9: `updated` timestamp stale (latent)

`WorkUnitInfo` has no `updated` field. Existing units preserve stale `updated` from initial `loadData()`. No component currently renders this, so no visible impact — but it is a latent issue. The `loadData()` fix resolves this.
