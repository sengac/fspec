# RPC-437 — AST research: TUI-side consumption of SessionEvent::Cleared

Date: 2026-10-12 · Work unit: RPC-437 · Tool: AstGrep (language=rust) over `rust/fspec-tui/src/`

Scope: identify every touch-point the RPC-437 TUI-side implementation must
modify or extend, mirroring the RPC-434 (created/removed) pattern.

## 1. Trait surface — `FspecBackend` (`src/transport/mod.rs`)

AstGrep pattern `fn session_removed_rx($$$ARGS) -> $$$RET { $$$BODY }` (rust) found
the three existing sibling methods RPC-437 must extend:

| match | location | role in RPC-437 |
|---|---|---|
| `fn session_removed_rx(&self) -> broadcast::Receiver<SessionId>` | `src/transport/mod.rs:1011` | the DEFAULT (closed receiver) that `session_cleared_rx` will mirror |
| `fn session_removed_rx(&self) -> broadcast::Receiver<codelet_rpc_types::SessionId>` | `src/transport/embedded.rs:1076` | the embedded override to mirror (zero-cost `handle.session_*_rx()` direct read; `None` manager → closed receiver) |
| `fn session_removed_rx(&self) -> broadcast::Receiver<SessionId>` | `src/transport/websocket.rs:346` | the WS splitter to mirror (per-subscriber `broadcast::channel(SESSION_EVENT_SPLITS_CAPACITY=16)`, fresh `client_session_events_rx()`, spawn splitter task forwarding only the matching variant; `None` client → `empty_broadcast_rx()`) |

New trait method (appended after `session_removed_rx`, before
`session_snapshot`):

```rust
fn session_cleared_rx(&self) -> broadcast::Receiver<SessionId> {
    let (tx, rx) = broadcast::channel(1);
    drop(tx);
    rx
}
```

## 2. Transport impls

- `src/transport/embedded.rs` — `impl FspecBackend for EmbeddedFspecBackend`
  (line 71). Add `session_cleared_rx` mirroring `session_removed_rx`
  (line 1076): `match self.service.session_manager() { Some(handle) =>
  handle.session_cleared_rx(), None => closed }`.
  (The downstream `SessionManagerHandle::session_cleared_rx` already exists
  since RPC-436 — `rust/core/src/session_manager_handle.rs:755` default,
  override at `:2243`.)
- `src/transport/websocket.rs` — `impl FspecBackend for
  WebSocketFspecBackend` (line 204). Add `session_cleared_rx` mirroring
  `session_removed_rx` (line 346): spawn a splitter over
  `FspecWsClient::session_events_rx()` (private helper
  `client_session_events_rx`, line 162) forwarding only
  `SessionEvent::Cleared(id)` — the variant added by RPC-436 in
  `codelet_rpc_types` (`rust/rpc-types/src/lib.rs:331`, appended after
  Removed so bincode indices stay stable).

## 3. Action enum — `src/components/mod.rs`

`Action::SessionRemoved(codelet_rpc_types::SessionId)` (line 148) is the
RPC-434 sibling. Append `SessionCleared(codelet_rpc_types::SessionId)`
directly after it (file measured 1458 vs the rpc094 itemized ceiling 1458 —
adding one variant + doc stanza will require re-itemizing the measured
ceiling in `tests/scrollback_scroll_rpc094.rs` per the documented
reconciliation convention).

## 4. App dispatch + handler

AstGrep pattern `pub(crate) fn $NAME($$$ARGS) { $$$BODY }` over
`src/app/` (rust) confirmed the handler inventory:

- `dispatch.rs:113` — `Action::SessionRemoved(session)` arm delegates to
  `self.handle_session_removed(session.clone())` (body in
  `src/app/dispatch_session_lifecycle.rs:106`, matched at
  `dispatch_session_lifecycle.rs:106`). The new
  `Action::SessionCleared` arm will sit beside it and delegate to the new
  `handle_session_cleared` in the same file (currently 143 lines; +~55
  lines still < 300 ceiling).
- `src/app/dispatch_slash_clear.rs:44` — `handle_slash_clear` (the LOCAL
  /clear path, matched). Its local reset step
  `self.navigator.agent.reset_scrollback(&mut self.agent_view_store)`
  (line 47) is the DRY target.

## 5. View + store reset surface (DRY extraction)

- `src/views/agent.rs:242` — `AgentView::reset_scrollback(&mut self,
  store: &mut AgentViewStore)` (file at 299/300 lines — at the ceiling;
  the RPC-437 rename keeps the method 1:1 in size: rename to
  `reset_local_scrollback`, same body: current-session `ctx.reset_scrollback()`
  + `input.reset()` + slash/file popup clears).
- `src/store/agent_view/session_context.rs:115` —
  `SessionContext::reset_scrollback()` (the store-side reset reused by the
  per-session fold; also clears in-flight slots + `input_transition`).
- `src/store/agent_view/session_context.rs:40` — `input_draft: String`
  field (per-session draft the remote fold must blank).
- `src/store/agent_view/isolation_state.rs:69` — `set_session_status`
  (RPC-100 chrome mirror).
- `src/store/agent_view/work_unit_state.rs:58` — `reset_token_state`
  (RPC-100 chrome mirror).
- `src/store/agent_view/chrome_state.rs:183` — `clear_compaction_reduction`
  (RPC-100 chrome mirror).
- `src/store/agent_view.rs:204` — `current_session()`; `:148`
  `session_context_mut_for` (focus gate + per-session fold accessors).

The chunk-side Cleared reset this mirrors: `src/app/dispatch_stream_chunks.rs:79`
(`SessionState::Cleared` arm: `reset_token_state` +
`clear_compaction_reduction`).

## 6. Bootstrap subscriber (9th task)

- `src/app/bootstrap.rs:138` — `spawn_subscriber_tasks` (matched at
  `bootstrap.rs:138`), currently 8 tasks; (h)
  `spawn_session_removed_subscriber` (line 288) calls into
  `src/app/bootstrap_session_removed.rs:30` (matched). The new 9th task
  lands in a new sibling `src/app/bootstrap_session_cleared.rs`
  (`spawn_session_cleared_subscriber`, same Lagged-debug/Closed-break loop
  shape) registered in `src/app/mod.rs` (module list at line 19ff) and
  called from `spawn_subscriber_tasks`.
- `src/app/state/state_accessors.rs:128` — `subscriber_task_count()`
  (test accessor; count 8 → 9).
- `src/app/dispatch_reconnect.rs:135` — `handle_reconnected` respawns ALL
  subscriber tasks via `spawn_subscriber_tasks()` — no reconnect wiring
  needed for the 9th task (R4).

## 7. Test double — `tests/common/mod.rs` (MockBackend)

- `session_removed_tx` field (line 260) + `new()` channel (line 785) +
  `push_session_removed` (line 1156) + `session_removed_rx` override
  (line 2911) + `disconnect_all` (line 1232) + `reconnect_all`
  (line 1254) — all six touch-points gain a `session_cleared_*` twin.

## 8. Count pins to bump (8 → 9)

- `tests/app_bootstrap_rpc009.rs:98` + `:263` (`subscriber_task_count() == 8`)
- `tests/bug181_live_checkpoint_counts.rs:153` (`== 8`)
- `tests/tui109_checkpoints_progress.rs:379` (`== 8`)
- `tests/rpc434_session_events_tui_consumption.rs:310` (`== 8`)
- `tests/spawned_subordinate_session_registration_parity_rpc385.rs:208` (`== 8`)
- `tests/auto_reconnect_slice2_rpc011.rs:209` (`full_stream_count == 8`)
- `tests/reconnect_resubscribe_rpc415.rs:42` (`SUBSCRIBER_STREAM_COUNT: usize = 8`)

## 9. Source-shape ceilings at risk

- `src/components/mod.rs` = 1458 lines — measured ceiling 1458 pinned in
  `tests/scrollback_scroll_rpc094.rs` (rpc094 itemized budget). Adding the
  `SessionCleared` variant + doc stanza requires re-itemizing the measured
  ceiling (documented reconciliation convention).
- `src/views/agent.rs` = 299/300 lines — the `reset_scrollback` →
  `reset_local_scrollback` rename is size-neutral (name change only).
- `src/app/dispatch_session_lifecycle.rs` = 143 lines — +~55 for
  `handle_session_cleared` → ~198, under ceiling.
- `src/app/bootstrap_session_cleared.rs` — new file, ~50 lines (mirrors
  `bootstrap_session_removed.rs` at 48 lines).
- `src/transport/{mod,embedded,websocket}.rs` and
  `src/store/agent_view.rs` are exempt from the 300-LoC shape tests
  (only `views/agent/`, `views/agent.rs`, `components/mod.rs` (itemized
  budget) and the `source_shape_stores_rpc012.rs` target list are pinned).

## 10. Wire/daemon side already in place (RPC-436 — no change needed)

- `codelet_rpc_types::SessionEvent::Cleared(SessionId)`
  (`rust/rpc-types/src/lib.rs:331`)
- `BackgroundSession::clear_history` fires `session_cleared_tx`
  (`rust/sessions/src/background_session.rs:1746+`, single source of truth
  for clear; liveness-gated)
- `SessionManager::session_cleared_tx` / `session_cleared_rx`
  (`rust/sessions/src/session_manager.rs:190,315`)
- `SessionManagerHandle::session_cleared_rx` trait method + override
  (`rust/core/src/session_manager_handle.rs:755,2243`)
- `SharedFspecService::session_cleared_rx` (`rust/rpc/src/lib.rs:1091`)
- WS server `session_cleared_fanout` task
  (`rust/rpc-server/src/server.rs:247,487`)
