# RPC-433 Research — Daemon-side session lifecycle event fan-out

Investigated 2026-10-11 (user report: "if I create a session on one machine, it
doesn't send an event to all machines listening to the daemon"). This document
records the root-cause trace and the daemon-side design required to fan out
session lifecycle events (created / removed) to ALL connected WebSocket
clients, and to replay open sessions when a client connects.

## 1. Root cause: the event exists in-process but never crosses the wire

### 1.1 Producer already exists (daemon side)

`SessionManager` owns a per-manager broadcast, alongside its
chunks/logs/status siblings:

- `rust/sessions/src/session_manager.rs:169`
  `session_created_tx: broadcast::Sender<SessionInfo>`
  (capacity `SUPERVISOR_BROADCAST_CAPACITY` = 256,
  `rust/sessions/src/background_session.rs:89`).
- All three creation paths fire it after inserting into the in-memory map:
  - `create_session_with_id` — line 806
  - `create_session_from_manifest` (resume) — line 972
  - `create_isolated_session_with_id` — line 1179
- `SharedFspecService::session_created_rx()`
  (`rust/rpc/src/lib.rs:1055`) delegates to the attached handle:
  `SessionManagerHandle::session_created_rx()`
  (`rust/core/src/session_manager_handle.rs:724`, default returns a closed
  receiver; the real impl at `rust/sessions/src/handle_impl.rs:315`
  subscribes to the manager's sender).

So the daemon DOES emit a session-created event for every creation
(including AgentManager-spawned subordinates and /resume).

### 1.2 The WS server never fans it out

`codelet-rpc-server::server::handle_connection`
(`rust/rpc-server/src/server.rs:125`) spawns exactly FOUR per-connection
fan-out tasks, each draining one `SharedFspecService` broadcast and wrapping
it in an `Envelope` frame:

| source broadcast                | Envelope variant     | fan-out fn                  |
|---------------------------------|----------------------|-----------------------------|
| `watcher_rx()` (work units)     | `WorkUnitsUpdate`    | `work_units_fanout`         |
| `chunks_rx()`                   | `Event`              | `stream_chunks_fanout`      |
| `logs_rx()`                      | `LogEvent`           | `log_events_fanout`         |
| `status_changes_rx()`           | `StatusUpdate`       | `status_changes_fanout`     |

There is NO fan-out task for `session_created_rx()`, and
`Envelope` (`rust/rpc-server/src/envelope.rs`) has no session-lifecycle
variant at all — only `Rpc, Event, LogEvent, WorkUnitsUpdate, StatusUpdate,
CmdReq*, CmdRes*` (the two `Cmd*` are reserved-and-rejected).

### 1.3 The remote client is stubbed out

`WebSocketFspecBackend::session_created_rx()`
(`rust/fspec-tui/src/transport/websocket.rs:290`) returns a never-firing
(closed) receiver with a comment: "Full remote (tarpc/websocket) parity is an
explicit OUT-OF-SCOPE RPC follow-up." `FspecWsClient`
(`rust/rpc-server/src/client.rs`) likewise has no session-event template
receiver. `ClientInbound` (`rust/rpc-server/src/pump.rs:126`) has no
`on_session_event` hook.

### 1.4 The only working path is embedded (in-process)

`EmbeddedFspecBackend::session_created_rx()`
(`rust/fspec-tui/src/transport/embedded.rs:1060`) reads the handle's
broadcast directly (zero-cost, same process). A TUI running INSIDE the daemon
process sees new sessions; any machine connected over WebSocket does not.
This is exactly the reported symptom.

### 1.5 Removal has no event at all (either transport)

`SessionManager::destroy_session` (`rust/sessions/src/session_manager.rs:1247`)
performs `shift_remove` on the in-memory map (:1262), clears the active
pointer when it pointed at the destroyed session (BUG-206, :1272-1280), and
runs all cleanup — but fires NO broadcast. There is no
`session_removed_tx` anywhere in the codebase. (The only session-removal path
from the map is `destroy_session`; chain-of-command cleanup only removes
links, not sessions.)

### 1.6 Late joiners get no session snapshot

On connect the server pushes an initial `WorkUnitsUpdate` snapshot
(`server.rs:147-151`) but nothing session-equivalent. A client connecting
after sessions were created only learns about the daemon's active session via
the BUG-206 auto-restore RPC round-trips (`get_active_session` +
`list_live_sessions`), which resolve a single candidate — not the other live
tabs.

## 2. Design (daemon side)

### 2.1 Wire types — `codelet-rpc-types`

New payload type (serde, mirroring `SessionInfo`/`LogRecord` conventions):

```rust
/// A session lifecycle event pushed to ALL connected clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionEvent {
    /// A session was created (any client, spawned subordinate, or /resume).
    Created(SessionInfo),
    /// A session was destroyed / removed from the daemon's live map.
    Removed(SessionId),
}
```

- `Removed` carries only the id: the client already holds all tab state and
  needs nothing else to close the tab.
- No "replay" flag on the wire: the connect-time replay is a separate
  SNAPSHOT frame (below), keeping live events pure. This is the design
  decision from the investigation: replayed adds must NOT run the
  focus/active-pointer side effects of a live create, so they must be
  distinguishable on the client — a separate snapshot frame is cleaner than a
  `replay: bool` threaded through the event log.

Two new `Envelope` variants in `rust/rpc-server/src/envelope.rs` (APPENDED
after `CmdRes` so existing bincode variant indices stay stable — same
forward-compat rule the reserved `Cmd*` variants were added under):

```rust
/// Server-pushed session lifecycle event (live fan-out).
SessionEvent(SessionEvent),
/// Server-pushed open-session snapshot, sent once per connection right after
/// the initial WorkUnitsUpdate snapshot (connect-time "add replay").
SessionSnapshot(Vec<SessionInfo>),
```

+ `variant_name()` entries `"SessionEvent"` / `"SessionSnapshot"`.
Both are server→client only: client-pushed frames of either variant hit
`InboundHandler::on_reserved` on the server (exactly like `StatusUpdate`),
increment `rejected_envelopes`, and land in `rejected_variants`.

### 2.2 Manager — `codelet-sessions`

- New field `session_removed_tx: broadcast::Sender<SessionId>`
  (capacity 256, next to `session_created_tx` at `session_manager.rs:199`).
- `destroy_session`: fire
  `session_removed_tx.send(SessionId::from(uuid.to_string()))` ONLY when the
  `shift_remove` actually returned a session (`if let Some(session) = ...`
  block, ~:1288). Idempotent destroy of an unknown id emits nothing.
- New `pub fn live_session_infos(&self) -> Vec<SessionInfo>`: in-memory map
  only — `sessions.read().values().map(|s| s.get_info())` (the same
  collection `list_sessions` builds at :372, WITHOUT the persisted-manifest
  merge and without project filtering). This is the connect-time replay
  source. It MUST NOT read disk (parity with `all_in_memory_session_ids`,
  the BUG-206 liveness gate).
- Accessor `pub fn session_removed_tx(&self) -> &broadcast::Sender<SessionId>`
  mirroring `session_created_tx()` (:267).

### 2.3 Handle trait — `codelet-core` (`session_manager_handle.rs`)

- `fn session_removed_rx(&self) -> broadcast::Receiver<SessionId>` — DEFAULT
  returns a closed receiver (mirrors the `session_created_rx` convention at
  :724, so `StubSessionManagerHandle` and all test doubles compile unchanged
  unless they opt in).
- `fn live_session_infos(&self) -> Vec<SessionInfo>` — DEFAULT returns
  `Vec::new()` (mirrors the `list_live_sessions` BUG-206 convention at :500).

`StubSessionManagerHandle` (test surface used by EVERY rpc-server
integration test) must be upgraded so fan-out tests can drive events:
- new fields `session_created_tx` / `session_removed_tx`
  (constructed with the standard capacities).
- `create_session` / `create_isolated_session`: build the `SessionInfo`
  (same field mapping as `list_sessions` at :1545) and fire
  `session_created_tx.send(info)` — the stub currently fires NO
  session-created event, so multi-client parity tests need this.
- `destroy_session`: fire `session_removed_tx` only when the session existed.
- override `session_removed_rx()` / `live_session_infos()`.

### 2.4 Shared service — `codelet-rpc` (`SharedFspecService`)

- `pub fn session_removed_rx(&self) -> broadcast::Receiver<SessionId>` —
  delegate to the handle when attached; otherwise a degenerate closed
  receiver (exact shape of `status_changes_rx` at :1034).
- `pub fn live_sessions_snapshot(&self) -> Vec<SessionInfo>` — delegate to
  `handle.live_session_infos()`; empty when no manager is attached.

No new tarpc RPC methods are needed: this is pure server→client push +
connect-time snapshot (the BUG-206 `list_live_sessions` RPC already exists
for the liveness gate; it is a different, id-only surface and stays as-is).

### 2.5 WS server — `codelet-rpc-server`

`rust/rpc-server/src/pump.rs`:
- `InboundHandler::on_session_event(&self, SessionEvent)` — default defers to
  `on_reserved("SessionEvent")` (server side: client-pushed events are
  rejected, same as `StatusUpdate`).
- `InboundHandler::on_session_snapshot(&self, Vec<SessionInfo>)` — default
  defers to `on_reserved("SessionSnapshot")`.
- `ClientInbound` gains:
  - `session_events_tx: broadcast::Sender<SessionEvent>` — ONE stream for
    both variants (preserves created/removed ordering per session;
    `on_session_event` forwards into it).
  - `session_snapshot_tx: watch::Sender<Option<Vec<SessionInfo>>>` — WATCH,
    not broadcast: this is the exact fix documented for the work-units
    initial-snapshot race (pump.rs:116-119 doc: "Broadcast was insufficient
    there because broadcast receivers only see messages sent *after* they
    call `subscribe()`, which raced against the server's initial-snapshot
    push during connection handshake"). The TUI subscribes to session
    streams only AFTER bootstrap's initial RPCs, so a plain broadcast
    snapshot would be lost.

`rust/rpc-server/src/client.rs` (`FspecWsClient` + `ws_client_connect`):
- new fields: `session_events_rx_template` (broadcast template, sender owned
  by the pump task — drop-on-disconnect propagation per RPC-010 rule [23])
  and `session_snapshot_watch_rx`.
- new accessors:
  - `session_events_rx() -> broadcast::Receiver<SessionEvent>`
    (plain `resubscribe()`, like `chunks_rx`).
  - `session_snapshot_rx() -> broadcast::Receiver<Vec<SessionInfo>>`
    (forwarder task replaying the cached snapshot then forwarding updates —
    copy the `work_units_rx` forwarder shape at client.rs:105).
- `ws_client_connect` wires both senders into `ClientInbound`.

`rust/rpc-server/src/server.rs` (`handle_connection`):
- TWO new per-connection fan-out tasks mirroring `status_changes_fanout`:
  - drain `service.session_created_rx()` →
    `Envelope::SessionEvent(SessionEvent::Created(info))`
  - drain `service.session_removed_rx()` →
    `Envelope::SessionEvent(SessionEvent::Removed(id))`
  both aborted + awaited at connection teardown with the others.
- Connect-time replay, right after the initial `WorkUnitsUpdate`
  (:147-151): `let live = service.live_sessions_snapshot();` then
  `envelope_out_tx.send(Envelope::SessionSnapshot(live))` — always sent
  (even empty) so the client's watch is stamped deterministically, matching
  the work-units "always send initial snapshot" behaviour.

`rust/rpc-server/src/lib.rs` (`ServerStats`):
- new `lag_session_events: Arc<AtomicU64>` (shared by both session fan-out
  tasks; `RecvError::Lagged` accounting parity with the other fan-outs).
  NOT added to `ServerStatsHandle`/`HealthInfo` — same as `lag_status`,
  which is stats-only.

## 3. File-by-file change list (daemon side)

| crate | file | change |
|-------|------|--------|
| codelet-rpc-types | `src/lib.rs` | new `SessionEvent` enum |
| codelet-sessions | `src/session_manager.rs` | `session_removed_tx` field + ctor + accessor; fire on `destroy_session`; new `live_session_infos()` |
| codelet-sessions | `src/handle_impl.rs` | impl `session_removed_rx` + `live_session_infos` on the real handle |
| codelet-core | `src/session_manager_handle.rs` | trait: `session_removed_rx` (default closed) + `live_session_infos` (default empty); stub: fields + fire on create/destroy + override both |
| codelet-rpc | `src/lib.rs` | `SharedFspecService::session_removed_rx` + `live_sessions_snapshot` |
| codelet-rpc-server | `src/envelope.rs` | `Envelope::SessionEvent` + `Envelope::SessionSnapshot` + `variant_name` |
| codelet-rpc-server | `src/pump.rs` | `on_session_event` / `on_session_snapshot` hooks; `ClientInbound` fields |
| codelet-rpc-server | `src/client.rs` | `FspecWsClient` fields + `session_events_rx` / `session_snapshot_rx`; `ws_client_connect` wiring |
| codelet-rpc-server | `src/server.rs` | two fan-out tasks; connect-time `SessionSnapshot` push |
| codelet-rpc-server | `src/lib.rs` | `ServerStats.lag_session_events` |

## 4. Test plan (this card)

Integration tests in `rust/rpc-server/tests/` (shape:
`ws_multi_client_chunks.rs` — temp workspace + `StubSessionManagerHandle`
with `StubProvider` + two `ws_client_connect` clients):

1. **Created fan-out**: clients A+B connected; A calls `create_session`;
   both A and B observe `SessionEvent::Created` for that id on
   `session_events_rx()`.
2. **Removed fan-out**: A destroys the session; both observe
   `SessionEvent::Removed(id)`.
3. **Connect-time replay**: A creates a session, THEN B connects; B's
   `session_snapshot_rx()` yields a snapshot containing that session
   (late-subscriber guarantee — the watch replay, no race).
4. **Reserved rejection**: a client sends raw `Envelope::SessionEvent` /
   `Envelope::SessionSnapshot` binary frames; server counts them in
   `rejected_envelopes` and names both variants in `rejected_variants`
   (shape: `ws_reserved_variants_after_rpc007.rs`).
5. **No-manager degenerate**: `SharedFspecService::new(watcher)` (no
   manager); client connects; receives an EMPTY `SessionSnapshot`; no
   crash, no events.

Unit/integration in `codelet-sessions` / `codelet-core`:
- `destroy_session` fires `session_removed_tx` exactly once when the
  session existed; a second (idempotent) destroy fires nothing.
- `live_session_infos` returns in-memory sessions only (no persisted
  manifests).
- stub: `create_session`/`create_isolated_session` fire
  `session_created_tx`; `destroy_session` fires `session_removed_tx`;
  `live_session_infos` maps records.

## 5. Risks / notes

- bincode enum variant ordering: new variants MUST be appended after
  `CmdRes` (indices 5/6 are reserved). Appending preserves all existing
  indices → no breaking wire change.
- `destroy_session` cleanup block runs hooks (footer poller, MCP, loops);
  the removed-broadcast must fire AFTER `shift_remove` and BEFORE those
  hooks complete? No ordering requirement — it only has to be after the
  map removal. Firing inside the `if let Some(session)` block is correct
  and idempotent-destroy-safe.
- The connect-time snapshot duplicates data the BUG-206 auto-restore path
  fetches via RPC (`list_live_sessions` + `get_active_session`). That path
  stays (it resolves the FOCUS candidate with liveness gating); the
  snapshot feeds the TAB list. No conflict; the two are independent.
- Session destroyed during a client's WS disconnect: the client misses the
  `Removed` event; on reconnect the snapshot re-adds open sessions but does
  NOT prune stale tabs (add-only replay per the stated requirement).
  Recorded as a known limitation / follow-up candidate — NOT in scope here.
