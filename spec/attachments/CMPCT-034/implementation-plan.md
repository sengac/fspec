# CMPCT-034: Implementation Plan

**Date:** 2026-04-18
**Depends on:** CMPCT-033

## Problem

`useCompaction()` maintains `isActive`, `progress`, `sessionId`, `trigger` in local React `useState`, duplicating what Rust already tracks per-session via `SessionStatus::Compacting` and `CompactionProgress`. This local state is NOT session-scoped — it follows the user when they Shift+Left/Right to switch sessions.

The existing `useRustSessionState(currentSessionId)` hook already reads `isCompacting` and `compactionProgress` from Rust per-session (see `fetchFreshSnapshot()` in `useRustSessionState.ts:157-187`). When `currentSessionId` changes on navigation, the hook re-subscribes and fetches the new session's state from Rust — automatically correct.

## Root Cause Trace

1. Rust sets `SessionStatus::Compacting` on Session B → `session_manager.rs:5869`
2. Rust emits `SessionStateChange(Compacting)` chunk through global callback
3. TypeScript handler calls `compactionRef.current.startCompaction('hook-triggered', 'sess-B', ...)` → **writes to LOCAL React useState** (not Rust)
4. AgentView renders `isCompacting={compaction.state.isActive}` → reads from LOCAL state → shows "Compacting..." on Session A
5. User presses Shift+Left → `currentSessionId` changes to Session A → `rustSnapshot.isCompacting` correctly reads `false` from Rust for Session A
6. BUT `compaction.state.isActive` is still `true` (never cleared) → MultiLineInput still shows "Compacting..."

## Architecture: Rust is Source of Truth

### Already exists (no changes needed):
- `Session.status: AtomicU8` → `SessionStatus::Compacting` (Rust)
- `Session.compaction_progress: RwLock<Option<CompactionProgress>>` (Rust)
- `session_get_status(session_id)` → returns `"compacting"` (NAPI)
- `session_get_compaction_progress(session_id)` → returns progress (NAPI)
- `useRustSessionState(currentSessionId)` → `rustSnapshot.isCompacting` + `rustSnapshot.compactionProgress` (React)
- `refreshRustState(sessionId)` → triggers re-read from Rust (React)

### Pattern reference (already working correctly):
- **IsolationStateChange**: `globalSessionStreamManager.ts:290-312` — writes to `useSessionStore` (Zustand) keyed by sessionId
- **FooterStateUpdate**: `globalSessionStreamManager.ts:315-328` — writes to `useFooterStore` (Zustand) keyed by sessionId
- **isPaused/pauseInfo**: comes from `rustSnapshot.isPaused` / `rustSnapshot.pauseInfo` (via `useRustSessionState`)
- **isLoading**: comes from `rustSnapshot.isLoading` (via `useRustSessionState`)

## Fix Plan

### Step 1: Strip display state from `useCompaction`

**File:** `src/tui/hooks/useCompaction.ts`

Remove from `UnifiedCompactionState`:
- `isActive` → replaced by `rustSnapshot.isCompacting`
- `progress` → replaced by `rustSnapshot.compactionProgress`
- `trigger` → not needed for UI display
- `sessionId` → not needed for UI display

Keep in `useCompaction`:
- `performManualCompaction(sessionId)` → still needed to call NAPI `sessionCompact()`
- `retryState` → still needed for retry dialog
- `handleRetryOption` / `clearRetryState` → still needed for retry dialog

Remove from `useCompaction`:
- `startCompaction()` → no longer needed; Rust sets status to Compacting before emitting the chunk
- `endCompaction()` → no longer needed; Rust sets status to Idle on CompactionComplete
- `updateProgress()` → no longer needed; `rustSnapshot.compactionProgress` reads directly from Rust
- Progress polling `useEffect` → no longer needed; `refreshRustState()` handles reactivity

### Step 2: Remove `startCompaction` / `endCompaction` calls from handlers

**File:** `src/tui/handlers/persistentSessionStateHandler.ts`

Remove the `Compacting` branch entirely — no need to call `startCompaction`. The `refreshRustState()` call at the end already tells `useRustSessionState` to re-read from Rust, which picks up `isCompacting: true` + `compactionProgress`.

Update `SessionStateChangeDeps`:
- Remove `startCompaction`
- Remove `getCompactionProgress`

**File:** `src/tui/components/AgentView.tsx`

In `persistentChunkHandler` (~line 988-1019):
- Remove the `startCompaction` dep wiring
- Remove the `getCompactionProgress` dep wiring

In `handleSubmit` inline handler (~line 2354-2366):
- Remove `compactionRef.current.startCompaction(...)` call
- `refreshRustState(activeSessionId)` at line 2372 already handles it

In `handleStreamChunk` (~line 3403-3415):
- Remove `compactionRef.current.startCompaction(...)` call
- `refreshRustState(currentSessionIdRef.current)` at line 3422 already handles it

In `handleCompactionComplete` (~line 970-986):
- Remove `compactionRef.current.endCompaction()` call
- `refreshRustStateRef.current(sessionId)` at line 982 already handles it (Rust sets status to Idle)

### Step 3: Switch UI to `rustSnapshot`

**File:** `src/tui/components/AgentView.tsx`

Line 5476-5477: Change from:
```tsx
isCompacting={compaction.state.isActive}
compactionProgress={compaction.state.progress}
```
To:
```tsx
isCompacting={rustSnapshot.isCompacting}
compactionProgress={rustSnapshot.compactionProgress}
```

Line 5493: Change from:
```tsx
compaction.state.isActive
```
To:
```tsx
rustSnapshot.isCompacting
```

### Step 4: Remove `compactionRef` (no longer needed)

The `compactionRef` was only needed because stream handlers (closures) needed access to `startCompaction`/`endCompaction`. Since those calls are removed, the ref is no longer needed.

Remove:
- `compactionRef` declaration (~line 895)
- `compactionRef.current = compaction` effect (~line 896-898)
- All `compactionRef.current.*` calls

### Step 5: Update tests

**File:** `src/tui/handlers/__tests__/persistentSessionStateHandler-routed-sessionid.test.ts`

Update all tests:
- Remove `startCompaction` and `getCompactionProgress` from `SessionStateChangeDeps` mocks
- Remove assertions about `startCompaction` being called
- The handler no longer needs to route compaction — it just handles `Cleared` and always calls `refreshRustState`

**File:** `src/tui/hooks/__tests__/useCompaction.test.tsx`

Update to reflect the stripped-down hook:
- Remove tests for `startCompaction`, `endCompaction`, `updateProgress`
- Remove tests for `state.isActive`, `state.progress`
- Keep tests for `performManualCompaction`, `retryState`, `handleRetryOption`

### Step 6: Update feature file and coverage

Update the feature file for CMPCT-034 to reflect:
- The UI reads compaction state from `rustSnapshot` (Rust source of truth)
- Session switch clears the compacting indicator because `useRustSessionState` re-subscribes to the new session
- `useCompaction` is reduced to manual compaction operations + retry logic only

## Files Modified

| File | Change |
|---|---|
| `src/tui/hooks/useCompaction.ts` | Strip display state, remove `startCompaction`/`endCompaction`/`updateProgress`/polling |
| `src/tui/handlers/persistentSessionStateHandler.ts` | Remove `Compacting` branch and deps |
| `src/tui/components/AgentView.tsx` | Switch UI to `rustSnapshot`, remove `compactionRef`, remove `startCompaction`/`endCompaction` calls |
| `src/tui/handlers/__tests__/persistentSessionStateHandler-routed-sessionid.test.ts` | Update deps and assertions |
| `src/tui/hooks/__tests__/useCompaction.test.tsx` | Strip display state tests |

## Risk Assessment

**Low risk.** The `rustSnapshot.isCompacting` and `rustSnapshot.compactionProgress` paths already exist and work correctly — they're already used for the ESC interrupt flow (`AgentView.tsx:4787-4788`). We're just removing the parallel local state that shadows them.

The `performManualCompaction` action path is unchanged — it still calls `sessionCompact()` via NAPI. The retry dialog is unchanged. The only thing changing is where the "Compacting..." indicator reads its state from: Rust (via `rustSnapshot`) instead of local React `useState`.
