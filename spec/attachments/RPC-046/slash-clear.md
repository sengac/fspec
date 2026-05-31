# RPC-046 — `/clear` slash command end-to-end

**Parent:** RPC-030 · **Phase:** 6.4 · **Estimate:** 2 pts · **Depends on:** RPC-045

## Goal

Wire `SlashCommandAction::Clear` to call `backend.clear_history(session_id)` in addition to the existing local scrollback reset.

## Current state

`codelet/fspec-tui/src/app/dispatch_rpc020.rs::handle_slash_command` already handles `Clear` by calling `self.navigator.agent.reset_scrollback(&mut self.agent_view_store)`. This clears UI state only — it does NOT clear the backend's session history.

The TS frontend's `handleClearCommand()` (referenced from `AgentView.tsx` line 2730) clears both the conversation array AND calls the backend to clear persisted history.

## Work to do

Extend `handle_slash_command(Clear)`:

```rust
SlashCommandAction::Clear => {
    // 1. Reset local scrollback (existing).
    self.navigator.agent.reset_scrollback(&mut self.agent_view_store);

    // 2. Call backend to clear persisted history.
    if let Some(session_id) = self.agent_view_store.current_session().cloned() {
        let backend = self.backend.clone();
        let store_handle = /* a sender for emitting notices */;
        tokio::spawn(async move {
            match backend.clear_history(session_id.clone()).await {
                Ok(()) => {
                    let _ = store_handle.send(Action::EmitNotice {
                        session_id,
                        text: "[notice] /clear: history cleared".to_string(),
                    });
                }
                Err(e) => {
                    let _ = store_handle.send(Action::EmitNotice {
                        session_id,
                        text: format!("[error] /clear failed: {e}"),
                    });
                }
            }
        });
    }
}
```

## Trait wiring (already done in RPC-037)

`FspecBackend::clear_history(&self, session_id: SessionId) -> Result<()>` exists on both `EmbeddedFspecBackend` and `WebSocketFspecBackend`.

## Acceptance criteria

1. `/clear` resets local scrollback AND calls `backend.clear_history(session_id)`.
2. Success emits a `[notice] /clear: history cleared` line.
3. Failure emits a `[error] /clear failed: {reason}` line.
4. Integration test in `codelet/fspec-tui/tests/slash_clear.rs`:
   - Stub backend with `clear_history → Ok(())` → assert scrollback reset + notice emitted.
   - Stub backend with `clear_history → Err("boom")` → assert error notice.

## Out of scope

- Confirm dialog before destructive clear (out of parity scope — TS doesn't have one either).
