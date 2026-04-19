# BUG-135 — [DEBUG] Badge Disappears on Session Cycling

## Summary

After BUG-133 wired the `[DEBUG]` badge through Zustand + Rust stream events, a regression was introduced: cycling **A → B → A** via `Shift+Left` / `Shift+Right` loses the badge on the second visit to A, even though Rust's authoritative per-session `BackgroundSession::is_debug_enabled: AtomicBool` still holds `true`.

## Symptom (User Reproduction)

1. Open the TUI, create / attach to session **A**.
2. Run `/debug` in session A → `[DEBUG]` badge appears. ✓
3. `Shift+Right` → switch to session **B** → `[DEBUG]` correctly hidden. ✓
4. `Shift+Left` → switch back to session A → **`[DEBUG]` badge is gone.** ✗

Expected: `[DEBUG]` re-appears on A because Rust still has `is_debug_enabled=true` for A.

## Root Cause (Three Cooperating Mistakes in BUG-133)

### Mistake 1 — Flat boolean Zustand slot

File: `src/tui/store/sessionStore.ts:41`

```typescript
/** BUG-133: Whether debug capture is enabled for the current session */
isDebugEnabled: boolean;
```

A single flat boolean cannot remember two sessions' states simultaneously. The reference pattern (`isIsolated`) gets away with a flat boolean only because isolation never changes after session creation — so it's effectively a one-shot hydration. **Debug state, by contrast, can be toggled at any time**, so every session needs its own persistent record.

### Mistake 2 — `activateSession()` blanks debug on every switch

File: `src/tui/store/sessionStore.ts:142-159`

```typescript
activateSession: (sessionId: string) => {
  ...
  set(state => {
    state.currentSessionId = sessionId;
    ...
    // BUG-133: Reset debug state on session switch - will be rehydrated
    // via applyPendingDebugState() or DebugStateChange stream event
    state.isDebugEnabled = false;   // ← resets to false every time
  });
},
```

The comment promised rehydration would happen via `applyPendingDebugState()` or a fresh `DebugStateChange` chunk. Neither fires on a simple activation-without-toggle.

### Mistake 3 — `getPendingDebugState` consumes on read

File: `src/tui/services/globalSessionStreamManager.ts:296-308`

```typescript
public getPendingDebugState(sessionId: string): PendingDebugState | null {
  const state = this.pendingDebugState.get(sessionId);
  if (state) {
    this.pendingDebugState.delete(sessionId);   // ← entry gone after first read
    ...
    return state;
  }
  return null;
}
```

After the first visit to A consumes A's entry, there is nothing left to rehydrate from on the second visit. Rust never re-emits a `DebugStateChange` on mere session activation — it only emits on toggles.

### Additional complication — Rust ground-truth fallback is actively forbidden by tests

File: `src/tui/components/__tests__/debug-badge-session-awareness.test.tsx:288-301`

The BUG-133 tests explicitly forbid `AgentView` from OR-ing in `rustSnapshot.isDebugEnabled` from `useRustSessionState`, which was the previous safety net. So there is now no fallback path that consults Rust's authoritative per-session `AtomicBool` when Zustand has been blanked.

## Reference Pattern vs. Actual Implementation

| Concern | `isIsolated` (works) | `isDebugEnabled` (broken) |
|---|---|---|
| Mutability over session lifetime | Set once at creation, never changes | Toggleable anytime via `/debug` |
| Zustand slot | Flat `boolean` (OK because immutable) | Flat `boolean` (WRONG — loses state across switches) |
| Re-entry rehydration | Not needed (value never stale) | **Required** — debug can have been toggled |
| Pending-state consumption | Safe (one-shot event per session) | Unsafe (entry gone after first visit) |

`isDebugEnabled` is a fundamentally different lifecycle from `isIsolated` and must not blindly copy its pattern.

## Chosen Fix — Per-Session Map in Zustand

Make Zustand's debug state an authoritative per-session map, with the current session's value surfaced via a selector. This mirrors the **intent** of the Rust `HashMap<Uuid, BackgroundSession>` one level up in the stack.

### Design

**1. Replace flat field with per-session map.**

`src/tui/store/sessionStore.ts`:

```typescript
export interface SessionStoreState {
  ...
  /** BUG-135: Per-session debug capture state, keyed by sessionId.
   *  Mirrors Rust's per-session BackgroundSession::is_debug_enabled. */
  debugStateBySession: Map<string, boolean>;
  ...
  /** Set debug capture state for a given session. */
  setDebugState: (sessionId: string, isDebugEnabled: boolean) => void;
}
```

**2. Selector derives current-session value.**

```typescript
export const useIsDebugEnabled = () =>
  useSessionStore(state => {
    const sid = state.currentSessionId;
    return sid ? (state.debugStateBySession.get(sid) ?? false) : false;
  });
```

**3. `activateSession` no longer blanks debug state.**

Removing the `state.isDebugEnabled = false` line — the selector now reads the target session's retained value directly.

**4. `applyPendingDebugState` feeds the per-session map with a `sessionId` argument.**

```typescript
setDebugState(sessionId, pendingState.isDebugEnabled);
```

**5. Initial hydration on attach.**

After `applyPendingDebugState(sessionId)`, also re-seed from Rust's authoritative per-session `AtomicBool` via `sessionGetDebugEnabled(sessionId)`. This covers the A→B→A case where no pending event exists but Rust still knows the truth.

### Why This Fix (vs. Alternatives)

| Option | Verdict |
|---|---|
| A. Peek instead of consume pending map | ❌ Still loses state when Rust-origin toggle happens before TUI attach and pending state is the only record |
| B. Always re-read Rust on every selector call | ❌ Turns a pure Zustand selector into a NAPI call every render |
| C. Re-seed Zustand from Rust on every `resumeSessionById` | ⚠️ Works but duplicates source-of-truth (Zustand AND Rust) with no clear invariant |
| **D. Per-session Map + hydrate-on-attach** | ✅ Zustand is authoritative per-session, Rust is only consulted on attach, selector is pure |

Option D is chosen.

## Scope of Changes

### Files to modify

| File | Change |
|---|---|
| `src/tui/store/sessionStore.ts` | `isDebugEnabled: boolean` → `debugStateBySession: Map<string, boolean>`. Update `setDebugState`, `activateSession`, `clearAndResetSession`, `useIsDebugEnabled`. |
| `src/tui/services/globalSessionStreamManager.ts` | `setDebugState(isDebugEnabled)` call sites → `setDebugState(sessionId, isDebugEnabled)`. `applyPendingDebugState` passes `sessionId` through. |
| `src/tui/components/AgentView.tsx` | In `resumeSessionById` and other session-attach paths, after `applyPendingDebugState`, also call `setDebugState(sessionId, sessionGetDebugEnabled(sessionId))` as a fallback hydration. |
| `src/tui/components/__tests__/debug-badge-session-awareness.test.tsx` | Update existing assertions to address `debugStateBySession.get(sessionId)` rather than flat `isDebugEnabled`. Add new regression scenarios for A→B→A cycling. |

### Tests to add

1. **Cycling regression** — A has debug on, switch to B, switch back to A, badge must re-appear.
2. **Per-session isolation** — A debug on, B debug on, toggling A off must not affect B's entry in the map.
3. **Hydration fallback** — After TUI restart where pending map is empty, switching to A must read `sessionGetDebugEnabled(A) = true` and surface the badge.
4. **Selector purity** — `useIsDebugEnabled` must derive from Zustand state without any NAPI call per render.

## Out of Scope

- BUG-134 (Rust-side per-session `DebugCaptureManager` refactor) is already `done` and unrelated to this fix.
- No changes to `SessionHeader.tsx` itself — it already correctly renders the badge when its prop is `true`.

## Backward Compatibility

No external API changes. Zustand store shape change is internal to the TUI layer.

## Risk Assessment

- **Low risk** — Pure TUI state-management change; no Rust or NAPI changes.
- **Test coverage** — Existing `debug-badge-session-awareness.test.tsx` tests will need updates; new cycling test will catch future regressions.
