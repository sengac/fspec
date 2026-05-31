# RPC-051 AST Research — Keyboard cascade landing spots

## Esc handlers on the path

```
Event ─► App::handle_event (app/events.rs)
        │
        ├─ topmost_is_disconnect (DisconnectDialog @ Priority::Critical)
        │     → handle_disconnect_dialog_event (consumes q/r only)
        │
        ├─ !topmost_is_critical ⇒ handle_app_shortcut (?, q, Ctrl+D)
        │
        ├─ compositor.handle_event(event)   ← LEVEL 2 dialog dismiss
        │     - HelpDialog::handle_event (components/help_dialog.rs:63)
        │       Priority::Critical, consumes Esc, removes self.
        │     - ModelSelectorDialog::handle_event (components/model_selector_dialog.rs:168)
        │       Priority::Foreground, consumes Esc.
        │     - ThinkingLevelDialog::handle_event (components/thinking_level_dialog.rs:110)
        │       Priority::Foreground, consumes Esc.
        │     - ConfirmDialog (inside resume_view) — handled INSIDE resume_view's
        │       state machine, NOT on the compositor.
        │
        └─ navigator.handle_event ─► AgentView::handle_event (views/agent/dispatch.rs:194)
              │
              ├─ Ctrl+R (no popup / mode-view) → emit OpenSearchView
              ├─ handle_mode_view_key             ← LEVEL 3 mode-view dismiss
              │     - resume_view → CloseResumeView on Esc
              │     - search_view → CloseSearchView on Esc
              ├─ handle_popup_key                 ← LEVEL 1 popup dismiss
              │     - slash_popup → PopupOutcome::Dismiss on Esc
              │     - file_popup → FilePopupOutcome::Dismiss on Esc
              ├─ Esc default arm (line 226)      ← LEVEL 4/5 — REQUIRES CHANGE
              │     CURRENT: self.emit(Action::BackToBoard);
              │     NEW:     self.emit(Action::AgentEscPressed);
              ├─ Ctrl+C → emit Action::Interrupt (line 231)
              └─ Shift+arrows → HistoryPrev / HistoryNext / SessionPrev / SessionNext
```

## Existing Action::Interrupt wiring (already useful for level 4)

`app/dispatch.rs:21-28`:

```rust
Action::Interrupt => {
    if let Some(session) = self.agent_view_store.current_session().cloned() {
        let backend = self.backend.clone();
        tokio::spawn(async move {
            let _ = backend.interrupt(session).await;
        });
    }
}
```

This is the SAME spawn pattern level 4 needs. We can re-use it by having
`handle_agent_esc_pressed` re-dispatch `Action::Interrupt` when status is
Running/Compacting — that keeps the interrupt spawn logic in ONE place.

## SessionStatus readout

`store/agent_view/isolation_state.rs:61`:

```rust
pub fn session_status_for(&self, session: &SessionId) -> Option<&SessionStatus> {
    self.session_status_by_session.get(session)
}
```

The store already tracks status via `Action::SessionStatusChanged` (RPC-045)
and `StreamChunk::SessionStateChange`. No new write path needed.

## SessionStatus variants (codelet-rpc-types)

```
Idle | Running | Paused | Compacting | Interrupted | Cleared
```

Per the attachment, the interrupt cascade fires on `Running` and `Compacting`.

## SearchHistoryView Ctrl+R focus

`views/agent/search_history_view.rs:215-220`:

```rust
KeyCode::Char(c) => {
    self.query.push(c);
    ...
    SearchHistoryViewOutcome::FilterChanged(self.query.clone())
}
```

The view captures char input directly into `query` — input is implicitly
focused on mount (no separate "focus" state machine). The cursor is painted
by `render_title` as a `Modifier::REVERSED` space at the end of the query.
**Conclusion:** no changes needed for level "Ctrl+R focuses input on mount"
beyond pinning the behaviour with a regression test.

## Shift+↑/↓ recall state (RPC-025)

- `app/dispatch_rpc025.rs::handle_history_prev/next` — already wired.
- `store/agent_view.rs::history_state_for/_mut` — already wired.
- Semantics:
  - First Shift+↑: snapshots draft → `cached_draft`, async fetch, snaps `recall_index = Some(0)`.
  - Subsequent Shift+↑: walks back, clamps at `len - 1`.
  - Shift+↓ in recall at index k>0: decrements.
  - Shift+↓ at index 0: restores `cached_draft`, clears `recall_index`.
  - Shift+↓ in live mode (None): no-op.

Matches the TS behaviour described in the attachment. **No code change** —
regression test only.

## Source-shape budget

- `views/agent/dispatch.rs` currently 275 LoC.
- Diff for RPC-051: change one line (Action::BackToBoard → Action::AgentEscPressed).
  Net delta: 0. Budget preserved.
- New file `app/dispatch_rpc051.rs` ≤ 80 LoC (single helper function).
- `app/dispatch.rs` currently 297 LoC. Need to route the new variant via
  the existing `_ => { let _ = self.try_dispatch_rpc022(&action); }` tail
  OR a new `try_dispatch_rpc051` shortcut. Going with the existing tail to
  keep dispatch.rs unchanged in line count.

## MockBackend hooks already present

- `interrupt_calls() -> usize` (common/mod.rs:497)
- `last_interrupt() -> Option<SessionId>` (via `last_interrupt`)
- Status seeding via `agent_view_store_mut().set_session_status(sid, status)`
  (RPC-045 — already used by `agent_view_chunk_dispatch_rpc045.rs`)
