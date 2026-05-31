# AST Research: RPC-040 — Move SessionManager into codelet-sessions

This research uses `ast-grep` and `grep` to identify the exact symbols, line spans and call sites that RPC-040 must move from `codelet/napi/src/session_manager.rs` into `codelet/sessions/src/`. The findings reconcile the attachment's quoted line numbers (which have drifted by ~1100 LOC) with the actual codebase state.

## 1. SessionManager struct + impl

- `pub struct SessionManager` at `codelet/napi/src/session_manager.rs:2135` (was reported as line 3141 in attachment)
- `impl Default for SessionManager` at line 2148
- `impl SessionManager` block opens at line 2154 and closes at line 3013
- Total span: ~880 LOC (2135-3013)

Methods inside the impl:
- `new()` line 2156
- `set_default_model(&self, model: &str)` line 2170
- `get_default_model(&self) -> Option<String>` line 2177
- `session_count(&self)` line 2181
- `live_session_ids(&self)` line 2185
- `find_session_by_schedule_name(...)` line 2190
- `spawn_scheduled_session(...)` line 2208
- `instance() -> &'static SessionManager` line 2246
- `create_session(...)` line 2253
- `create_session_with_id(&self, id: &str, model: &str, project: &str, name: &str) -> Result<()>` line 2264
- `create_isolated_session_with_id(...)` line 2538
- `list_sessions(...)` line 2798 (approx)
- `destroy_session(...)` line 2939 (approx)
- Supervisor delegation methods (add_supervisor, remove_supervisor, get_supervisors, get_subordinates) 2989-3011

## 2. ChainOfCommand struct + impl

- `pub struct ChainOfCommand` at line 354
- `impl Default for ChainOfCommand` line 361
- `impl ChainOfCommand` block at line 367
- Closing brace at line 512
- Total span: ~160 LOC (354-512)

Methods:
- `new()` line 369
- `add_supervisor(&self, subordinate_id, supervisor_id) -> Result<(), String>` line 384
- `remove_supervisor(&self, supervisor_id)` line 434
- `get_supervisors(&self, subordinate_id) -> Vec<Uuid>` line 459
- `get_subordinate(&self, supervisor_id) -> Option<Uuid>` line 467
- `get_subordinates(&self, supervisor_id) -> Vec<Uuid>` line 476
- `cleanup_subordinate(&self, subordinate_id)` line 484 (approx)
- `is_empty(&self)` line 509

## 3. NAPI-private dependencies referenced from SessionManager methods

Found via `grep -n "crate::scheduler\|crate::navigation\|crate::credentials\|GLOBAL_CHUNK_CALLBACK\|spawn_footer_poller\|stop_footer_poller\|agent_loop("`:

### `crate::credentials::resolve_and_set_env_var(...)` — 2 call sites
- Line 2352 inside `create_session_with_id`
- Line 2635 inside `create_isolated_session_with_id`

### `agent_loop(...)` — 2 call sites
- Line 2500 (inside create_session_with_id, spawned via `tokio::spawn(async move { agent_loop(...).await })`)
- Line 2761 (inside create_isolated_session_with_id, same pattern)

### `GLOBAL_CHUNK_CALLBACK.get()` — 2 call sites in SessionManager methods
- Line 2517 (create_session_with_id, emits `IsolationStateChange`)
- Line 2771 (create_isolated_session_with_id, emits `IsolationStateChange`)
- Also defined at line 78: `static GLOBAL_CHUNK_CALLBACK: OnceCell<GlobalChunkCallback> = OnceCell::new();`

### `spawn_footer_poller(...)` / `stop_footer_poller(...)` — 3 call sites
- Line 2523 inside create_session_with_id
- Line 2780 inside create_isolated_session_with_id
- Line 2956 inside destroy_session
- Defined at line 5104 (free fn) and 5223 (free fn) — stay napi-side

### `crate::scheduler::spawn_scheduler(...)` — 2 call sites
- Line 2845
- Line 2863

### `crate::scheduler::LoopStore::instance()` — 1 call site
- Line 2961 inside destroy_session

### `crate::navigation::*` — 3 call sites
- Line 2874: `use crate::navigation::{build_navigation_list, get_next_target, NavigationTarget};`
- Line 2898: `use crate::navigation::{build_navigation_list, get_prev_target, NavigationTarget};`
- Line 2917: `use crate::navigation::build_navigation_list;`

## 4. Files to lift (NAPI-free already)

- `codelet/napi/src/navigation.rs` (285 lines) → `codelet/sessions/src/navigation.rs`
  - Has only one offending import: `use crate::session_manager::{BackgroundSession, ChainOfCommand};` → rewrite to `use crate::background_session::BackgroundSession; use crate::chain_of_command::ChainOfCommand;`
- `codelet/napi/src/credentials/mod.rs` (23 lines), `resolver.rs` (250 lines), `store.rs` (206 lines), `types.rs` (41 lines)
- `napi_bindings.rs` (21 lines) STAYS in napi

## 5. NAPI free functions that wrap SessionManager (stay in napi)

Searched via `grep "session_manager_"`:
- `session_manager_create`
- `session_manager_create_isolated`
- `session_manager_list`
- `session_manager_destroy`
- `session_manager_add_supervisor`
- `session_manager_remove_supervisor`
- `session_manager_get_supervisors`
- `session_manager_get_subordinates`
- `session_manager_get_active`
- `session_manager_set_active`
- `session_manager_clear_active`
- `session_set_global_chunk_callback`

These all stay napi-side and must continue working through `pub use codelet_sessions::session_manager::SessionManager;` re-export.

## 6. Pre-existing in-file unit tests (ChainOfCommand) that must keep passing via re-export

Lines 1180-1450 in `codelet/napi/src/session_manager.rs`:
- `test_register_supervisor_for_subordinate_session`
- `test_subordinate_with_multiple_supervisors`
- `test_query_subordinate_for_supervisor`
- `test_remove_supervisor_relationship`
- `test_supervisor_can_observe_multiple_subordinates`
- `test_duplicate_subordinate_under_same_supervisor_rejected`
- `test_circular_supervision_prevented`
- `test_regular_session_has_no_subordinate`
- `test_cleanup_supervisors_when_subordinate_removed`

## 7. NapiSessionManagerHooks delegation targets

After lifting, NapiSessionManagerHooks (lives in napi) must delegate:
- `spawn_agent_loop` → `tokio::spawn(async move { agent_loop(session, input_rx, mcp_injection_rx).await })`
  where `agent_loop` is the free fn at line 3620
- `spawn_scheduler` → `crate::scheduler::spawn_scheduler(project, rt)`
- `spawn_footer_poller` → free fn at line 5104
- `stop_footer_poller` → free fn at line 5223
- `cleanup_session_loops` → `crate::scheduler::LoopStore::instance().remove_for_session(...)` (line 2961 pattern)
- `emit_isolation_state_change` → `GLOBAL_CHUNK_CALLBACK.get().map(|cb| cb.call(...))` (lines 2517/2771 pattern)

## 8. IsolatedSessionResult NAPI wrapper

Located at `codelet/napi/src/session_manager.rs:5258-5267`:
```rust
#[napi(object)]
pub struct IsolatedSessionResult {
    pub session_id: String,
    pub worktree_path: String,
    pub base_commit: String,
}
```

This struct stays in napi. The moved `create_isolated_session_with_id` returns `codelet_rpc_types::IsolatedSessionInfo` (already lifted by RPC-036). NAPI adds `impl From<codelet_rpc_types::IsolatedSessionInfo> for IsolatedSessionResult`.

## 9. SessionManagerHooks trait surface (new in codelet-sessions)

```rust
pub trait SessionManagerHooks: Send + Sync + 'static {
    fn spawn_agent_loop(
        &self,
        session: Arc<BackgroundSession>,
        input_rx: mpsc::Receiver<PromptInput>,
        mcp_injection_rx: mpsc::Receiver<IncomingMessage>,
    );
    fn spawn_scheduler(&self, project: String, rt: &Handle);
    fn ensure_scheduler_running_for_loop(&self, loop_id: &str);
    fn spawn_footer_poller(&self, session_id: String, cwd: String, worktree_path: Option<String>);
    fn stop_footer_poller(&self, session_id: &str);
    fn cleanup_session_loops(&self, session_id: &str);
    fn emit_isolation_state_change(&self, session_id: String, is_isolated: bool, worktree_path: Option<String>);
}

pub struct NoopSessionManagerHooks;
impl Default for NoopSessionManagerHooks { ... }
impl SessionManagerHooks for NoopSessionManagerHooks { /* all no-ops */ }
```

## 10. SessionManager new fields (per attachment)

The 5 existing fields + 4 new fields after the move:
```rust
pub struct SessionManager {
    sessions: RwLock<IndexMap<Uuid, Arc<BackgroundSession>>>,
    chain_of_command: ChainOfCommand,
    active_session_id: RwLock<Option<Uuid>>,
    scheduler_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
    default_model: RwLock<Option<String>>,
    // NEW for RPC-040:
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    logs_tx: broadcast::Sender<LogRecord>,
    status_changes_tx: broadcast::Sender<(SessionId, SessionStatus)>,
    hooks: ArcSwap<Arc<dyn SessionManagerHooks>>,
}
```

`SessionId`, `StreamChunk`, `LogRecord`, `SessionStatus` come from `codelet_rpc_types`. `ArcSwap` from `arc-swap` crate.
