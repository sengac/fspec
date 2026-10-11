# AST Research — RPC-433: session event fan-out touch points

Captured 2026-10-11 via AstGrep / GraphSearch (ast_search content mode).

## 1. `Envelope` enum (rust/rpc-server/src/envelope.rs:35)

```
pub enum Envelope {
    Rpc(Vec<u8>),
    Event { session_id, chunk },
    LogEvent(LogRecord),
    WorkUnitsUpdate(Vec<WorkUnitInfo>),
    StatusUpdate { session_id, status },
    CmdReq,   // reserved index 5
    CmdRes,   // reserved index 6
}
```
New variants `SessionEvent(SessionEvent)` and `SessionSnapshot(Vec<SessionInfo>)`
MUST be appended after `CmdRes` (bincode variant indices 0-6 stay stable).

## 2. Fan-out functions in rust/rpc-server/src/server.rs

- work_units_fanout (:240)
- stream_chunks_fanout (:271)
- log_events_fanout (:298)
- status_changes_fanout (:325)

All four share the identical shape: `loop { match rx.recv().await { Ok(payload) =>
envelope_out_tx.send(Envelope::X(payload)) ... Lagged => lag_counter.fetch_add,
Closed => break } }`. The two new session fan-out tasks copy `status_changes_fanout`.
`handle_connection` (:125) is the spawn site (currently 4 fan-out tasks + initial
WorkUnitsUpdate snapshot at :147-151).

## 3. SessionManager struct (rust/sessions/src/session_manager.rs:150-181)

Fields: `sessions: RwLock<IndexMap<Uuid, Arc<BackgroundSession>>>`,
`chain_of_command`, `active_session_id`, `scheduler_handle`, `default_model`,
`chunks_tx`, `logs_tx`, `status_changes_tx`,
`session_created_tx: broadcast::Sender<SessionInfo>` (RPC-385), `hooks`, `self_weak`.

Constructor `new()` (:191-219) creates all broadcast channels with
`SUPERVISOR_BROADCAST_CAPACITY` (256, background_session.rs:89) —
`session_removed_tx` joins here with the same capacity.

Accessors: `session_created_tx()` (:267). `list_sessions` (:371) shows the
in-memory collection idiom (`sessions.read().values().map(|s| s.get_info())`)
that `live_session_infos()` reuses WITHOUT the persisted-manifest merge.
`destroy_session` (:1247) — `shift_remove` at :1262, the `if let Some(session)`
cleanup block at :1288 is where `session_removed_tx.send` fires.

## 4. Handle trait (rust/core/src/session_manager_handle.rs)

- `fn session_created_rx(&self) -> broadcast::Receiver<SessionInfo>` default
  returns a closed channel (:724)
- `fn list_live_sessions(&self) -> Vec<SessionId>` default empty (:500, BUG-206)
- `SessionManagerHandle for StubSessionManagerHandle` (:1544+):
  `create_session` (:1568) builds SessionRecord + pushes to `sessions` vec
  (NO broadcast fired today — must fire session_created_tx);
  `create_isolated_session` (:2105) same;
  `destroy_session` (:2133) `sessions.retain(|r| r.id != *session_id)`
  (must fire session_removed_tx when a record was removed);
  `list_sessions` (:1545) contains the SessionInfo field-mapping for the stub
  (reused by the stub's `live_session_infos` override).

## 5. Real handle impl (rust/sessions/src/handle_impl.rs)

- `session_created_rx` (:315): `SessionManager::session_created_tx(self).subscribe()`
- `list_live_sessions` (:793)
- `resume_session` (:173): checks `already_exists` and only creates
  via `create_session_from_manifest` when NOT in memory — so the created
  broadcast fires at most once per resume (no double-add).

## 6. SharedFspecService (rust/rpc/src/lib.rs)

- `session_created_rx` (:1055): `Some(handle) => handle.session_created_rx()`
  else closed channel — the exact template for `session_removed_rx`.
- `status_changes_rx` (:1034): same delegate shape.
- `chunks_tx` (:946) / `logs_tx` (:970): cloneable-sender delegation
  pattern (not needed here — the manager owns the removed sender).

## 7. Client pump (rust/rpc-server/src/pump.rs + client.rs)

- `InboundHandler` trait: `on_event` / `on_log_event` / `on_status_update`
  each default to `on_reserved`; `ClientInbound` overrides with broadcast
  sends. New: `on_session_event` / `on_session_snapshot` (same shape).
- `ClientInbound` struct (:126): `work_units_tx` (watch), `chunks_tx`,
  `logs_tx`, `status_tx` (broadcasts). New fields: `session_events_tx`
  (broadcast `SessionEvent`), `session_snapshot_tx` (watch
  `Option<Vec<SessionInfo>>`).
- `ws_client_connect` (client.rs:168): builds watch + 3 broadcasts, moves
  senders into `ClientInbound` inside the pump task; `FspecWsClient` keeps
  template receivers. New: `session_events_rx_template` +
  `session_snapshot_watch_rx`; accessors `session_events_rx()`
  (resubscribe, like chunks_rx :143) and `session_snapshot_rx()`
  (forwarder task, like work_units_rx :105 — replays the cached frame).

## 8. TUI-side MockBackend (rust/fspec-tui/tests/common/mod.rs)

Already models `session_created_tx: Mutex<Option<broadcast::Sender<SessionInfo>>>`
(RPC-385/RPC-415), with `push_session_created` (:1041), `session_created_rx`
(:2591), and disconnect_all/reconnect_all swap patterns. RPC-434 adds the
`session_removed_tx` twin + `push_session_removed` + `session_snapshot`
scripted field to this mock so the App-level tests can drive both events
without a live daemon.
