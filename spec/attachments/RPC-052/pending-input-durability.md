# RPC-052 — Pending-input draft persistence on session switch

**Parent:** RPC-030 · **Phase:** 6.7 · **Estimate:** 2 pts · **Depends on:** RPC-051

## Goal

Upgrade the per-session input-draft state from in-memory only (`SessionContext.input_draft`, snapshotted on Shift+←/→) to backend-backed durable storage so drafts survive across reloads and session destroys.

## Current state (post-RPC-024)

`SessionContext.input_draft: String` (in `codelet/fspec-tui/src/store/agent_view/session_context.rs`) holds the per-session draft. The current implementation:

- `App::handle_session_cycle(delta)` (in `dispatch_rpc024.rs`):
  1. Snapshot `self.navigator.agent.input.value()` into outgoing session's `input_draft`.
  2. `cycle_session(delta)`.
  3. Seed `self.navigator.agent.input` with incoming session's `input_draft`.

This works for live session switching but loses drafts if:
- The fspec process restarts.
- The session is destroyed and recreated.
- Multiple clients are connected to the same daemon (drafts don't sync).

## Trait wiring (already in RPC-037)

- `FspecBackend::set_pending_input(session_id, Option<String>) -> Result<()>`
- `FspecBackend::get_pending_input(session_id) -> Result<Option<String>>`

Backend storage: `BackgroundSession::pending_input: RwLock<Option<String>>` (`codelet/sessions/src/background_session.rs` line 510 in original `session_manager.rs`).

The TS frontend already calls `sessionSetPendingInput` / `sessionGetPendingInput` for the same purpose.

## Work

### Step 1 — Debounced sync from MultiLineInput

`MultiLineInput` currently has an `on_change` callback. Wire it to dispatch `Action::PendingInputChanged { text }` on every change:

```rust
// In codelet/fspec-tui/src/components/multi_line_input.rs
self.on_change(|new_value| {
    sender.send(Action::PendingInputChanged { text: new_value });
});
```

Dispatcher debounces:

```rust
Action::PendingInputChanged { text } => {
    let Some(session_id) = self.agent_view_store.current_session().cloned() else { return };
    let backend = self.backend.clone();
    // 300ms debounce: cancel any in-flight save and schedule a new one
    self.debounce_pending_input_save(session_id, text, Duration::from_millis(300));
}
```

`debounce_pending_input_save` uses a `JoinHandle` stored in `AgentViewStore::pending_input_save_handle` — abort if exists, then spawn new task that sleeps 300ms then calls `backend.set_pending_input(session_id, Some(text)).await`.

### Step 2 — Hydrate on session activation

When a session becomes active (`cycle_session` or `attach_to_session`):

```rust
fn on_session_activated(&mut self, session_id: SessionId) {
    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    tokio::spawn(async move {
        if let Ok(Some(draft)) = backend.get_pending_input(session_id.clone()).await {
            let _ = sender.send(Action::SeedPendingInput { session_id, text: draft });
        }
    });
}

Action::SeedPendingInput { session_id, text } => {
    if self.agent_view_store.current_session() == Some(&session_id) {
        self.navigator.agent.input.set_value(&text);
    }
    // Also update the local SessionContext.input_draft for snapshot consistency.
    if let Some(ctx) = self.agent_view_store.session_context_mut_for(&session_id) {
        ctx.input_draft = text;
    }
}
```

### Step 3 — Submit clears the draft

On `Action::InputSubmitted`, after dispatching `send_input`, clear the draft:

```rust
let backend = self.backend.clone();
let id = session_id.clone();
tokio::spawn(async move {
    let _ = backend.set_pending_input(id, None).await;
});
```

## Acceptance criteria

1. Typing into `MultiLineInput` triggers a debounced `set_pending_input` after 300ms idle.
2. Session activation seeds the live input from `get_pending_input`.
3. Submitting clears the draft (`set_pending_input(None)`).
4. Restart smoke test: type draft, kill fspec, restart, attach same session → draft is restored.
5. Multi-client smoke test: client A types draft, client B connected to same daemon sees the draft on session activation.
6. Integration test in `codelet/fspec-tui/tests/pending_input_durability.rs` covers happy path + restart scenario.

## Risks

- Debounce + tokio cancellation: ensure aborting an in-flight `set_pending_input` doesn't leave the backend with a stale value. Use a monotonic version counter or just accept "last write wins" (typical).
- Network blip on WS backend: `set_pending_input` returning Err shouldn't crash the UI. Log and continue.

## Out of scope

- Cross-machine sync (no fspec daemon distribution).
