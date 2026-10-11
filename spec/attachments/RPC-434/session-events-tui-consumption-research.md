# RPC-434 Research — TUI-side session event consumption (embedded + WS)

Companion to RPC-433 (daemon-side fan-out). This card wires the TUI to
CONSUME the new push streams on both transports: `SessionEvent`
(Created/Removed) live events + the connect-time `SessionSnapshot` replay,
replacing the WS transport's never-firing `session_created_rx` stub and
adding session-removal handling for sessions destroyed from ANOTHER client.

Investigated 2026-10-11.

## 1. Current state

### 1.1 The 5 (→6) bootstrap subscriber tasks

`App::spawn_subscriber_tasks` (`rust/fspec-tui/src/app/bootstrap.rs:152`)
spawns:
- (a) `work_units_rx` → `Action::WorkUnitsLoaded` (Lagged → refetch)
- (b) `chunks_rx` → `Action::ChunkReceived` (unfiltered, all sessions)
- (c) `logs_rx` → tracing
- (d) `status_changes_rx` → `Action::SessionStatusChanged`
- (e) `session_created_rx` → `Action::SessionCreated(id)` (RPC-385)

These are aborted + respawned on `Action::Reconnected`
(`app/dispatch_reconnect.rs:135 handle_reconnected`, which also re-runs
the RPC-011 one-shot re-bootstrap + BUG-206 auto-restore).

### 1.2 Action handlers (already exist — this card mostly reuses them)

- `Action::SessionCreated(SessionId)` →
  `App::handle_session_created` (`app/dispatch_create_session_dialog.rs:122`):
  - **Idempotency guard** (:136): early-return when the tab already exists
    (dedup lives authoritatively in
    `AgentViewStore::append_session`, `store/agent_view.rs:165`).
  - side effects: `append_session`, `mux_sync_window`,
    `navigator.mux.note_session_created`,
    `active_session_tx.send(Some(session))`,
    `spawn_set_active_session` (daemon pointer), work-unit attach when a
    WU context is current, `refresh_session_chrome`,
    `spawn_hydrate_pending_input`, `spawn_load_supervisors`,
    pre-session debug propagation.
- Tab removal: `AgentViewStore::remove_session_if_open(id)`
  (`store/agent_view.rs:188`) — currently called only from
  LOCAL-initiated close paths:
  - `app/dispatch_agent_exit.rs:105` (Esc → close agent: remove tab,
    `mux_sync_window`, clear current work unit, THEN spawn
    `backend.destroy_session`)
  - `app/merge_worktree_success.rs:41`
  - `app/dispatch_resume_search_views.rs:168`

  There is NO `Action::SessionRemoved` — a session destroyed by ANOTHER
  client leaves a dead tab in every other connected TUI.

### 1.3 Backend trait surface

`FspecBackend` (`rust/fspec-tui/src/transport/mod.rs`):
- `session_created_rx()` (:994) — default returns a CLOSED receiver
  ("Full remote (tarpc/websocket) parity is an explicit OUT-OF-SCOPE RPC
  follow-up" — that follow-up is RPC-433 + this card).
- `session_removed_rx()` does NOT exist.

`EmbeddedFspecBackend`:
- `session_created_rx` (:1060) → `service.session_manager().session_created_rx()`
  (real, same-process).
- `session_removed_rx` — to be added, same shape.

`WebSocketFspecBackend`:
- `session_created_rx` (:290) → `empty_broadcast_rx()` — THE STUB.
  `session_removed_rx` — to be added; both feed from the new
  `FspecWsClient::session_events_rx()` stream (RPC-433).

## 2. Design

### 2.1 FspecBackend additions (`transport/mod.rs`)

```rust
/// Subscribe to session-created events (embedded: direct; WS: from the
/// SessionEvent live stream). (existing, now real on both transports)
fn session_created_rx(&self) -> broadcast::Receiver<SessionInfo>;

/// RPC-434: subscribe to session-removed events. Default: closed receiver
/// (existing convention — transports that don't wire it degrade
/// gracefully; the subscriber exits on RecvError::Closed).
fn session_removed_rx(&self) -> broadcast::Receiver<SessionId> {
    let (tx, rx) = broadcast::channel(1);
    drop(tx);
    rx
}

/// RPC-434: the connect-time open-session snapshot. Default: empty Vec.
/// Returned once per connection establishment (embedded: the
/// manager's in-memory map at subscribe time; WS: the server-pushed
/// SessionSnapshot frame).
fn session_snapshot(&self) -> Vec<SessionInfo> { Vec::new() }
```

### 2.2 Embedded transport (`transport/embedded.rs`)

- `session_removed_rx` → `service.session_manager().session_removed_rx()`
  (degenerate closed receiver when no manager — same match as
  `session_created_rx` at :1060).
- `session_snapshot` → `service.live_sessions_snapshot()`
  (RPC-433 `SharedFspecService` method; in-memory map only, empty when no
  manager).

### 2.3 WebSocket transport (`transport/websocket.rs`)

The client's live stream carries BOTH variants, but the backend surface
exposes TWO receivers (created / removed) plus the snapshot. A single
fan-out task splits the stream:

- New fields on `WebSocketFspecBackend`:
  - `session_created_tx: broadcast::Sender<SessionInfo>`
  - `session_removed_tx: broadcast::Sender<SessionId>`
  - `session_snapshot: Arc<tokio::sync::Mutex<Option<Vec<SessionInfo>>>>`
    (or a watch; a one-shot read is all `session_snapshot()` needs —
    but it must survive supervisor swaps? NO: on reconnect the supervisor
    swaps the whole `FspecWsClient`; the snapshot is re-read from the NEW
    client. So the snapshot state must live PER-CLIENT, not per-backend.
    Simplest correct shape: `FspecWsClient` (RPC-433) exposes
    `session_snapshot_rx()` as a watch-backed forwarder — but a
    synchronous `session_snapshot()` call can't await a watch.

    Resolved shape: `WebSocketFspecBackend` holds
    `session_snapshot_rx_template: broadcast::Receiver<Vec<SessionInfo>>`
    swapped atomically with the client slot? That doubles the swap logic.

    SIMPLER + correct: make `session_snapshot` ASYNC on the trait:
    `async fn session_snapshot(&self) -> Vec<SessionInfo>` —
    embedded: `service.live_sessions_snapshot()` (sync, wrapped);
    websocket: `self.client.read().await` → guard the client slot →
    `client.session_snapshot_rx().recv().await` (the RPC-433 watch
    forwarder replays the cached frame; bounded by a short timeout →
    `Vec::new()` on failure/disconnect). The only caller is bootstrap
    (once, async context) so an async default is acceptable; the trait
    already has many async methods.

- `session_created_rx` / `session_removed_rx`:
  - if the client slot holds a `FspecWsClient`, call
    `client.session_events_rx()` and spawn a per-subscriber splitter task
    that forwards `Created` → `session_created_tx`, `Removed` →
    `session_removed_tx`, exiting on Closed/Lagged.
  - when the slot is `None` (disconnected): return a closed receiver
    (existing `empty_broadcast_rx()` pattern) — the bootstrap subscriber
    task exits on Closed and is respawned on `Action::Reconnected`
    (dispatch_reconnect.rs:135), which re-subscribes against the fresh
    client. This mirrors the existing chunks/status disconnect
    propagation (RPC-010 rule [23]).

### 2.4 App dispatch — new `Action::SessionRemoved(SessionId)`

`components/mod.rs`: add the variant next to `SessionCreated` (:141).

`App::handle_session_removed` (new; `app/dispatch_session_lifecycle.rs` or
fold into `dispatch_create_session_dialog.rs` under the 300-line ceiling —
prefer a new small file):
```
1. If the tab is not open: no-op (session we never saw, or already closed).
2. If it's the FOCUSED session: move focus to a surviving session
   (or BackToBoard semantics — mirror dispatch_agent_exit.rs: focus
   handling on the last tab close: MUX-002 re-clamp via
   mux_sync_window, navigator fallback to board when none remain).
3. agent_view_store.remove_session_if_open(id)
4. mux_sync_window()
5. board_store.detach_session(wu_id) if the tab had a work-unit binding
   (mirror dispatch_agent_exit.rs:91-92).
6. clear current work unit when it pointed at the removed session
   (mirror dispatch_agent_exit.rs:120).
```

The handler must be IDEMPOTENT (double-fire from the local destroy path +
the remote event): step 1's not-open check makes re-entry a no-op; the
local close path (`dispatch_agent_exit`) calls `remove_session_if_open`
synchronously BEFORE the daemon fires the event, so the remote event
arrives to find nothing open — correct.

`App::dispatch`: add the arm (`Action::SessionRemoved(id) =>
self.handle_session_removed(id);`).

### 2.5 Bootstrap — 6th subscriber + snapshot seeding

`spawn_subscriber_tasks`:
- (f) `session_removed_rx` → `Action::SessionRemoved` (same
  Lagged-debug / Closed-break loop shape as (e)).
- (g) SNAPSHOT: in `App::bootstrap` (NOT a subscriber loop) — BEFORE
  `spawn_session_restore(None)`:
  ```
  for info in self.backend.session_snapshot().await {
      self.dispatch(Action::SessionCreated(SessionId::new(info.id.clone())));
  }
  ```
  i.e. replayed adds ride the EXISTING `Action::SessionCreated` handler —
  zero new fold code. Consequence: replayed adds DO run the full
  `handle_session_created` side effects (focus + active pointer + chrome
  fetches). The design decision (see §2.6) is that this is ACCEPTABLE and
  actually desirable:

### 2.6 Focus semantics of the connect-time replay (decision)

Replay fires `SessionCreated` per open session in snapshot order
(IndexMap insertion order = creation order). `handle_session_created`
sends `active_session_tx.send(Some(id))` + `spawn_set_active_session(id)`
for each, so the LAST snapshot session becomes active — then
`spawn_session_restore` (BUG-206 auto-restore, spawned AFTER the replay
in `bootstrap`) re-resolves the FOCUS candidate (focused session on
reconnect / daemon active pointer, liveness-gated via
`list_live_sessions`) and re-asserts it.

Two competing options:
1. **Accept last-wins + restore re-assert** (chosen): simplest; no new
   wire payload; the BUG-206 restore already exists to pick the correct
   focus and runs after the replay. Transient double-focus is invisible
   (same render pass). Ordering dependency: restore must run after
   replay — guaranteed by `bootstrap` calling the replay synchronously
   before `spawn_session_restore(None)`.
2. Replay flag / separate no-focus action: cleaner isolation but a new
   Action variant + a parallel handler duplicating the append/mux
   wiring; rejected as over-engineering for the current surface.

RISK (documented): if the daemon active pointer is None (no restore
candidate) AND N>0 sessions were replayed, the last-created session is
focused. That matches "most recent" intuition and matches the embedded
transport's existing behaviour (embedded TUIs already fold every live
create through the same handler).

### 2.7 Reconnect path

No new work: `handle_reconnected` (dispatch_reconnect.rs:135) already
aborts + respawns ALL subscriber tasks (so the new session-removed task
and the re-subscribed session-created splitter come back automatically)
and re-runs `spawn_session_restore`. The snapshot should ALSO be
re-applied on reconnect — extend `handle_reconnected` to run the same
replay loop against the fresh client before `spawn_session_restore`.
(Duplicated 4-line loop extracted into a shared helper
`App::replay_session_snapshot()` used by bootstrap + reconnect.)

## 3. File-by-file change list (TUI side)

| crate | file | change |
|-------|------|--------|
| codelet-fspec-tui | `src/components/mod.rs` | `Action::SessionRemoved(SessionId)` variant |
| codelet-fspec-tui | `src/app/dispatch.rs` | dispatch arm for `SessionRemoved` |
| codelet-fspec-tui | `src/app/dispatch_session_lifecycle.rs` (new) | `handle_session_removed` (idempotent removal + focus/mux/WU cleanup) |
| codelet-fspec-tui | `src/app/bootstrap.rs` | 6th subscriber (session_removed); snapshot replay helper call; `replay_session_snapshot()` helper |
| codelet-fspec-tui | `src/app/dispatch_reconnect.rs` | call `replay_session_snapshot()` on Reconnected |
| codelet-fspec-tui | `src/transport/mod.rs` | `session_removed_rx` (default closed) + `async session_snapshot` (default empty) on `FspecBackend` |
| codelet-fspec-tui | `src/transport/embedded.rs` | both: delegate to service/handle |
| codelet-fspec-tui | `src/transport/websocket.rs` | per-subscriber event splitter; `session_snapshot` via client-slot read + snapshot rx |
| codelet-fspec-tui | `tests/*` | see §4 |

## 4. Test plan (this card)

TUI integration tests in `rust/fspec-tui/tests/` (shape:
`spawned_subordinate_session_registration_parity_rpc385.rs` for the
subscriber-wiring pattern; `session_restore_tui113.rs` for the
App-level harness; `common/mod.rs` helpers for embedded+WS dual-backend
suites):

1. **Embedded: created fan-out** (regression, exists — extend): manager
   creates a session the TUI didn't initiate → tab appears.
2. **Embedded: removed event** → `Action::SessionRemoved` removes the tab,
   re-clamps mux, clears WU binding; double-emit is a no-op.
3. **WS: created fan-out via wire**: WS backend against a live
   `bind_and_serve` daemon (rpc-server test harness); client A creates;
   client B's App subscriber adds the tab. (End-to-end cross-client
   parity — the user's exact reported scenario.)
4. **WS: removed fan-out via wire**: A destroys; B's tab disappears.
5. **Snapshot replay (embedded)**: pre-create 2 sessions in the manager,
   bootstrap a fresh App → both tabs present, board WU state intact.
6. **Snapshot replay (WS)**: A creates a session, B connects + bootstraps
   → B sees the tab; B's focused session (if any) per BUG-206 restore
   rules is unaffected.
7. **Reconnect replay**: kill + reconnect B's transport (supervisor
   supervisor path or manual slot swap in test) → snapshot re-applied, no
   duplicate tabs (idempotent guard).
8. **Degenerate backend** (mock): `session_removed_rx` closed /
   `session_snapshot` empty → bootstrap completes, no tabs, no panics
   (default-impl convention holds).

Source-shape tests (project convention for wiring regressions):
- `mod.rs` contains `session_removed_rx` with a closed-receiver default;
- `dispatch.rs` routes `Action::SessionRemoved`;
- bootstrap spawns the session-removed subscriber task (count check).

## 5. Dependencies & ordering

- HARD dependency on RPC-433: the WS half of every test in §4 items 3-7
  needs the wire frames + `FspecWsClient` accessors. The EMBEDDED half
  (items 1-2, 5, 8) needs only the manager's `session_removed_tx` +
  `live_session_infos` (RPC-433 §2.2-2.3). Sequence: RPC-433 first (or at
  least its sessions/core/rpc layers), then RPC-434.
- `Action::SessionRemoved` must land before the bootstrap subscriber that
  emits it (same card).
- Keep files ≤ 300 lines (workspace standard) — `handle_session_removed`
  goes in a new `dispatch_session_lifecycle.rs`; bootstrap.rs grows only
  by the helper call + one loop.
