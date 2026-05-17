# RPC-025 — AST research: integration points for history lift + per-session recall

Generated 2026-05-17 during the SPECIFYING phase of RPC-025 (child b of
RPC-021). All scan results were produced via `AstGrep` against the live
codebase at the time of writing.

## 1. Existing FspecBackend trait surface (codelet/fspec-tui/src/transport/mod.rs)

`async fn ...` decls already present (RPC-009 through RPC-024):

```
list_work_units, list_sessions, create_session, send_input, interrupt,
work_units_rx, chunks_rx, logs_rx, health, checkpoint_counts,
move_work_unit_up, move_work_unit_down, get_model_info,
get_thinking_level, get_workspace_info, search_files,
request_manual_reconnect (default no-op).
```

RPC-025 adds THREE methods at the end of the trait (preserve existing
ordering — new methods land after `search_files`, before the default
`request_manual_reconnect`):

```rust
async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()>;
async fn persistence_get_history(&self, session: SessionId, limit: u32) -> Result<Vec<String>>;
async fn persistence_search_history(&self, query: String) -> Result<Vec<HistoryMatch>>;
```

`HistoryMatch` is imported from `codelet_rpc_types`.

## 2. Existing FspecService surface (codelet/rpc/src/lib.rs)

Same pattern — RPC-013/017 method shapes:

```
async fn move_work_unit_up(id: String) -> Result<(), String>;
async fn move_work_unit_down(id: String) -> Result<(), String>;
```

RPC-025 adds parallel decls returning `Result<_, String>` (tarpc
convention used in this crate):

```rust
async fn persistence_add_history(session: SessionId, text: String) -> Result<(), String>;
async fn persistence_get_history(session: SessionId, limit: u32) -> Result<Vec<String>, String>;
async fn persistence_search_history(query: String) -> Result<Vec<HistoryMatch>, String>;
```

`HistoryMatch` is imported from `codelet_rpc_types`.

## 3. Embedded vs WebSocket backend impl pattern

Both backends already have one-line delegate impls for the recent
families. The pattern for RPC-025:

```rust
// embedded.rs
async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()> {
    self.service.persistence_add_history(context::current(), session, text)
        .await
        .map_err(anyhow::Error::msg)?;
    Ok(())
}

// websocket.rs
async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()> {
    let client = self.client.lock().clone().ok_or(BackendError::Disconnected)?;
    client.persistence_add_history(context::current(), session, text)
        .await
        .map_err(anyhow::Error::msg)?
        .map_err(anyhow::Error::msg)
}
```

(The exact `Mutex` access and `?` chain matches the existing
`move_work_unit_up` impl in each file — see embedded.rs:105 and
websocket.rs:276 for the template.)

## 4. App::dispatch routing (codelet/fspec-tui/src/app/dispatch.rs)

Current state (after RPC-024):
- `Action::SessionPrev / SessionNext` → `self.handle_session_cycle(±1)`
  via `dispatch_rpc024.rs`.
- `Action::ScrollbackPageUp / PageDown` → wired (RPC-024).
- `Action::InputSubmitted(text)` → `self.handle_input_submitted(text.clone())`.
- The catch-all `_ => {}` arm at line 309 currently swallows
  `Action::HistoryPrev` and `Action::HistoryNext` (emitted by RPC-019's
  MultiLineInput on Shift+↑/↓).

RPC-025 adds two routing arms BEFORE the `_ => {}` catch-all, plus a
sibling helper file `app/dispatch_rpc025.rs` (under 300 LoC) holding
`handle_history_prev` and `handle_history_next` so `dispatch.rs` does
not grow past its current 317-LoC size:

```rust
Action::HistoryPrev => self.handle_history_prev(),
Action::HistoryNext => self.handle_history_next(),
```

`handle_input_submitted` (currently in dispatch.rs) is extended to fire
`tokio::spawn(async move { let _ = backend.persistence_add_history(...).await; })`
and to call `self.agent_view_store.reset_history_state(&session)`.

## 5. AgentViewStore touch points (codelet/fspec-tui/src/store/agent_view.rs)

After RPC-024 the store is 281 LoC. RPC-025 must add:
- `history_state_by_session: HashMap<SessionId, HistoryNavState>` field
- `cached_history_snapshot: HashMap<SessionId, Vec<String>>` field
- `pub fn history_state_for(&mut self, &SessionId) -> &mut HistoryNavState`
  (or split accessor + mutator)
- `pub fn reset_history_state(&mut self, &SessionId)`
- `pub fn set_history_snapshot(&mut self, SessionId, Vec<String>)`
- `pub fn cached_history_snapshot(&self, &SessionId) -> Option<&Vec<String>>`

To stay under 300 LoC, `HistoryNavState` is split into a sibling module
`codelet/fspec-tui/src/store/agent_view/history_state.rs` (mirrors the
RPC-024 split that put `SessionContext` into
`store/agent_view/session_context.rs`).

## 6. codelet_napi → codelet_core lift surface

Current locations:
- `codelet/napi/src/persistence/history.rs` (158 LoC) — `HistoryStore`.
- `codelet/napi/src/persistence/types.rs:227` — `HistoryEntry`.
- `codelet/napi/src/persistence/mod.rs:525-552` — `add_history_entry`,
  `get_history`, `search_history` helpers.
- `codelet/napi/src/persistence/napi_bindings.rs:265-290` — `#[napi]`
  exports `persistence_add_history`, `persistence_get_history`,
  `persistence_search_history`.

Target locations after lift:
- `codelet/core/src/persistence/mod.rs` (new, <100 LoC) — re-exports.
- `codelet/core/src/persistence/history.rs` (new, <300 LoC) — owns
  `HistoryStore` + helper fns + (lifted) `HistoryEntry`.
- The napi side becomes thin re-exports / one-line delegates so the JS
  surface stays byte-identical.

`HistoryEntry::to_history_match()` is a new method on the lifted
`HistoryEntry` that returns a `codelet_rpc_types::HistoryMatch`
(timestamp formatted via `.to_rfc3339()`).

## 7. Cross-transport-parity test template

The existing RPC-024 tests at
`codelet/fspec-tui/tests/app_dispatch_rpc024.rs` and RPC-020 parity
tests at `codelet/fspec-tui/tests/parity_search_files_rpc020.rs` (or
similar) provide the template: construct an EmbeddedFspecBackend
against a temp HOME, spin up the WebSocket server in-process,
construct a WebSocketFspecBackend, and assert both return the same
result for the same call.

## 8. Action enum touch (codelet/fspec-tui/src/components/mod.rs)

`Action::HistoryPrev` and `Action::HistoryNext` already exist at lines
228 and 231 (added by RPC-019). No new Action variants are required for
RPC-025 — only the routing arms in `dispatch.rs`.

## 9. Forbidden imports invariant

The `tests/source_shape_*` scans assert that no file under
`codelet/fspec-tui/src/views/` imports `codelet_core`, `codelet_napi`,
`tarpc`, or `tokio_tungstenite`. The HistoryNavState + new store
fields live under `store/`, not `views/`, so this invariant is
preserved by construction. The new `app/dispatch_rpc025.rs` lives
under `app/`, which is also outside the restricted `views/` scope.
