# RPC-049 — `/resume` durable restore via `restore_session_messages` + `restore_session_token_state`

**Parent:** RPC-030 · **Phase:** 6.4 · **Estimate:** 5 pts · **Depends on:** RPC-048

## Goal

Extend `ResumeSessionView` (already exists from RPC-026) to perform a **durable restore** on selection: call `backend.restore_session_messages(session_id, envelopes)` + `backend.restore_session_token_state(session_id, state)`. Then re-subscribe and seed scrollback from persistence.

## Current state

`ResumeSessionView` (RPC-026) lists sessions via `backend.list_sessions()` and lets the user pick one. Currently it only switches the focused session — it does NOT replay messages from disk into a freshly restored session.

The TS frontend's `handleResumeMode` (triggered by `triggerResumeModeInit` at `AgentView.tsx` line 2736) does:

1. List sessions
2. User picks one
3. Read messages.jsonl envelopes
4. Call `sessionRestoreMessages(sessionId, envelopes)` to seed the agent's in-memory history
5. Call `sessionRestoreTokenState(sessionId, tokenState)` to restore counters
6. Re-subscribe to the chunk stream
7. Seed UI scrollback with rendered envelopes

## Trait wiring (already done in RPC-037)

- `FspecBackend::restore_session_messages(SessionId, Vec<String>) -> Result<()>`
- `FspecBackend::restore_session_token_state(SessionId, TokenRestoreState) -> Result<()>`

## Backend-side: where do envelopes come from?

Two options:

**Option A — Client reads from persistence directly.** The TUI calls a `persistence_get_session_envelopes(session_id) -> Vec<String>` method, then passes those envelopes back via `restore_session_messages`. This requires a new RPC method `persistence_get_session_envelopes` on `FspecBackend`.

**Option B — Backend does it all in one round-trip.** Add `resume_session(session_id) -> Result<()>` that internally loads envelopes + token state. Single RPC.

**Recommendation: Option B.** Simpler client wiring, fewer round-trips. Add the method to `SessionManagerHandle` and `FspecBackend` in this card (since it's purely a refactor of the trait surface added in RPC-037).

## Work

### Step 1 — Add `resume_session` to traits

```rust
fn resume_session(&self, session_id: &SessionId) -> Result<(), String> {
    // Default: load envelopes via codelet_core::persistence::get_session_message_envelopes,
    // load token state from session manifest, call restore_session_messages + restore_session_token_state.
}
```

Implementation in `codelet/sessions/src/lib.rs::impl SessionManagerHandle for SessionManager`:

```rust
fn resume_session(&self, session_id: &SessionId) -> Result<(), String> {
    let uuid = uuid_from(session_id);
    let manifest = codelet_core::persistence::load_session(uuid).map_err(|e| e.to_string())?;
    let envelopes = codelet_core::persistence::get_session_message_envelopes(&manifest)
        .map_err(|e| e.to_string())?;
    let token_state = TokenRestoreState::from(&manifest); // From impl on TokenRestoreState
    self.restore_session_messages(session_id, envelopes)?;
    self.restore_session_token_state(session_id, token_state)?;
    Ok(())
}
```

### Step 2 — ResumeSessionView wiring

In `codelet/fspec-tui/src/views/resume/mod.rs` (or wherever), the `Action::AttachToSession(id)` handler becomes:

```rust
Action::AttachToSession(session_id) => {
    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    tokio::spawn(async move {
        match backend.resume_session(session_id.clone()).await {
            Ok(()) => {
                let _ = sender.send(Action::SessionResumeComplete { session_id });
            }
            Err(e) => {
                let _ = sender.send(Action::EmitNotice {
                    session_id,
                    text: format!("[error] /resume failed: {e}"),
                });
            }
        }
    });
}
```

`Action::SessionResumeComplete` seeds the SessionContext scrollback by replaying envelopes:

```rust
Action::SessionResumeComplete { session_id } => {
    let backend = self.backend.clone();
    // Fetch buffered output for the resumed session
    tokio::spawn(async move {
        let chunks = backend.get_buffered_output(session_id.clone(), 1000).await
            .unwrap_or_default();
        for chunk in chunks {
            let _ = sender.send(Action::StreamChunkReceived { session_id: session_id.clone(), chunk });
        }
    });
    // Mark session as restored in store
    self.agent_view_store.mark_session_restored(&session_id);
}
```

### Step 3 — UI feedback

While restore is in progress, ResumeSessionView shows a "Restoring …" spinner. On success, navigate back to AgentView with the restored session focused.

## Acceptance criteria

1. `resume_session` exists on `SessionManagerHandle`, `FspecService`, and `FspecBackend`.
2. ResumeSessionView selection triggers `backend.resume_session(...)`.
3. On success, the resumed session has its full message history visible in scrollback (loaded via `get_buffered_output`).
4. Token counters in `SessionFooter` reflect the restored state.
5. Failure emits `[error] /resume failed: ...`.
6. Integration test drives a stub backend with a recorded session → assert scrollback matches expected envelopes.

## Risks

- Large sessions (10k+ messages) may take seconds to restore. Show progress indicator.
- `restore_session_messages(envelopes: Vec<String>)` is a string-wire interface — confirm envelopes parse via `serde_json::from_str::<MessageEnvelope>` on the backend side.
- Re-subscribing to the chunk stream must not duplicate the existing subscription. Use idempotent `subscribe()` semantics.

## Out of scope

- Resuming a session that was already running — undefined behaviour, document as "destroys current state".
