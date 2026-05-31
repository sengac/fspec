# RPC-050 — Work-unit context binding (BoardView attach + SessionHeader chip + `/detach`)

**Parent:** RPC-030 · **Phase:** 6.4 + 6.6 · **Estimate:** 5 pts · **Depends on:** RPC-049

## Goal

1. BoardView click/keypress on a work unit attaches it to the active AgentView session.
2. SessionHeader renders a work-unit chip pulled from `backend.get_work_unit_context(session_id)`.
3. `/detach` slash command clears the binding and resets conversation state.

## Trait wiring (already in RPC-037)

- `FspecBackend::set_work_unit_context(SessionId, Option<WorkUnitContext>) -> Result<()>`
- `FspecBackend::get_work_unit_context(SessionId) -> Result<Option<WorkUnitContext>>`

## Work

### Step 1 — BoardView "attach" action

In `codelet/fspec-tui/src/views/board/dispatch.rs` (or similar), add handler for "attach to session":

```rust
Action::AttachWorkUnitToSession { work_unit_id } => {
    let Some(session_id) = self.agent_view_store.current_session().cloned() else {
        self.emit_notice("no active session to attach to — create one first");
        return;
    };
    let Some(wu) = self.board_store.work_unit(&work_unit_id) else {
        return;
    };
    let ctx = WorkUnitContext {
        id: wu.id.clone(),
        title: wu.title.clone(),
        status: wu.status.clone(),
    };
    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    tokio::spawn(async move {
        let _ = backend.set_work_unit_context(session_id.clone(), Some(ctx.clone())).await;
        let _ = sender.send(Action::WorkUnitAttached { session_id, ctx });
    });
    self.navigator.go_to_agent_view();
}
```

Trigger: in BoardView key handler, when a work unit is focused and the user presses `Enter` (or a dedicated `A` key), dispatch `AttachWorkUnitToSession`.

### Step 2 — SessionHeader chip

In `codelet/fspec-tui/src/components/session_header.rs`, render a chip when `agent_view_store.work_unit_context_for(&session_id)` returns `Some(ctx)`:

```
┌──────────────────────────────────────────────┐
│ AgentView · [AUTH-001: User Login (testing)] │
└──────────────────────────────────────────────┘
```

Store side: add to `AgentViewStore`:

```rust
work_unit_context_by_session: HashMap<SessionId, WorkUnitContext>,
```

Updated by `Action::WorkUnitAttached` and `Action::WorkUnitDetached`.

### Step 3 — `/detach` handler

`SlashCommandAction::Detach` currently emits a "not yet implemented" notice. Replace:

```rust
SlashCommandAction::Detach => {
    let Some(session_id) = self.agent_view_store.current_session().cloned() else {
        self.emit_notice("/detach: no active session");
        return;
    };
    let existing = self.agent_view_store.work_unit_context_for(&session_id).cloned();
    if existing.is_none() {
        self.emit_notice("/detach: no work unit attached");
        return;
    }

    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    tokio::spawn(async move {
        let _ = backend.set_work_unit_context(session_id.clone(), None).await;
        let _ = sender.send(Action::WorkUnitDetached { session_id });
    });

    // Mirror TS prepareForNewSession: clear scrollback, reset token state.
    self.navigator.agent.reset_scrollback(&mut self.agent_view_store);
    self.agent_view_store.reset_token_state(&session_id);
}
```

### Step 4 — Store updates on action

```rust
Action::WorkUnitAttached { session_id, ctx } => {
    self.agent_view_store.set_work_unit_context(session_id.clone(), ctx);
}
Action::WorkUnitDetached { session_id } => {
    self.agent_view_store.clear_work_unit_context(&session_id);
}
```

## Acceptance criteria

1. BoardView Enter key on a work unit attaches it to the current AgentView session and navigates to AgentView.
2. SessionHeader renders the work-unit chip when a context is set.
3. `/detach` clears the context, scrollback, and token state.
4. `/detach` with no active session emits a sensible notice.
5. `/detach` with no work unit attached emits a sensible notice.
6. Integration test in `codelet/fspec-tui/tests/work_unit_binding.rs` covers all four paths.

## TS parity reference

`AgentView.tsx` line 2764: `detachFromWorkUnit(currentSessionId)` + `prepareForNewSession()` + clear `conversation` / `tokenUsage`.

## Out of scope

- BoardView right-pane "attach to this session" button (UX polish; covered later).
- Showing all attached sessions per work-unit (a TS feature not yet ported).
