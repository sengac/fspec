# AST Research — RPC-434: TUI-side session event consumption touch points

Captured 2026-10-11 via AstGrep / GraphSearch (ast_search).

## 1. Bootstrap subscriber spawn (rust/fspec-tui/src/app/bootstrap.rs)

- `pub(crate) fn spawn_subscriber_tasks(&mut self)` at :152 — spawns 5
  (soon 7) subscriber tasks: (a) work_units_rx→WorkUnitsLoaded
  (Lagged→refetch), (b) chunks_rx→ChunkReceived, (c) logs_rx→tracing,
  (d) status_changes_rx→SessionStatusChanged,
  (e) session_created_rx→SessionCreated (`info.id` folded to
  `Action::SessionCreated(SessionId::new(info.id))`).
  The new (f) session_removed_rx→SessionRemoved task copies loop (e):
  `Ok(id) => tx.send(Action::SessionRemoved(id))`,
  `Lagged(n) => debug!(...)`, `Closed => break`.
- `App::bootstrap` (:27) sequence ends with
  `self.spawn_session_restore(None);` — the snapshot replay MUST run
  synchronously BEFORE that line (R4 ordering guarantee).

## 2. Action::SessionCreated handler (dispatch_create_session_dialog.rs:122)

`App::handle_session_created(session: SessionId)` — complexity 4:
1. idempotent guard: `agent_view_store.session_context_for(&session).is_some()`
   → early return (authoritative dedup in `AgentViewStore::append_session`)
2. `append_session(SessionContext::new(session))`
3. `mux_sync_window()` + `navigator.mux.note_session_created()`
4. `sync_mux_focus_to_session()` (BUG-163, no-op outside mux)
5. `active_session_tx.send(Some(session))`
6. (`spawn_set_active_session(session)` — added by BUG-206 commit, present
   in current source at :161 but missing from this stale graph capture)
7. work-unit attach when a current WU context exists
8. `refresh_session_chrome` + `spawn_hydrate_pending_input` +
   `spawn_load_supervisors` + pre-session debug propagation.

Snapshot replay folds into THIS handler (R4) — the guard makes duplicate
replay a no-op; side effects (focus/active pointer) are intentional
(assumption A1).

## 3. Tab removal (store/agent_view.rs)

- `session_context_for(&self, id)` :144
- `append_session(&mut self, ctx)` :165
- `remove_session_if_open(&mut self, id) -> bool` :188 — the removal
  primitive `handle_session_removed` uses first (its return / not-open
  check is the idempotency gate, R3).

## 4. Local close cleanup (app/dispatch_agent_exit.rs:90-136)

Steps mirrored by `handle_session_removed`:
- `board_store.detach_session(wu_id)` (:92)
- `agent_view_store.remove_session_if_open(&session)` (:105)
- `mux_sync_window()` (:109)
- `agent_view_store.set_current_work_unit(None, None)` (:120)
- (backend destroy is LOCAL-only — NOT mirrored; the daemon already
  destroyed the session on the remote path)
- focus/BackToBoard via `Action::BackToBoard` (:135) — remote removal
  needs a SOFTER focus fallback (surviving session, or board only when
  zero remain), so `handle_session_removed` computes the fallback instead
  of blindly re-dispatching BackToBoard.

## 5. Reconnect respawn (app/dispatch_reconnect.rs:135)

`handle_reconnected`: `show_reconnected_success()` → abort all
`subscriber_tasks` → `spawn_subscriber_tasks()` → re-fetch
list_work_units → `spawn_session_restore(focused)`. The snapshot replay
hook goes BEFORE `spawn_session_restore(focused)` in this function,
mirroring the bootstrap ordering (R4/R5: no new subscriber wiring needed —
the abort+respawn covers task (f) automatically).

## 6. FspecBackend trait + transports (transport/mod.rs, embedded.rs, websocket.rs)

- trait `session_created_rx()` :994 — default closed channel (two-line
  `broadcast::channel(1); drop(tx)`). New siblings `session_removed_rx()`
  (same default) + `async fn session_snapshot()` (default `Vec::new()`).
- `EmbeddedFspecBackend::session_created_rx` :1060 —
  `self.service.session_manager()` match → `handle.session_created_rx()`
  / closed channel. New methods copy the match for
  `session_removed_rx`; `session_snapshot` →
  `self.service.live_sessions_snapshot()`.
- `WebSocketFspecBackend::session_created_rx` :290 — `empty_broadcast_rx()`
  (THE STUB being replaced). Client slot is
  `Arc<RwLock<Option<FspecWsClient>>>`; all `*_rx` methods use the
  `try_read()` → Some(client) / None→empty pattern (chunks :250, logs :259,
  status :275). New: per-subscriber splitter task over
  `client.session_events_rx()` (RPC-433) feeding two backend-level
  broadcasts; `session_snapshot()` = `self.client.read().await` guard →
  fresh client's snapshot receiver (bounded timeout → empty on
  disconnect).
- `tests/common/mod.rs` MockBackend — `session_created_tx`
  `Mutex<Option<Sender>>` (:213+) with `push_session_created` (:1041),
  `session_created_rx` (:2591), disconnect_all/reconnect_all (:1083/:1103).
  RPC-434 adds the `session_removed_tx` twin, `push_session_removed`,
  `session_removed_rx`, and a scripted `session_snapshot` vec +
  `set_session_snapshot` helper.
