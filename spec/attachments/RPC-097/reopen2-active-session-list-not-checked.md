# RPC-097 reopen #2 — Active Session List Not Consulted on BoardView Shift+Right

## User report (verbatim)

> "it asks me if i want to create a new agent from the board when i hit
>  shift+right after I go back to the board with shift left and i already
>  have an agent open - so it's not checking the active agent list properly"

## Reproduction sequence

1. From BoardView, focus a work unit `W₀` (no attached session).
2. Press **Shift+Right** → CreateSessionDialog appears (RPC-097 reopen #1
   contract — correct, because no sessions exist yet).
3. Press **Enter** on **Yes** → session `sid-A` is created and the user
   is switched to AgentView; `open_sessions = [sid-A]`.
4. Press **Shift+Left** → back to BoardView (focus may be on `W₀` or any
   work unit; in practice the user lands on the same `W₀` they came from,
   which still has no attachment).
5. Press **Shift+Right** → **BUG**: CreateSessionDialog appears AGAIN
   even though `sid-A` is already open and just needs to be resumed.

## Root cause

The Rust port's BoardView Shift+Right path uses a **per-work-unit
attachment** lookup (`BoardStore::session_for(work_unit_id)`) to decide
the dispatch target, then `dispatch_rpc024.rs::handle_open_agent_view`
unconditionally mounts CreateSessionDialog whenever that lookup returns
`None`. The **GLOBAL open-session list is never consulted on the
Shift+Right path**.

### Code paths (current state)

**`codelet/fspec-tui/src/views/board.rs:117-122`**

```rust
if key.code == KeyCode::Right && key.modifiers.contains(KeyModifiers::SHIFT) {
    let target = self.selected_session(store);   // ← per-work-unit lookup
    self.emit(Action::OpenAgentView(target));
    return EventResult::consumed();
}

fn selected_session(&self, store: &BoardStore) -> Option<SessionId> {
    let unit = store.selected_work_unit()?;
    store.session_for(&unit.id).cloned()         // ← work_unit_id → session_id map
}
```

**`codelet/fspec-tui/src/app/dispatch_rpc024.rs:41-57`**

```rust
pub(crate) fn handle_open_agent_view(&mut self, target: Option<SessionId>) {
    match target {
        Some(sid) => {
            self.agent_view_store.set_navigation_target(Some(sid));
            self.navigator.active_view = ViewMode::Agent;
        }
        None => {
            // ← UNCONDITIONAL dialog mount when work unit has no attachment
            self.agent_view_store.request_create_session_dialog_no_auto();
            self.handle_open_create_session_dialog(None);
        }
    }
}
```

### TS canonical contract (DeepSearch-confirmed)

**`src/tui/components/BoardView.tsx:347-356`** — Shift+Right handler
takes NO work-unit parameter:

```typescript
if (key.shift && key.rightArrow) {
    handleShiftRight();   // ← no work unit reference
    return true;
}
```

**`src/tui/hooks/useSessionNavigation.ts:48-62`** — branches on the
result of a GLOBAL probe:

```typescript
const handleShiftRight = useCallback(() => {
    const result = navigateRight();
    switch (result.type) {
        case 'session':       onNavigate(result.sessionId); break;
        case 'create-dialog': openCreateSessionDialog();    break;
        case 'board':         /* unreachable */              break;
    }
}, ...);
```

**`src/tui/utils/sessionNavigation.ts:32-40`** — the global probe:

```typescript
export function navigateRight(): NavigationResult {
    const next = sessionGetNext();              // ← Rust SessionManager IndexMap cursor
    if (next) return { type: 'session', sessionId: next };
    else      return { type: 'create-dialog' };
}
```

`sessionGetNext()` walks Rust's `SessionManager` `IndexMap` from the
currently active cursor. With no active session (after `setViewMode('board')`
on shift+left), the cursor is unanchored and `get_next` returns the first
session in insertion order (or `None` if empty).

The per-work-unit attachment lookup
(`useFspecStore.getState().getAttachedSession(workUnit.id)`) only happens
inside the **Enter** handler (`BoardView.tsx:553`), never on Shift+Right.

## Behavioural comparison table

| State on Shift+Right | TS canonical | Rust current (BUG) | Rust target |
|---|---|---|---|
| open_sessions=[], W₀ unattached | Show dialog | Show dialog ✓ | Show dialog ✓ |
| open_sessions=[A], W₀ unattached | Navigate to A | **Show dialog** ✗ | Navigate to A |
| open_sessions=[A], W₀ attached to A | Navigate to "next" (A or wrap) | Navigate to A | Navigate to A |
| open_sessions=[A, B], W₀ unattached | Navigate to A | **Show dialog** ✗ | Navigate to A |
| open_sessions=[A, B], W₀ attached to B | Navigate to "next" | Navigate to B | Navigate to A (first global) |

The two `✗` rows are the user's complaint. The fix MUST cover both.

## Fix design

### New helper on AgentViewStore

```rust
// codelet/fspec-tui/src/store/agent_view.rs
impl AgentViewStore {
    /// RPC-097 reopen #2: mirror TS `sessionGetFirst()` — return the
    /// first open session id, if any. Used by BoardView Shift+Right to
    /// resume an existing session before falling through to the
    /// CreateSessionDialog overlay.
    pub fn first_open_session_id(&self) -> Option<SessionId> {
        self.open_sessions.first().map(|c| c.id.clone())
    }
}
```

### Modify `handle_open_agent_view(None)` branch

```rust
// codelet/fspec-tui/src/app/dispatch_rpc024.rs
None => {
    // RPC-097 reopen #2: BEFORE mounting CreateSessionDialog, probe
    // the global open-session list. If any session is already open,
    // navigate to it (TS `sessionGetNext()` semantics). Only when
    // zero sessions are open does the dialog overlay BoardView.
    if let Some(sid) = self.agent_view_store.first_open_session_id() {
        self.agent_view_store.set_navigation_target(Some(sid));
        self.navigator.active_view = ViewMode::Agent;
    } else {
        // No open sessions: preserve RPC-097 reopen #1 contract —
        // dialog overlays BoardView, active_view stays Board.
        self.agent_view_store.request_create_session_dialog_no_auto();
        self.handle_open_create_session_dialog(None);
    }
}
```

### `board.rs` Shift+Right — keep existing per-work-unit fast path

We keep `selected_session(store)` as a fast path so that pressing
Shift+Right on a work unit `W` that has an attached session `sid-W`
jumps to `sid-W` directly. The new fallback only fires when the focused
work unit has no attachment — exactly the buggy case.

This is a **superset** of TS Shift+Right semantics:
- Rust: per-work-unit-attachment first → global session list → dialog.
- TS:   global session list only → dialog.

Both converge in the user-reported scenario: when the focused work
unit has no attachment AND a session is open globally, we resume that
session.

## Affected files

| File | Change |
|---|---|
| `codelet/fspec-tui/src/store/agent_view.rs` | +6 lines: `first_open_session_id()` helper + unit test |
| `codelet/fspec-tui/src/app/dispatch_rpc024.rs` | +5/-2 lines: probe `first_open_session_id()` in `None` branch |
| `codelet/fspec-tui/tests/shift_right_create_session_dialog_rpc097.rs` | +3 scenarios (RPC-097 reopen #2 cases) |
| `spec/features/agentview-shift-right-create-session-dialog.feature` | +3 scenarios |

## Test plan (red → green)

1. **`boardview_shift_right_with_open_session_resumes_existing_session`**
   - Seed `open_sessions = [sid-A]`, BoardView focused on unattached W₀.
   - Press Shift+Right.
   - Assert: `active_view == Agent`, `navigation_target == Some(sid-A)`,
     compositor does NOT contain CREATE_SESSION_DIALOG_ID.

2. **`boardview_shift_right_with_two_open_sessions_resumes_first`**
   - Seed `open_sessions = [sid-A, sid-B]`, BoardView focused on
     unattached W₀.
   - Press Shift+Right.
   - Assert: `active_view == Agent`, `navigation_target == Some(sid-A)`,
     no dialog mounted.

3. **`boardview_shift_right_zero_open_sessions_still_mounts_dialog`**
   - Seed `open_sessions = []`, BoardView focused on unattached W₀.
   - Press Shift+Right.
   - Assert: `active_view == Board`, compositor CONTAINS
     CREATE_SESSION_DIALOG_ID. (regression-guard for RPC-097 reopen #1)

4. **`shift_left_then_shift_right_resumes_open_session_end_to_end`** —
   full sequence test:
   - Start on Board, focus W₀ (unattached).
   - Shift+Right → mounts dialog (sessions empty).
   - Confirm Yes → creates sid-A, switches to Agent.
   - Shift+Left → back to Board.
   - Shift+Right → should resume sid-A, NO dialog.

## Out-of-scope (deferred follow-ups)

- True TS-cursor parity for "next after current" semantics
  (`sessionGetNext` honors a cursor; this fix uses `sessionGetFirst`
   semantics which is the right behavior when coming from Board where
   no cursor is active).
- Cycling forward/backward through multiple sessions from BoardView —
  the user's complaint is about empty→resume, not multi-cycling.
