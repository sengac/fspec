# RPC-385 — Spawned subordinate agents are not registered/visible in the Rust TUI

## Problem Statement

When the `AgentManager` tool runs `spawn` (driven by an LLM tool call), the
subordinate session **is genuinely created** on the backend, but the embedded
Rust TUI never learns about it, so **no agent tab appears** and the spawned
agent is invisible to the user. The supervisor receives a valid `session_id`
back (proving creation succeeded), yet the TUI shows nothing.

This was discovered by deep-searching the TypeScript reference (`src/`) and the
Rust port (`codelet/`) for the spawn → visibility flow.

---

## Backend creation — COMPLETE (not the bug)

`handle_spawn()` in `codelet/agent-loop/src/agent_manager_handler.rs`
(byte-identical copy in `codelet/napi/src/agent_manager_handler.rs`) correctly:

1. Generates `subordinate_id = Uuid::new_v4()` + name `"Agent <8hex>"`.
2. Validates the spawner's model string.
3. Writes a persistence manifest (`SessionManifest::with_provider` + `save_session`) — searchable via SessionSearch.
4. Creates the in-memory session: `SessionManager::create_session_with_id(subordinate_id, model, project, name)`.
5. Propagates per-model limits (MODEL-005).
6. Registers the relationship: `session_manager.add_supervisor(subordinate_id, spawner_id)` (ChainOfCommand).
7. Sets the role if provided.
8. Starts `spawn_subordinate_forwarding_task()` (forwards the subordinate's chunks up the supervisor chain to the root parent's relay connections for the external dashboard).
9. Returns `AgentManagerResult::Spawned { session_id }`.

So the session, manifest, and chain-of-command relationship all exist. **Creation
is not the problem.**

---

## The gap — TUI is never notified

The Rust TUI adds an agent tab through exactly ONE path:

```
Action::SessionCreated(SessionId)
  → App::dispatch (codelet/fspec-tui/src/app/dispatch.rs:124)
  → handle_session_created()  (dispatch_create_session_dialog.rs:122)
  → agent_view_store.append_session(SessionContext::new(...))
```

`Action::SessionCreated` is emitted **only** by TUI-initiated creation:
- create-session dialog (`dispatch_create_session_dialog.rs:84`)
- bootstrap / lazy "enter work unit"
- reconnect / resume overlay (`dispatch_resume_search_views.rs:101`)

A spawn that happens inside Rust (LLM tool call, not a user action) never produces
this action. Concrete structural gaps:

### Gap 1 — No session-creation broadcast exists
`SessionManager` (`codelet/sessions/src/session_manager.rs`) has broadcast senders
`chunks_tx`, `logs_tx`, `status_changes_tx` — but **no `session_created_tx`**.
`create_session_with_id` simply does `sessions.write().insert(uuid, session)` with
no lifecycle event. (grep for `session_created_tx` / `SessionListChanged` / `notify_session_created` → nothing.)

### Gap 2 — `FspecBackend` trait has no creation-subscription channel
`codelet/fspec-tui/src/transport/mod.rs` exposes only `work_units_rx()`,
`chunks_rx()`, `logs_rx()`, `status_changes_rx()`. No "a new session appeared"
channel exists.

### Gap 3 — TUI subscriber tasks wire only those four channels
`bootstrap.rs::spawn_subscriber_tasks` (lines ~137-210) spawns subscribers for
work_units / chunks / logs / status_changes. None carries session creation.

### Gap 4 — Chunks from an unknown session are silently dropped
`create_session_with_id` emits one `IsolationStateChange` chunk on `chunks_tx`,
which reaches the TUI as `Action::ChunkReceived`. But the handler is:
```rust
if let Some(ctx) = self.agent_view_store.session_context_mut_for(id) {
    ctx.record_chunk(chunk);
}
```
`session_context_mut_for` only **finds** an existing context — it never creates
one. So the chunk lands nowhere and no tab is born. Same for forwarded
subordinate chunks.

### Gap 5 — The only "creation" notification goes to the dashboard
`create_session_with_id` calls `codelet_tools::broadcast_metadata_update()`
(`bridge_relay.rs`), which sends a `relay/metadataUpdate` envelope over bridge
WebSockets to the **external JS dashboard** (SESS-015). The embedded TUI does not
consume metadata updates and has no equivalent.

### Gap 6 — No reconciliation poll
`backend.list_sessions()` *would* reveal the subordinate, but it is only called
in the manual `/resume` search-view flow. There is no periodic reconcile diffing
`list_sessions()` against `open_sessions`.

---

## How the TypeScript reference avoids this

The TS terminal TUI (`src/tui/components/AgentView.tsx`) uses a **pull/query**
model rather than a creation event:
- `sessionManagerList()` is queried for numbering and merged into the resume
  overlay (lines ~4072-4121); any session absent from persistence is appended as
  a "Background Session".
- Navigation uses `sessionGetNext()` / `sessionGetPrev()` / `sessionGetSubordinate()`
  (`src/tui/utils/sessionNavigation.ts`), so Shift+←/→ reaches a spawned agent
  because Rust now returns it.
- Live output flows through `GlobalSessionStreamManager`
  (`sessionSetGlobalChunkCallback` → per-session `registerHandler`).

The Rust port ported **neither** the auto-query nor a notify path for spawned
subordinates — hence the invisibility.

---

## Chosen fix — Approach A: session-lifecycle broadcast

This is the cleanest option and mirrors the **existing** broadcast architecture
(`chunks_tx` / `logs_tx` / `status_changes_tx`).

1. **Backend broadcast.** Add `session_created_tx: broadcast::Sender<SessionInfo>`
   (or `(SessionId, name)`) to `SessionManager`, with an accessor
   `session_created_tx()`. Fire it inside `create_session_with_id` right after the
   session is inserted into the `sessions` map (alongside the existing
   `broadcast_metadata_update()` call). This fires for **every** created session,
   including LLM-spawned subordinates.

2. **Transport surface.** Add `fn session_created_rx(&self) -> broadcast::Receiver<...>`
   to the `FspecBackend` trait. Implement it in the embedded backend by returning
   `session_manager.session_created_tx().subscribe()`. The remote (tarpc/WebSocket)
   backend may provide a default no-op receiver for now; full remote-transport
   parity is tracked as an explicit follow-up (OUT OF SCOPE here).

3. **TUI subscriber.** In `spawn_subscriber_tasks`, add a subscriber that folds
   `session_created_rx` events into `Action::SessionCreated(id)`.

4. **Idempotent append.** Because TUI-initiated creates ALREADY emit
   `Action::SessionCreated`, `handle_session_created` / `append_session` must be
   **idempotent**: if a `SessionContext` for that id already exists, do NOT append
   a duplicate tab. This makes the broadcast safe for all creation paths —
   user-initiated tabs are a no-op, spawned subordinates get added exactly once.

### Invariants / guard-rails
- No regression for TUI-initiated session creation (no duplicate tabs).
- Keep all touched Rust files < 300 lines (extract helpers if needed).
- Tests follow the `*_parity_rpcNNN.rs` convention where they assert parity with
  the TypeScript reference behavior, plus store-level unit tests for idempotency.
- The lagged-receiver branch must resync gracefully (mirror the existing
  `work_units_rx` lag handling).

---

## Out of scope (explicit follow-ups)
- **Remote/tarpc transport parity:** carrying `session_created` events over the
  WebSocket service surface so a *remote* ratatui client also sees spawned
  subordinates. The embedded (in-process) backend — what the local Rust TUI uses
  — is the target of this card.
- **Subordinate-specific UI affordances** (e.g. visually distinguishing a
  subordinate tab, or sidebar grouping by supervisor). This card only ensures the
  spawned agent becomes a visible, navigable session.

---

## Key file references
| Concern | Location |
|---|---|
| Spawn handler | `codelet/agent-loop/src/agent_manager_handler.rs::handle_spawn` |
| Session creation | `codelet/sessions/src/session_manager.rs::create_session_with_id` (~429-670) |
| Existing broadcasts | `session_manager.rs` `chunks_tx`/`logs_tx`/`status_changes_tx` (~157-219) |
| Backend trait | `codelet/fspec-tui/src/transport/mod.rs` (`*_rx` methods ~82-90) |
| Subscriber tasks | `codelet/fspec-tui/src/app/bootstrap.rs::spawn_subscriber_tasks` (~137-210) |
| SessionCreated handler | `codelet/fspec-tui/src/app/dispatch_create_session_dialog.rs::handle_session_created` (~122) |
| append_session | `codelet/fspec-tui/src/store/agent_view.rs::append_session` (~144) |
| Dashboard-only notify | `codelet/tools/src/bridge_relay.rs::broadcast_metadata_update` |
