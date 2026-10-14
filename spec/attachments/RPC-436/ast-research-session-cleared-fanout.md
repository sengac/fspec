# RPC-436 — AST research: session-cleared event fan-out

Date: 2026-10-11
Work unit: RPC-436 (Session cleared event fan-out, SessionEvent::Cleared)

## Question

Where exactly must `SessionEvent::Cleared` be added and who must fire the
`session_cleared_tx` broadcast so that `/clear` (history clear) reaches every
connected client over the RPC-433 session-events wire stream — mirroring
Created/Removed?

## Code paths discovered (via grep/AST inspection)

### 1. Wire payload — `codelet-rpc-types`
- `rust/rpc-types/src/lib.rs:320-326` — `pub enum SessionEvent { Created(SessionInfo), Removed(SessionId) }`.
  `Cleared(SessionId)` must be APPENDED (bincode index 2) so existing
  discriminants (Created=0, Removed=1) stay stable (RPC-433 forward-compat
  rule). `Envelope::SessionEvent(SessionEvent)` (rpc-server/src/envelope.rs:84)
  and the pump (pump.rs `on_session_event`) need NO changes — they carry the
  whole payload opaquely.

### 2. Manager broadcast — `codelet-sessions::SessionManager`
- `rust/sessions/src/session_manager.rs:169,178` — `session_created_tx`,
  `session_removed_tx` (both `broadcast::Sender<...>` capacity
  `SUPERVISOR_BROADCAST_CAPACITY` = 256, `background_session.rs:89`).
  New `session_cleared_tx: broadcast::Sender<SessionId>` sits next to them
  (struct field + `new()` channel + `session_cleared_tx()` accessor).
- Fire site decision: the event must fire from
  `BackgroundSession::clear_history` (`rust/sessions/src/background_session.rs:1732-1756`,
  the documented "single source of truth for clear" — TUI-065 DRY note) so
  ALL entry points emit exactly one event:
  - RPC: `handle_impl.rs::clear_history` → `block_in_place(|| session.clear_history())`
  - NAPI: `rust/napi/src/session_bindings.rs:1551` `session_clear_history`
  - Bridge: `rust/napi/src/agent_loop.rs:922` / `rust/agent-loop/src/agent_loop.rs:952`
    (control handler "clear")
  Fire it there via `self.owning_manager()` (background_session.rs:619,
  RPC-386 weak back-ref) → `SessionManager::instance()` fallback (the
  singleton never stamps self_weak — documented at session_manager.rs:381-384
  and RPC-386). No re-entrancy risk: `clear_history` never runs under the
  sessions-map write lock (destroy_session holds it, clear doesn't).
- New `SessionManager::clear_session_history(&str) -> Result<(), String>`:
  resolves the session from the map (unknown id → `Err("Session not found")`
  + NO event, mirroring `destroy_session`'s R2 idempotency,
  session_manager.rs:1327-1341), then `block_in_place(|| session.clear_history())`
  so the event fires inside the real clear.
  Note: handle_impl's `clear_history` currently calls the session directly;
  it is switched to `SessionManager::clear_session_history` so the liveness
  gate is shared by both the trait method and the manager method.

### 3. Handle surface — `codelet-core`
- `rust/core/src/session_manager_handle.rs:724,739` — trait defaults
  `session_created_rx` / `session_removed_rx` (closed receiver). New
  `session_cleared_rx()` default mirrors them.
- Production override: `rust/sessions/src/handle_impl.rs:315-324` —
  `session_created_tx(self).subscribe()` pattern → `session_cleared_rx`
  override.
- `StubSessionManagerHandle` (session_manager_handle.rs:1166-1190, 2195-2203,
  1828-1842): gains `session_cleared_tx` field + `session_cleared_rx`
  override + `clear_history` fires it. The stub's `clear_history` has no
  liveness gate (its `destroy_session` only gates on the in-memory Vec,
  lines 2205-2220) — consistent with its unconditional Created stub; the
  REAL liveness gate is asserted on the real manager in
  sessions/tests/rpc436_session_cleared_broadcast.rs.

### 4. Service delegation — `codelet-rpc`
- `rust/rpc/src/lib.rs:1055-1096` — `session_created_rx` /
  `session_removed_rx` / `live_sessions_snapshot` delegate to the handle.
  New `session_cleared_rx()` mirrors `session_removed_rx` (degenerate closed
  receiver when no manager).

### 5. WS server fan-out — `codelet-rpc-server`
- `rust/rpc-server/src/server.rs:211-236` — per-connection
  `session_created_fanout` / `session_removed_fanout` tasks draining
  `service.session_created_rx()` / `service.session_removed_rx()`, sharing
  `lag_session_events`. New `session_cleared_fanout` (server.rs:430-459
  pattern) wraps ids in `Envelope::SessionEvent(SessionEvent::Cleared(id))`,
  spawned in `handle_connection` next to the others, aborted+awaited in the
  cleanup block (server.rs:266-277).

### 6. Regression-test constraints checked
- `rust/fspec/tests/rpc073_clear_history_no_panic.rs:319-341` source-shape
  test: `handle_impl.rs::clear_history` body MUST keep the literal
  `tokio::task::block_in_place(` — preserved (the manager method also
  block_in_places; the trait method body retains the wrapper by delegating
  into `SessionManager::clear_session_history` which contains it — verified
  by re-running that test after the change).
- `rust/sessions/tests/session_manager_shape.rs:1180-1231`: asserts
  `chunks_tx.clone()` / `status_changes_tx.clone()` at the two
  `BackgroundSession::new(` sites — untouched (we add the event via the
  owning-manager resolution, NOT a new constructor param, so this test stays
  green without changes).
- `rust/fspec-tui/tests/rpc037_cross_transport_parity.rs:158-187` +
  `rust/fspec/tests/cross_frontend_parity.rs`: assert the
  `SessionStateChange{state:Cleared}` chunk — chunk emission inside
  `clear_history` is untouched (event is ADDED, not a replacement).

## TUI consumption (out of scope, follow-up WU)

- `fspec-tui/src/transport/mod.rs:980-1015` — `FspecBackend::
  session_created_rx/session_removed_rx` defaults; `transport/websocket.rs:
  322-363` splitter over `session_events_rx`; `transport/embedded.rs:1060-1085`
  handle delegation; `app/bootstrap.rs:247-311` +
  `app/bootstrap_session_removed.rs` subscriber + `Action::SessionRemoved`
  handler (`app/dispatch_session_lifecycle.rs:106-142`).
  The follow-up WU adds a `session_cleared_rx` sibling + `Action::
  SessionCleared` + idempotent App fold (reset focused-session scrollback,
  mirror `handle_slash_clear`'s local step for the REMOTE session).

## Test files written (RED phase)

- `rust/rpc-server/tests/rpc436_session_cleared_fan_out.rs` — wire-side
  scenario via StubSessionManagerHandle + two WS clients (mirrors
  rpc-server/tests/rpc433_session_event_fan_out.rs). RED: `SessionEvent::
  Cleared` does not exist (compile error).
- `rust/sessions/tests/rpc436_session_cleared_broadcast.rs` — manager-level
  liveness gate + exactly-once (mirrors
  sessions/tests/rpc433_session_removed_broadcast.rs). RED:
  `session_cleared_tx` / `clear_session_history` do not exist (compile error).
