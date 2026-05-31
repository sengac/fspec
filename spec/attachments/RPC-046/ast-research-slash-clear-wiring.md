# AST Research — RPC-046: `/clear` slash command end-to-end

**Card:** RPC-046 · **Parent:** RPC-030 · **Phase:** 6.4

## Goal

Wire `SlashCommandAction::Clear` (in `app/dispatch_rpc020.rs::handle_slash_command`) to call
`backend.clear_history(session_id)` in addition to the existing local scrollback reset, then
emit a `[notice]` (or `[error]`) line into the originating session's scrollback when the RPC
round-trip resolves.

## Findings

### 1. Current `/clear` handler

File: `codelet/fspec-tui/src/app/dispatch_rpc020.rs`

```rust
SlashCommandAction::Clear => {
    self.navigator
        .agent
        .reset_scrollback(&mut self.agent_view_store);
}
```

Only resets the local scrollback (the focused session). No backend call. No notice.

### 2. `FspecBackend::clear_history` already exists end-to-end

| Layer | Location | Shape |
|-------|----------|-------|
| Trait (default impl) | `codelet/fspec-tui/src/transport/mod.rs:290` | `async fn clear_history(&self, _session_id: SessionId) -> Result<()> { Ok(()) }` |
| Embedded backend | `codelet/fspec-tui/src/transport/embedded.rs:301` | Delegates to `client.clear_history(context::current(), session_id)` |
| WebSocket backend | `codelet/fspec-tui/src/transport/websocket.rs:575` | Same, plus `BackendError::Disconnected` guard |

The trait was widened in RPC-037; both transports already route through `FspecService::clear_history`. **No transport-level work** required for RPC-046.

### 3. Notice plumbing already exists per-session

| Helper | Location | Purpose |
|--------|----------|---------|
| `AgentView::push_line` | `codelet/fspec-tui/src/views/agent.rs:151` | Push a raw line into the **focused** session's scrollback |
| `SessionContext::push_line` | `codelet/fspec-tui/src/store/agent_view/session_context.rs:79` | Push a raw line into a **specific** session's scrollback |

For RPC-046 we need the **per-session** variant (the spawned task races with session switches; we must always land the notice on the originating session, not whoever is focused when the response arrives).

### 4. Pattern precedent: spawned-task → action-bus dispatch

`codelet/fspec-tui/src/app/dispatch_rpc022.rs::handle_thinking_level_selected` (lines 115–135) is the closest peer. Pattern:

```rust
let backend = self.backend.clone();
let action_tx = self.action_tx.clone();
let sid = session_id.clone();
let handle = tokio::spawn(async move {
    let _ = backend.set_thinking_level(sid, level).await;
    if let Ok(fresh) = backend.get_thinking_level(sid_for_refresh.clone()).await {
        let _ = action_tx.send(Action::ThinkingLevelLoaded(sid_for_refresh, fresh));
    }
});
self.pending_tasks.push(handle);
```

And `codelet/fspec-tui/src/app/dispatch_rpc045.rs::spawn_fspec_command_runner` (lines 128–151) shows the round-trip-then-action pattern.

For `/clear` we mirror this: spawn → await `clear_history` → dispatch `Action::EmitSessionNotice(session_id, text)`.

### 5. New Action variant required

No `Action::EmitNotice` / `Action::EmitSessionNotice` exists today (verified via `grep -r 'EmitNotice\\|emit_notice' codelet/fspec-tui/src/`). We must:

1. Add `EmitSessionNotice(SessionId, String)` to `Action` enum in `codelet/fspec-tui/src/components/mod.rs`.
2. Add a routing arm in `App::dispatch` (or a helper in `dispatch_rpc020.rs`) that calls `agent_view_store.session_context_mut_for(&id).map(|ctx| ctx.push_line(text))`.

### 6. Synchronous-runtime guard required

`tokio::runtime::Handle::try_current().is_err() { return; }` is the established guard for unit tests dispatched outside a tokio runtime — see `dispatch_rpc022.rs:40, 93, 120, 149` and `dispatch_rpc045.rs:133`. We MUST guard before the `tokio::spawn` so the scrollback-reset scenario can be exercised by a plain `#[test]` (no `#[tokio::test]`).

### 7. Test fixtures

`tests/common/mod.rs` `MockBackend` does NOT yet override `clear_history` — it falls through to the default `Ok(())` impl. RPC-046 tests need:

- Scripting `clear_history` to return `Ok(())` or `Err(...)`.
- Counter for `clear_history` calls.
- Capture of the last `(SessionId)` passed.

Mirror the `set_thinking_level` / `set_session_role` helper pattern in `MockBackend`.

## Implementation plan

1. **`Action::EmitSessionNotice(SessionId, String)`** — new enum variant.
2. **`App::dispatch`** — new arm calls `agent_view_store.session_context_mut_for(&id).map(|ctx| ctx.push_line(text))`.
3. **`SlashCommandAction::Clear` arm in `dispatch_rpc020.rs`** — capture `session_id`, reset scrollback (existing), then `tokio::spawn` → await `backend.clear_history` → emit success/error notice via `Action::EmitSessionNotice`.
4. **`MockBackend`** — add `clear_history_calls`, `last_clear_history_session`, `clear_history_error` scripting + override `clear_history` impl.
5. **`tests/slash_clear_rpc046.rs`** — six scenarios mirroring the feature file:
   - synchronous scrollback reset
   - backend.clear_history is called once with the right session_id
   - success notice text
   - error notice text
   - no-session no-op
   - background-session isolation (focus s-1, clear, assert s-2 untouched)

## Risks

- Lifecycle: the spawned task can outlive the user's session switch. Routing via `Action::EmitSessionNotice(session_id, ...)` ensures the notice lands on the right SessionContext regardless of current focus — verified by the background-session isolation scenario.
- Test determinism: `tokio::time::timeout(Duration::from_secs(1), ...)` poll loop matches the RPC-045 pattern for spawned-task round-trip assertions.

## Out of scope

- Confirm dialog before destructive clear (TS does not have one either — per RPC-046 attachment).
- Compositor-level notice routing (the existing per-session scrollback path is sufficient).
