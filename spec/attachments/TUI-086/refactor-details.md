# TUI-086: Refactor sessionStore — Split, DRY, State Desync Fix

**Addresses:** C1, C2, C6
**Priority:** 1 (highest impact)

---

## C1: File Size — 415 Lines (38% Over Limit)

### Current Structure
```
Lines 17–21:    Imports (5 lines)
Lines 26–158:   SessionStoreState interface (133 lines)
Lines 163–174:  initialState object (12 lines)
Lines 179–361:  Store creation + all actions with immer (183 lines)
Lines 366–393:  Selector hooks (28 lines)
Lines 398–415:  Action hooks with useShallow (18 lines)
```

### Proposed Split

**`sessionStore.ts`** (store + types + initial state + actions) — ~210 lines
- Interface definition
- Initial state
- Store creation with all action implementations

**`sessionSelectors.ts`** (selector hooks) — ~35 lines
- Extract all `useXxx` selector hooks
- Use named module-level selectors for efficiency:
  ```typescript
  const selectCurrentWorkUnitId = (s: SessionStoreState) => s.currentWorkUnitId;
  export const useCurrentWorkUnitId = () => useSessionStore(selectCurrentWorkUnitId);
  ```

**`sessionActions.ts`** (action hooks) — ~25 lines
- Extract `useSessionActions` with `useShallow`
- Re-export from barrel if needed

---

## C2: DRY Violation — 4 Functions with Identical Patterns

### Evidence (Field Overlap Table)

| Field | prepareForNew | reset | navigateToNew | navigateIsolated |
|-------|:---:|:---:|:---:|:---:|
| `currentSessionId = null` | ✅ | ✅ | ✅ | ✅ |
| `isReadyForNewSession = true` | ✅ | ✅ | ✅ | ✅ |
| `showCreateSessionDialog = false` | ✅ | ✅ | ✅ | ✅ |
| `currentWorkUnitId = null` | ✅ | ✅ | ✅ | ✅ |
| `currentWorkUnitStatus = null` | ✅ | ✅ | ✅ | ✅ |
| `isIsolated = false` | ✅ | ✅ | ✅ | ✅ |
| `worktreePath = null` | ✅ | ✅ | ✅ | ✅ |
| `shouldAutoCreateSession` | ❌ | `false` | `true` | `true` |
| `pendingIsolatedSession` | ❌ | `false` | `false` | `isolated` |
| `navigationTargetSessionId` | ❌ | `null` | `null` | `null` |

### Proposed Fix

```typescript
/** Reset common session state fields. Called by all session transition actions. */
function clearAndResetSession(
  set: SetState,
  options?: {
    shouldAutoCreateSession?: boolean;
    pendingIsolatedSession?: boolean;
    navigationTargetSessionId?: string | null;
  }
): void {
  try {
    sessionClearActive();
  } catch (e) {
    logger.warn(`[SessionStore] Failed to clear active session in Rust: ${e}`);
  }
  set(state => {
    state.currentSessionId = null;
    state.isReadyForNewSession = true;
    state.showCreateSessionDialog = false;
    state.currentWorkUnitId = null;
    state.currentWorkUnitStatus = null;
    state.isIsolated = false;
    state.worktreePath = null;
    state.pendingIsolatedSession = options?.pendingIsolatedSession ?? false;
    state.shouldAutoCreateSession = options?.shouldAutoCreateSession ?? false;
    state.navigationTargetSessionId = options?.navigationTargetSessionId ?? null;
  });
}
```

Then `navigateToNewSession` and `navigateToNewSessionIsolated` can merge into:
```typescript
navigateToNewSession: (isolated = false) => {
  clearAndResetSession(set, {
    shouldAutoCreateSession: true,
    pendingIsolatedSession: isolated,
  });
}
```

---

## C6: setIsolationState Missing `pendingIsolatedSession` Reset

### Current Code (lines 249–256)
```typescript
setIsolationState: (isIsolated: boolean, worktreePath: string | null) => {
  set(state => {
    state.isIsolated = isIsolated;
    state.worktreePath = worktreePath;
  });
}
```

### Problem
`pendingIsolatedSession` is set to `true` in `navigateToNewSessionIsolated` but NEVER reset by `setIsolationState`. If isolation state changes externally (e.g., via `IsolationStateChange` chunk from globalStreamListener), the pending flag remains stale.

### Fix
```typescript
setIsolationState: (isIsolated: boolean, worktreePath: string | null) => {
  set(state => {
    state.isIsolated = isIsolated;
    state.worktreePath = worktreePath;
    state.pendingIsolatedSession = false;  // Clear pending flag on actual state change
  });
}
```

---

## Acceptance Criteria

1. `sessionStore.ts` is under 300 lines
2. New `sessionSelectors.ts` and `sessionActions.ts` files exist
3. All 4 `sessionClearActive + set()` functions use a shared helper
4. `navigateToNewSession` and `navigateToNewSessionIsolated` are merged
5. `setIsolationState` resets `pendingIsolatedSession`
6. All existing tests pass without modification (public API unchanged)
7. No new `any` types introduced
