# RPC-096 — Shift+Left/Right end-of-list parity research

## Context

User report (turn 5575):

> when I go shift+left and shift+right, none of that functionality works in
> the rust port

The natural interpretation ("text selection extension") was ruled out by the
first DeepSearch — the TypeScript Ink original never had selection. The actual
gap is the **session-navigation contract** that RPC-024 only partially ported.

This document captures the TS contract verbatim and itemises the three concrete
divergences in the current Rust port so the ACDD test pass can target them
precisely.

---

## 1. TypeScript contract — `useSessionNavigation`

### Hook entry points

File: `src/tui/hooks/useSessionNavigation.ts`

```ts
// lines 48–62
const handleShiftRight = useCallback(() => {
  const result = navigateRight();
  switch (result.type) {
    case 'session':        onNavigate(result.sessionId); break;
    case 'create-dialog':  openCreateSessionDialog();    break;
    case 'board':          /* unreachable on right */    break;
  }
}, [onNavigate, openCreateSessionDialog]);

// lines 64–79
const handleShiftLeft = useCallback(() => {
  const result = navigateLeft();
  switch (result.type) {
    case 'session': onNavigate(result.sessionId); break;
    case 'board':
      clearActiveSession();
      onNavigateToBoard();
      break;
    case 'create-dialog': /* unreachable on left */ break;
  }
}, [onNavigate, onNavigateToBoard]);
```

### Navigation primitives

File: `src/tui/utils/sessionNavigation.ts`

```ts
// lines 32–40
export function navigateRight(): NavigationResult {
  const next = sessionGetNext();
  if (next) return { type: 'session', sessionId: next };
  else      return { type: 'create-dialog' };
}

// lines 48–56
export function navigateLeft(): NavigationResult {
  const prev = sessionGetPrev();
  if (prev) return { type: 'session', sessionId: prev };
  else      return { type: 'board' };
}
```

### Backing Rust (already correct under BUG-124)

File: `codelet/sessions/src/navigation.rs`

```rust
// lines 43–53 (build_navigation_list)
sessions.keys().copied().collect()
```

A flat IndexMap-insertion-order walk. The `chain_of_command` parameter is kept
for ABI stability and no longer consulted.

`get_next_target` returns `Session(uuid)` while inside the list and
`CreateDialog` once you walk off the end. `get_prev_target` returns
`Session(uuid)` while inside the list and `Board` once you walk off the start
(or if `active_session == None`).

### Behaviour matrix (TS, authoritative)

| State                                 | Shift+Right                       | Shift+Left                         |
|---------------------------------------|-----------------------------------|------------------------------------|
| 0 sessions, on Board                  | open CreateSessionDialog          | stay on Board (no-op-ish)          |
| 1 session, on Board                   | navigate into that session        | stay on Board                      |
| 1 session, attached to it             | open CreateSessionDialog          | exit to Board                      |
| N sessions, mid-list                  | navigate to next session          | navigate to previous session       |
| N sessions, at last index             | **open CreateSessionDialog**      | navigate to previous session       |
| N sessions, at first index            | navigate to next session          | **exit to Board**                  |

### What Shift+Right does NOT do

It does **not** auto-create a session — it only flips
`sessionStore.showCreateSessionDialog = true` (sessionStore.ts:242–247) and the
user must still confirm in the modal to actually call `createSession()`.
Likewise the hook never attaches a work unit; that wiring is the responsibility
of the dialog confirmation path.

---

## 2. Rust port — current state

### Dispatcher routing

File: `codelet/fspec-tui/src/views/agent/dispatch.rs`

```rust
// lines 25–33 — shift_arrow_to_action
KeyCode::Up    => Some(Action::HistoryPrev),
KeyCode::Down  => Some(Action::HistoryNext),
KeyCode::Left  => Some(Action::SessionPrev),
KeyCode::Right => Some(Action::SessionNext),

// lines 199–204 — emission site
if key.modifiers.contains(KeyModifiers::SHIFT) {
    if let Some(action) = Self::shift_arrow_to_action(key.code) {
        self.emit(action);
        return EventResult::consumed();
    }
}
```

### App handler

File: `codelet/fspec-tui/src/app/dispatch_rpc024.rs`

`handle_session_cycle(delta: isize)`:

1. Snapshot outgoing input draft.
2. `agent_view_store.cycle_session(delta)`.
3. Restore incoming session's draft.
4. `spawn_load_supervisors(incoming_session)` for the badge.

### Wrap-around in the store

File: `codelet/fspec-tui/src/store/agent_view.rs`

```rust
// lines 166–176
pub fn cycle_session(&mut self, delta: isize) {
    let len = self.open_sessions.len();
    if len <= 1 { return; }
    let len_i = len as isize;
    let cur = self.current_session_index as isize;
    let next = (cur + delta).rem_euclid(len_i);
    self.current_session_index = next as usize;
}
```

`rem_euclid` produces wrap-around. `len <= 1` early-returns to a true no-op.

---

## 3. Three concrete gaps to close

| # | Trigger                                          | TS does                          | Rust does                        |
|---|--------------------------------------------------|----------------------------------|----------------------------------|
| 1 | N sessions, at last index, Shift+Right           | open CreateSessionDialog modal   | wrap to index 0                  |
| 2 | N sessions, at first index, Shift+Left           | exit to BoardView                | wrap to last index               |
| 3 | 1 session attached, Shift+Right                  | open CreateSessionDialog modal   | no-op (early-return)             |
| 3 | 1 session attached, Shift+Left                   | exit to BoardView                | no-op (early-return)             |
| 3 | 0 sessions on Board, Shift+Right                 | open CreateSessionDialog modal   | no-op (empty)                    |

(Gap 3 is the user-visible bug they reported — they have a single attached
session at startup and both shift arrows do nothing.)

---

## 4. Target Rust contract

Add a navigation primitive on `AgentViewStore` modelled after TS `navigateNext`
/ `navigatePrev`:

```rust
pub enum NavTarget {
    Session(usize),  // index into open_sessions
    CreateDialog,    // off the right end
    Board,           // off the left end
}

impl AgentViewStore {
    pub fn navigate_next(&self) -> NavTarget {
        if self.open_sessions.is_empty() {
            return NavTarget::CreateDialog;
        }
        let next = self.current_session_index + 1;
        if next >= self.open_sessions.len() {
            NavTarget::CreateDialog
        } else {
            NavTarget::Session(next)
        }
    }
    pub fn navigate_prev(&self) -> NavTarget {
        if self.open_sessions.is_empty() {
            return NavTarget::Board;
        }
        if self.current_session_index == 0 {
            NavTarget::Board
        } else {
            NavTarget::Session(self.current_session_index - 1)
        }
    }
}
```

Then in `App::dispatch_rpc024.rs::handle_session_cycle(delta)`:

```rust
let target = if delta < 0 {
    self.agent_view_store.navigate_prev()
} else {
    self.agent_view_store.navigate_next()
};
match target {
    NavTarget::Session(idx) => self.switch_to_session_index(idx),
    NavTarget::CreateDialog => self.open_create_session_dialog(),
    NavTarget::Board        => self.exit_agent_view_to_board(),
}
```

Where:

- `switch_to_session_index(idx)`: the existing draft-snapshot / cycle /
  draft-restore / load-supervisors flow, parameterised on the target index
  instead of `cycle_session(delta)`.
- `open_create_session_dialog()`: pushes the existing
  `CreateSessionDialog` (RPC-026 picker family) onto the compositor at
  `Priority::Critical`. If the dialog doesn't yet exist in Rust, this story
  scopes only the "Action::OpenCreateSessionDialog" dispatch path — the dialog
  component itself can be implemented in a follow-up child story; for now,
  satisfy the contract by emitting an Action that future code wires to the
  modal. **Need to verify** whether a CreateSessionDialog already exists in
  `codelet/fspec-tui/src/components/` — if so, reuse it.
- `exit_agent_view_to_board()`: drive the same path that the `Esc` cascade
  uses to leave AgentView and return to BoardView; this is the existing
  `Action::ViewMode(ViewMode::Board)` (or equivalent) dispatch.

`cycle_session` becomes dead code and is removed from `agent_view.rs`.

---

## 5. Source-shape budget

`codelet/fspec-tui/src/store/agent_view.rs` was at the edge of 300 LoC after
RPC-024. Adding `NavTarget` + `navigate_next` + `navigate_prev` is ~25 LoC.
If the file exceeds 300 LoC, extract the navigation primitive to
`codelet/fspec-tui/src/store/agent_view/navigation.rs`.

`codelet/fspec-tui/src/app/dispatch_rpc024.rs` is currently small; replacing
the cycle call with a `match target` block adds ~30 LoC — well within budget.

---

## 6. Tests to write (preview)

1. `navigate_next_returns_create_dialog_when_empty`
2. `navigate_next_returns_create_dialog_at_last_index`
3. `navigate_next_returns_session_when_mid_list`
4. `navigate_prev_returns_board_when_empty`
5. `navigate_prev_returns_board_at_first_index`
6. `navigate_prev_returns_session_when_mid_list`
7. `single_session_shift_right_opens_create_dialog` (integration)
8. `single_session_shift_left_exits_to_board_view` (integration)
9. `n_sessions_at_last_shift_right_opens_create_dialog`
10. `n_sessions_at_first_shift_left_exits_to_board_view`
11. `mid_list_shift_arrows_still_switch_session_with_draft_round_trip`
    (regression for RPC-024)
12. `cycle_session_is_removed_from_public_api` (source-shape regression)
