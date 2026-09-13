# BUG-180: SessionHeader work-unit status goes stale

## Problem

The `SessionHeader` in the AgentView renders `(WU-ID: status)` for the work
unit bound to the focused session. This status is **frozen at attach time**
and never updates when the work unit's status changes.

### Reproduction

1. Attach a work unit (e.g. `AUTH-001` in `backlog`) to an agent session via the BoardView.
   The SessionHeader shows `(AUTH-001: backlog)`.
2. Move `AUTH-001` to `specifying` (via BoardView key, slash command, or the LLM's
   `update-work-unit-status` tool call).
3. BoardStore updates immediately — the board view shows `AUTH-001` in the `specifying`
   column. ✅
4. The SessionHeader still shows `(AUTH-001: backlog)`. ❌

The stale value persists until the session is detached and re-attached.

---

## Architecture: How work-unit state flows through fspec

### Data source (single source of truth)

`spec/work-units.json` is the single source of truth. Every status change
(`update-work-unit-status`, `update-work-unit`, `prioritize-work-unit`,
`delete-work-unit`, etc.) writes atomically to this file via
`codelet_core::work_units_write` (proper-lockfile compatible, `mkdir` lock on
`spec/work-units.json.lock`).

### Watcher (change detection)

`codelet_core::work_units::WorkUnitsWatcher` watches `spec/work-units.json`
via `notify` + `notify-debouncer-mini` (100 ms debounce). On every change it
reads the full snapshot and publishes it on a `tokio::sync::broadcast`
channel with capacity 64. The payload is a **full snapshot**
(`Vec<WorkUnitInfo>`), not an incremental delta — lagging subscribers simply
miss a frame and resync on the next.

### Transport layer (two paths)

Both paths deliver the same `Vec<WorkUnitInfo>` payload:

**Embedded (in-process):**
```
WorkUnitsWatcher::subscribe()
  → SharedFspecService::watcher_rx()
  → EmbeddedTransport::work_units_rx()
  → FspecBackend::work_units_rx() [broadcast::Receiver]
```

**WebSocket (remote daemon):**
```
WorkUnitsWatcher → rpc-server Envelope::WorkUnitsUpdate frame
  → FspecWsClient::ClientInbound::on_work_units_update
  → tokio::sync::watch (latest-snapshot-wins)
  → FspecWsClient::work_units_rx() [broadcast::Receiver]
```

Both transports implement the identical `FspecBackend::work_units_rx()`
interface, so the App task consumes a single code path regardless of transport.

### App task subscriber

`App::spawn_subscriber_tasks()` (in `app/bootstrap.rs`) spawns five tokio
tasks. The `work_units_rx` task:

```rust
loop {
    match rx.recv().await {
        Ok(units) => tx.send(Action::WorkUnitsLoaded(units)),
        Err(RecvError::Lagged(n)) => {
            // resync: backend.list_work_units() → Action::WorkUnitsLoaded
        }
        Err(RecvError::Closed) => break,
    }
}
```

### App::dispatch — the single mutation surface

`App::dispatch` is the single-task tenere mutation point (RPC-009). The
`Action::WorkUnitsLoaded(units)` handler currently does **only**:

```rust
Action::WorkUnitsLoaded(units) => {
    self.board_store.replace_work_units(units.clone());
    // ← BUG: no sync of per-session WorkUnitContext
}
```

This is the **critical injection point** for the fix. It is reached by:
- The embedded watcher (via `work_units_rx` subscriber task)
- The WebSocket transport (via the same subscriber task over WS)
- Bootstrap (`App::bootstrap` → `backend.list_work_units()` →
  `Action::WorkUnitsLoaded`)

So any change to `spec/work-units.json` from any writer (TUI key, slash
command, LLM tool call, external CLI) will eventually arrive here.

### Store layout (two stores, two stale values)

**BoardStore** (Kanban view):
```rust
work_units: Vec<WorkUnitInfo>        // ← live, always up to date
by_column: HashMap<String, Vec<usize>>
```
`BoardStore.replace_work_units()` re-groups by status on every
`WorkUnitsLoaded` dispatch. BoardView renders directly from this — always
correct.

**AgentViewStore** (session chrome):
```rust
// Per-session work-unit binding (RPC-050)
work_unit_context_by_session: HashMap<SessionId, WorkUnitContext>
//    WorkUnitContext { id, title, status }  ← frozen at attach time

// Legacy single-session slots (RPC-029 fallback)
current_work_unit_id: Option<String>        // ← frozen at EnterWorkUnit
current_work_unit_status: Option<String>    // ← frozen at EnterWorkUnit
```

The per-session slot is written **only** on:
- `Action::WorkUnitAttached(session, ctx)` — at BoardView attach time
- `Action::WorkUnitDetached(session)` — cleared at `/detach`

It is **never** written on `Action::WorkUnitsLoaded`. This is the bug.

### SessionHeader read path

`views/agent/chrome_paint.rs::paint_header_and_role`:
```rust
let bound = sid.and_then(|s| store.work_unit_context_for(s));
let work_unit_id   = bound.map(|c| c.id.as_str())
                        .or_else(|| store.current_work_unit_id());
let work_unit_status = bound.map(|c| c.status.as_str())
                            .or_else(|| store.current_work_unit_status());
```

The per-session slot takes priority; the legacy slots are the fallback.
Both are stale after an external status change.

---

## The root cause

The TUI architecture correctly uses a **push-driven RPC event flow** for
work-unit state:

```
spec/work-units.json  →  WorkUnitsWatcher  →  Envelope/broadcast
  →  FspecBackend::work_units_rx  →  App subscriber task
  →  Action::WorkUnitsLoaded  →  App::dispatch
```

The push **reaches** the App task. The bug is in the **reducer**: the
`Action::WorkUnitsLoaded` handler updates only `BoardStore` and ignores
`AgentViewStore`'s per-session work-unit context. The per-session slot
becomes a **frozen snapshot** from attach time.

This is a **missing projection** — the push event is delivered but the
per-session projection is never updated.

---

## Related state issues found

### 1. Legacy `AgentViewStore.current_work_unit_status` (same bug, different field)

`current_work_unit_id` / `current_work_unit_status` are set on
`EnterWorkUnit` (BoardView Enter path) and `MuxEnterWorkUnit` (mux grid
Enter path). They are the **fallback** source for SessionHeader when no
per-session binding exists. They are also stale — but less visible because
the per-session path usually shadows them.

**Important coupling:** the legacy `current_work_unit_id` slot is the
**late-attach source of truth** for lazily-created sessions.
`handle_session_created` (dispatch_create_session_dialog.rs) re-attaches a
newly-created session to the unit held in `current_work_unit_id`:

```rust
if let Some(id) = self.agent_view_store.current_work_unit_id().map(...) {
    let _ = self.action_tx.send(Action::AttachSession(id.clone(), session.clone()));
    let _ = self.action_tx.send(Action::AttachWorkUnitToSession(id));
}
```

The `WorkUnitContext` built at attach time comes from
`lookup_work_unit_context(&work_unit_id)` which reads the **fresh**
BoardStore — so a new session always gets the current status. The stale
slot is therefore display-only for id (the id never changes) and for
status (only painted when no per-session binding exists, i.e. between
EnterWorkUnit and the lazy session's attach round-trip completing).

**Fix:** same `WorkUnitsLoaded` handler should also update these slots when
the focused session's bound work unit has changed status.

### 2. Backend-side `WorkUnitContext` (on `BackgroundSession`)

`codelet_sessions::BackgroundSession` holds its own
`RwLock<Option<WorkUnitContext>>` which is written by
`FspecService::set_work_unit_context`. When the LLM calls
`update-work-unit-status` via the `FspecTool` (the `codelet_tools::FspecHandler`
closure → `codelet_fspec_core::dispatch_command`), the work unit's status in
`spec/work-units.json` changes but the `BackgroundSession`'s
`work_unit_context` slot is **not updated** — it stays at the last value
passed via `set_work_unit_context`.

This affects **BLOCK-006** stage gating (`work_unit_stage()` callback reads
`BackgroundSession::get_work_unit_context()` → `status`). After an
`update-work-unit-status` tool call, the blocklist stage gate uses the
stale status until the next `set_work_unit_context` call.

**Impact:** medium — stage gates may use the old status for the remainder of
the session until re-attached. Lower priority than the TUI display bug but
worth noting.

### 3. `StreamChunk::WorkUnitsUpdate` is dead in the TUI

`session_context.rs` marks `StreamChunk::WorkUnitsUpdate { .. }` as a
state-only chunk that is "consumed elsewhere". In
`dispatch_stream_chunks.rs::handle_stream_chunk_state_updates` it falls into
the `_ => {}` catch-all. The TUI uses the **separate**
`work_units_rx` broadcast (not per-session chunk stream) for work-unit
updates. The `StreamChunk::WorkUnitsUpdate` variant is only used by:
- The NAPI TS shim (`startWorkUnitsWatcher` → `StreamChunk::work_units_update`)
- The WS transport's envelope pump (for the WS transport's `work_units_rx`)

The TUI's `Action::WorkUnitsLoaded` path (via `work_units_rx`) is the
correct and sole path for the Rust TUI. No change needed here — just
documenting that the `StreamChunk::WorkUnitsUpdate` variant is correctly
unused in the TUI chunk stream.

---

## Correct architecture for monitoring state change

### Principle: Single writer projection at the App task

The existing push flow is architecturally correct. The fix is to complete
the projection at the single mutation point (`App::dispatch`), not to add
new push paths or poll.

```
spec/work-units.json (any writer)
  → WorkUnitsWatcher (debounced fs-watch)
  → broadcast / Envelope::WorkUnitsUpdate
  → FspecBackend::work_units_rx (transport-agnostic)
  → App subscriber task
  → Action::WorkUnitsLoaded(Vec<WorkUnitInfo>)
  → App::dispatch:
       1. BoardStore.replace_work_units(units)          ← already done
       2. AgentViewStore.sync_work_unit_contexts(units) ← NEW: sync both
                                                          per-session slots
                                                          and legacy fallback
```

### Why this is the right place

1. **Single writer** — the App task is the only place both stores are
   mutated (RPC-009 tenere pattern). Adding a sync here keeps the invariant.
2. **Transport-agnostic** — the fix is reached by both embedded and WS
   transports without any change to either.
3. **No new push path** — the event already arrives; we just complete the
   reducer.
4. **Idempotent** — `sync_work_unit_contexts` is safe to call on every
   `WorkUnitsLoaded` dispatch (including the bootstrap initial load).
   For each session with a bound work unit, it re-reads the current
   status from the fresh snapshot. If the status hasn't changed, the
   update is a no-op (same value written back).

### Why not a new `Action::WorkUnitStatusChanged`

A delta action would require:
- The watcher to emit per-unit deltas instead of full snapshots (breaking
  the full-snapshot contract)
- The App task to track per-unit previous state to detect deltas (adding
  the same `HashMap<unit_id, previous_status>` the sync already avoids)
- Two separate action variants to handle (delta and full-snapshot)

The full-snapshot `WorkUnitsLoaded` payload is already available and the
sync is a simple lookup — O(number of open sessions × 1 hash lookup).

---

## Fix plan

### 1. Add `AgentViewStore::sync_work_unit_contexts(units)`

New method in `store/agent_view/work_unit_state.rs`:

```rust
/// BUG-180: sync the per-session WorkUnitContext.status (and title) for
/// every session that has a work unit bound, against the fresh snapshot.
/// Also updates the legacy fallback slots when the focused session has
/// a bound work unit.
///
/// Called from `App::dispatch` on `Action::WorkUnitsLoaded` AFTER
/// `board_store.replace_work_units(units)` — the single-writer invariant
/// is preserved because this runs on the App task.
///
/// O(n) where n = number of open sessions. Each session's bound unit id
/// is looked up in the fresh snapshot by `id` (linear scan is fine for
/// the realistic < 10 open sessions; the BoardStore does the same).
pub fn sync_work_unit_contexts(&mut self, units: &[WorkUnitInfo]) {
    // Build a fast lookup: unit_id → (title, status)
    let lookup: std::collections::HashMap<&str, (&str, &str)> = units
        .iter()
        .map(|u| (u.id.as_str(), (u.title.as_str(), u.status.as_str())))
        .collect();

    // Sync per-session slots
    for (sid, ctx) in self.work_unit_context_by_session.iter_mut() {
        if let Some((title, status)) = lookup.get(ctx.id.as_str()) {
            ctx.title = title.to_string();
            ctx.status = status.to_string();
        }
        // If the unit is gone from the snapshot (deleted), leave the
        // slot untouched — the session's binding survives deletion of
        // the unit. The board shows the unit as gone; the header
        // continues to show the last known status until /detach.
    }

    // Sync legacy fallback slots (the "current" single-session slots).
    // Only update when the focused session has a per-session binding
    // with a known unit in the fresh snapshot.
    if let Some(current_sid) = self.current_session().cloned() {
        if let Some(ctx) = self.work_unit_context_by_session.get(&current_sid) {
            if let Some((title, status)) = lookup.get(ctx.id.as_str()) {
                self.current_work_unit_id = Some(ctx.id.clone());
                self.current_work_unit_status = Some(status.to_string());
                let _ = title; // title not stored in legacy slots
            }
        }
    }
}
```

### 2. Wire it into `App::dispatch` (AS IMPLEMENTED)

In `app/dispatch.rs`, the `Action::WorkUnitsLoaded` arm:

```rust
Action::WorkUnitsLoaded(units) => {
    self.board_store.replace_work_units(units.clone());
    // BUG-180: project the fresh snapshot onto the per-session
    // WorkUnitContext bindings (+ legacy fallback slots).
    self.agent_view_store.sync_work_unit_contexts(units);
}
```

Final implemented semantics (superset of the sketch above):

1. **Per-session slots** — for each entry in
   `work_unit_context_by_session`, rewrite `title` + `status` when the
   unit id is present in the snapshot; preserve the binding verbatim
   when the unit was deleted (a deleted unit must not silently detach
   the session — `/detach` or a new attach is the explicit teardown).
2. **Legacy fallback slots** — `current_work_unit_status` is refreshed
   from the snapshot whenever `current_work_unit_id` names a unit that
   exists in the snapshot (covers both the focused-session binding and
   standalone legacy slots set by `EnterWorkUnit` before the lazy
   session attach round-trip completes). When the snapshot no longer
   contains that id, the status is cleared while the id is preserved
   (id is stable; only the status is the live value). Note this is a
   deliberate divergence from the per-session rule: the fallback slots
   are display-only and the "unit gone" state should not paint a stale
   status, whereas the per-session binding is the authoritative chrome
   source and must survive deletion so the header keeps showing where
   the session's work lives until explicit detach.
3. **Detached sessions** — no entry in
   `work_unit_context_by_session` → no write; a snapshot can never
   invent a binding.

### 3. Also fix the backend-side stale `WorkUnitContext` (optional, separate card)

After `codelet_fspec_core::dispatch_command` runs a successful
`update-work-unit-status`, the `FspecAgentHooks` (or the
`FspecHandler` closure in `agent_loop.rs`) should re-read the unit's new
status from `spec/work-units.json` and call
`session.set_work_unit_context(new_ctx)` on the originating session's
`BackgroundSession` to update the BLOCK-006 stage gate. This is a
separate fix (BACKEND-180) that should be a follow-up card — the TUI
display fix does not depend on it.

### 4. Test plan

```
tests/work_unit_status_sync_bug180.rs

Scenario 1: work_units_loaded_syncs_per_session_context_status
  Given App with session s-1 bound to AUTH-001 (status "backlog")
  And AgentViewStore.current_work_unit_id == Some("AUTH-001")
  When Action::WorkUnitsLoaded([AUTH-001 implementing]) is dispatched
  Then AgentViewStore.work_unit_context_for(s-1).status == "implementing"
  And AgentViewStore.current_work_unit_status == Some("implementing")

Scenario 2: work_units_loaded_syncs_title
  Given App with session s-1 bound to AUTH-001 (title "Old Title", status "backlog")
  When Action::WorkUnitsLoaded([AUTH-001 title="New Title" status="backlog"]) is dispatched
  Then AgentViewStore.work_unit_context_for(s-1).title == "New Title"

Scenario 3: deleted_unit_preserves_stale_binding
  Given App with session s-1 bound to AUTH-001 (status "backlog")
  When Action::WorkUnitsLoaded([AUTH-002 backlog]) is dispatched  // AUTH-001 gone
  Then AgentViewStore.work_unit_context_for(s-1) is still Some
  And AgentViewStore.work_unit_context_for(s-1).status == "backlog"  // preserved

Scenario 4: work_units_loaded_does_not_overwrite_detached_sessions
  Given App with two sessions s-1 (bound to AUTH-001) and s-2 (detached)
  When Action::WorkUnitsLoaded([AUTH-001 implementing]) is dispatched
  Then AgentViewStore.work_unit_context_for(s-1).status == "implementing"
  And AgentViewStore.work_unit_context_for(s-2) is None  // unchanged

Scenario 5: SessionHeader_renders_fresh_status_after_sync
  Given App with session s-1 bound to AUTH-001 (status "backlog")
  And AgentView in Agent mode
  When Action::WorkUnitsLoaded([AUTH-001 implementing]) is dispatched
  And AgentView is rendered
  Then the top row contains "(AUTH-001: implementing)"
  And the top row does NOT contain "(AUTH-001: backlog)"
```

---

## Feature file for BUG-180

The feature file `spec/features/bug180-sessionheader-wu-status-stale.feature`
should be created with the scenarios above. The work unit is in `rust-frontend`
epic.
