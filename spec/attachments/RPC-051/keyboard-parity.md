# RPC-051 — Keyboard shortcut parity (`Shift+↑/↓` history, `Ctrl+R` search, `Esc` interrupt cascade)

**Parent:** RPC-030 · **Phase:** 6.5 · **Estimate:** 3 pts · **Depends on:** RPC-050

## Goal

Close keyboard parity gaps between the Rust AgentView and the TS Ink AgentView.

## Current state

| Shortcut | Status |
|---|---|
| `Shift+←/→` session navigation | ✓ wired (RPC-024) |
| `Shift+↑/↓` history recall | ✓ wired (RPC-025) — but verify against TS behaviour |
| `Tab` turn-selection mode | ✓ wired |
| `Ctrl+R` history search | ✓ wired in `dispatch.rs` line 195 (RPC-026) |
| `Esc` priority cascade | ✗ partial — only handles popup/dialog/mode-view dismiss, then goes to BackToBoard. Needs to call `backend.interrupt(session_id)` when session is Running. |

## Esc cascade — full priority order

```
1. If slash popup or file popup is open → close it.
2. Else if a dialog (Confirm/HITL/Model/Thinking/Role/Help) is open → dismiss it.
3. Else if a mode view (Resume/Search) is active → close it (`Action::CloseResumeView` / `CloseSearchView`).
4. Else if a session is currently Running (`agent_view_store.session_status_for(&id) == Running`)
   → call `backend.interrupt(session_id)`. Do NOT navigate back.
5. Else → `Action::BackToBoard`.
```

In `codelet/fspec-tui/src/views/agent/dispatch.rs::handle_event`, find the `Default Esc → Action::BackToBoard` branch (~line 230) and insert the interrupt check before the BackToBoard:

```rust
KeyCode::Esc => {
    // Cascade order (already handled above): popup → dialog → mode view.
    let session_id = self.agent_view_store.current_session().cloned();
    if let Some(id) = session_id {
        let status = self.agent_view_store.session_status_for(&id).unwrap_or(SessionStatus::Idle);
        if matches!(status, SessionStatus::Running | SessionStatus::Compacting) {
            let backend = self.backend.clone();
            let id_clone = id.clone();
            tokio::spawn(async move {
                let _ = backend.interrupt(id_clone).await;
            });
            // Don't navigate back — let the user see the interrupt land.
            return EventOutcome::Consumed;
        }
    }
    self.emit_action(Action::BackToBoard);
}
```

## Shift+↑/↓ verification against TS

The TS history-recall behaviour:

1. First `Shift+↑` snapshots the live input draft and replaces it with the most recent prompt.
2. Subsequent `Shift+↑` walk back through history.
3. `Shift+↓` walks forward; reaching the end restores the snapshotted draft.
4. Typing exits history mode and clears the recall pointer.

Confirm `AgentViewStore::history_state_by_session` + `cached_history_snapshot` (RPC-025) match this. Add a test if missing.

## `Ctrl+R` verification

Currently `dispatch.rs` line 195 emits `Action::OpenSearchView` on `Ctrl+R` when no popup/dialog is active. TS parity: Ctrl+R focuses the search input immediately. Confirm the Rust SearchHistoryView has the input focused on mount. If not, add `focus_on_mount=true`.

## Acceptance criteria

1. Esc cascade has 5 levels, in the order documented above.
2. Esc on a Running session interrupts via `backend.interrupt(session_id)` but does NOT navigate back.
3. Esc on an Idle session (no popup/dialog/mode-view) navigates back to Board.
4. Shift+↑/↓ matches TS behaviour (snapshot + recall + restore).
5. Ctrl+R opens SearchHistoryView with the input field focused.
6. Integration test in `codelet/fspec-tui/tests/keyboard_cascade.rs` drives all 5 Esc levels.

## Out of scope

- Visual feedback for interrupt-in-flight (covered by `SessionStateChange { state: Interrupted }` chunk).
