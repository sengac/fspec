# AST Research — TUI-113: Auto-restore last active session on client startup and reconnect

Date: 2026-10-10
Method: Grep/AstGrep + direct source reads across `rust/fspec-tui`, `rust/sessions`,
`rust/core`, `rust/rpc`, `rust/rpc-types`.

## Call sites that create/attach sessions (the surface this work unit modifies)

| Location | Symbol | Role |
|---|---|---|
| `rust/fspec-tui/src/app/bootstrap.rs:27` | `App::bootstrap` | Seeds BoardStore + spawns subscriber tasks. No session creation (RPC-012). NEW: call `spawn_session_restore()` here. |
| `rust/fspec-tui/src/app/dispatch_reconnect.rs:135` | `App::handle_reconnected` | RPC-011 one-shot re-bootstrap: `list_work_units` + **unconditional `create_session(None)`** (lines 148–161). CHANGED: resolve-then-attach with create fallback (R4/R5). |
| `rust/fspec-tui/src/app/dispatch_work_unit_entry.rs:17` | `App::handle_enter_work_unit` | Lazy first-session creation when no session open (`create_session(None)` spawn, lines 44–60). UNCHANGED (RPC-012 safety net preserved). |
| `rust/fspec-tui/src/app/dispatch_attach.rs:29` | `App::handle_attach_to_session` | Existing attach path: append/focus SessionContext + `active_session_tx` + `refresh_session_chrome` + hydrate pending input/supervisors/debug + RPC-049 `resume_session` → `SessionResumeComplete` → `get_buffered_output(id, 1000)` replay. REUSED as the restore attach mechanism. NEW: fire-and-forget `set_active_session` (R3). |
| `rust/fspec-tui/src/app/dispatch_session_cycle.rs:134` | `App::switch_to_session_index` | RPC-024 focus switch (cycle, mux follow, chip activation BUG-195). NEW: fire-and-forget `set_active_session` (R3). |
| `rust/fspec-tui/src/app/session_creation.rs:45` | `route_bootstrap_create_session` | Routes bootstrap/reconnect create results onto the bus + active_session_tx. NEW: fire-and-forget `set_active_session` (R3). |
| `rust/fspec-tui/src/app/dispatch.rs:94` | `Action::BackToBoard` arm | Board exit. STICKY decision (2026-10-10): no `clear_active_session` here. |

## Backend trait surface already available (no new RPCs)

`rust/fspec-tui/src/transport/mod.rs` (`FspecBackend`, RPC-037 defaults):
- `async fn get_active_session(&self) -> Result<Option<SessionId>>` (line 589)
- `async fn set_active_session(&self, SessionId) -> Result<()>` (line 579)
- `async fn clear_active_session(&self) -> Result<()>` (line 584)
- `async fn list_sessions(&self, project_path: String) -> Result<Vec<SessionInfo>>` (line 72)
- `async fn get_workspace_info(&self) -> Result<WorkspaceInfo>` (RPC-018; `WorkspaceInfo { cwd, git_branch }` in `rust/rpc-types/src/lib.rs:423`)
- `async fn get_buffered_output(&self, SessionId, u32) -> Result<Vec<StreamChunk>>` (line 499)
- `async fn resume_session(&self, SessionId) -> Result<()>` (line 547)

Both `EmbeddedFspecBackend` (transport/embedded.rs:598) and
`WebSocketFspecBackend` (transport/websocket.rs:922) implement all of these —
cross-transport parity (RPC-037) is free.

## Daemon-side state

- `rust/sessions/src/session_manager.rs:545/561` — `SessionManager::set_active_session/get_active_session`:
  plain in-memory `RwLock<Option<Uuid>>`. Survives client disconnect, NOT daemon restart.
- `rust/sessions/src/session_manager.rs:371` — `list_sessions(project_path)`:
  merges in-memory + persisted; sorted by `updated_at_ms` DESC (TUI-099);
  EMPTY project_path = no filter. `SessionInfo` fields: `id, name, status, project,
  message_count, …, updated_at_ms` (`rust/rpc-types/src/lib.rs:291`).
- `rust/sessions/src/handle_impl.rs:173` — `resume_session`: loads manifest from disk,
  creates in-memory session if absent, restores messages + token state. Works for
  both live (already_exists) and disk-only sessions → one code path for both
  "live daemon" and "daemon restarted" restore.
- `rust/core/src/persistence/manifest.rs:216/366` — `SessionStore::last_session`
  per-project pointer + `get_last_session/resume_last` (NAPI-wired today; the
  TUI restore uses `list_sessions(daemon_cwd)` instead, so no new core RPC is needed).

## Test infrastructure

- `rust/fspec-tui/tests/common/mod.rs:217` — `MockBackend` (implements FspecBackend):
  - `seed_sessions(Vec<SessionInfo>)` (1459), `list_sessions_project()` capture (1047)
  - `create_session_calls()` (1210), `resume_session_calls()` (1616)
  - `get_buffered_output_calls()` (1641), `set_buffered_output(...)` scripting (1635)
  - `set_workspace_info(WorkspaceInfo)` (1405)
  - `get_active_session` / `set_active_session` NOT yet overridden → add
    `scripted_active_session` + `set_active_session_calls`/`last_set_active_session`
    (RPC-365 capture pattern, cf. `restore_checkpoint_file_calls`).
- Existing test patterns to mirror:
  - `tests/app_bootstrap_rpc009.rs` — bootstrap + `wait_for_action` bus drain helper
  - `tests/slash_resume_rpc049.rs:99` — attach → `resume_session` round-trip assertions
  - `tests/rpc037_cross_transport_parity.rs` — embedded vs websocket parity harness
    (dual backends against `SharedFspecService` + `StubSessionManagerHandle`, which
    already stores `active_session` — `rust/core/src/session_manager_handle.rs:1857`)

## Resolution-order anchors (R2)

1. Reconnect override: `self.agent_view_store.current_session()` (agent_view.rs:204).
2. `backend.get_active_session().await` → `Option<SessionId>`.
3. `daemon_cwd = backend.get_workspace_info().await.map(|w| w.cwd).unwrap_or_default()`
   (empty string → no project filter, per `list_sessions` doc) →
   `backend.list_sessions(daemon_cwd).await.first()` (TUI-099 ordering = most-recent first).

## Files the implementation touches

- NEW `rust/fspec-tui/src/app/session_restore.rs` (+ `mod` declaration in `app/mod.rs`)
- `rust/fspec-tui/src/app/bootstrap.rs` (call site)
- `rust/fspec-tui/src/app/dispatch_reconnect.rs` (replace create-on-reconnect)
- `rust/fspec-tui/src/app/dispatch_attach.rs` (set_active_session on attach)
- `rust/fspec-tui/src/app/dispatch_session_cycle.rs` (set_active_session on focus)
- `rust/fspec-tui/src/app/session_creation.rs` (set_active_session on create)
- `rust/fspec-tui/tests/common/mod.rs` (MockBackend active-session surface)
- NEW `rust/fspec-tui/tests/session_restore_tui113.rs` (10 scenario tests)
- `spec/features/auto-reconnect-supervisor.feature` (SUPERSEDED annotation, done)

## Conflicts / constraints

- 300-LoC ceiling: new module must stay small; dispatch files already near the limit.
- `handle_reconnected` must keep spawning the `list_work_units` refetch + subscriber
  respawn unchanged (RPC-011/RPC-415 invariants pinned by existing tests).
- `route_bootstrap_create_session` is also used by `handle_enter_work_unit` (lazy
  creation) — the `set_active_session` addition there is safe (idempotent).
- `Action::AttachToSession` when NO session tabs exist + view on Board:
  `handle_attach_to_session` appends the SessionContext; the Agent view flip must come
  from the dispatch arm (mirror `handle_open_agent_view(Some(sid))` at
  dispatch_session_cycle.rs:53-63: set navigation target + flip `active_view` +
  `menu_focus_to_columns()`).
