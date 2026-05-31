# RPC-040 — Move `SessionManager` from `codelet-napi` into `codelet-sessions`

**Parent:** RPC-030 · **Phase:** 4.3 · **Estimate:** 8 pts · **Depends on:** RPC-039

## Goal

Move the entire `SessionManager` struct + impl out of `codelet/napi/src/session_manager.rs` (lines 3141–4025) into `codelet/sessions/src/session_manager.rs`. All `#[napi]` attributes removed. Plain Rust methods only.

## Source — `codelet/napi/src/session_manager.rs`

### Struct (lines 3141–3152)

```rust
pub struct SessionManager {
    sessions: RwLock<IndexMap<Uuid, Arc<BackgroundSession>>>,    // 3142
    chain_of_command: ChainOfCommand,                            // 3144
    active_session_id: RwLock<Option<Uuid>>,                     // 3146
    scheduler_handle: RwLock<Option<tokio::task::JoinHandle<()>>>, // 3148
    default_model: RwLock<Option<String>>,                       // 3151
}
```

Add new fields (for RPC-041 wiring):
```rust
chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
logs_tx: broadcast::Sender<LogRecord>,
status_changes_tx: broadcast::Sender<(SessionId, SessionStatus)>,
```

### Methods (lines 3160–4025) — full list

| Line | Method |
|---|---|
| 3162 | `new() -> Self` |
| 3176 | `set_default_model(&self, model)` |
| 3183 | `get_default_model() -> Option<String>` |
| 3188 | `async fn session_count() -> usize` |
| 3193 | `async fn live_session_ids() -> Vec<Uuid>` |
| 3198 | `async fn find_session_by_schedule_name(name) -> Option<Uuid>` |
| 3214 | `async fn spawn_scheduled_session(...)` |
| 3250 | `pub fn instance() -> &'static SessionManager` (singleton) |
| 3257 | `async fn create_session(_model, project) -> Result<String>` |
| 3268 | `async fn create_session_with_id(id, model, project, name) -> Result<()>` (large body) |
| 3542 | `async fn create_isolated_session_with_id(...) -> Result<IsolatedSessionResult>` (large body — uses codelet-git worktree + codelet-providers ProviderManager) |
| 3802 | `list_sessions() -> Vec<SessionInfo>` |
| 3814 | `set_active_session(id)` |
| 3819 | `clear_active_session()` |
| 3824 | `get_active_session() -> Option<Uuid>` |
| 3833 | `maybe_start_scheduler(project)` |
| 3861 | `ensure_scheduler_running(project, rt)` |
| 3877 | `get_next_session() -> Option<String>` |
| 3901 | `get_prev_session() -> Option<String>` |
| 3920 | `get_first_session() -> Option<String>` |
| 3930 | `get_session(id) -> Result<Arc<BackgroundSession>>` |
| 3943 | `destroy_session(id) -> Result<()>` |
| 3993 | `add_supervisor(subordinate_id, supervisor_id) -> Result<(), String>` |
| 3998 | `remove_supervisor(supervisor_id)` |
| 4003 | `get_supervisors(subordinate_id) -> Vec<Uuid>` |
| 4008 | `get_subordinate(supervisor_id) -> Option<Uuid>` |
| 4013 | `get_subordinates(supervisor_id) -> Vec<Uuid>` |

### Replacements

- All `#[napi]` attributes removed (those live on the wrapper functions starting at line 6240+ — those stay in NAPI for RPC-043).
- `#[napi(object)]` on `IsolatedSessionResult` (line ~3540) → derive `Serialize + Deserialize` and live in `codelet_rpc_types` as `IsolatedSessionInfo` (from RPC-036). Or keep the internal type in `codelet_sessions` and `From<IsolatedSessionResult> for IsolatedSessionInfo`.
- `chain_of_command: ChainOfCommand` — confirm the type lives in `codelet-cli` or `codelet-core` (NAPI-free). If it lives in NAPI, lift it here.

### `Default` impl (lines 3154–3158)

```rust
impl Default for SessionManager {
    fn default() -> Self { Self::new() }
}
```

Moves with the struct.

## Imports to add

```rust
use std::sync::Arc;
use std::path::PathBuf;
use indexmap::IndexMap;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use uuid::Uuid;
use anyhow::Result;

use codelet_core::persistence::*;
use codelet_rpc_types::{
    SessionId, SessionInfo, SessionStatus, StreamChunk, LogRecord,
    IsolatedSessionInfo, // from RPC-036
};
use codelet_cli::session::Session;
use codelet_cli::session::context_gathering::IsolationContext;
use codelet_providers::ProviderManager;
use codelet_tools::{init_mcp_session, cleanup_mcp_session};
use codelet_git::create_worktree;

use crate::background_session::BackgroundSession; // from RPC-039
```

## NAPI-side cleanup

Delete lines 3141–4025 from `codelet/napi/src/session_manager.rs`. Replace with:

```rust
pub use codelet_sessions::session_manager::SessionManager;
```

The `#[napi]` wrapper functions starting around line 6240 (`session_manager_create`, `session_manager_list`, etc.) stay in NAPI for now — they call methods on the re-exported `SessionManager`. They become "thin adapters" in RPC-043.

## Behavioural notes

- `SessionManager::instance()` (line 3250) is a static singleton. After the move, it must continue to be a global — but it now lives in `codelet-sessions`. NAPI accesses it through the re-export. The `fspec` binary in RPC-044 will construct its OWN `Arc<SessionManager>` instead of using `instance()`.
- `create_isolated_session_with_id` body (lines 3542–~3800) is the most complex method — it constructs an isolation context via `codelet_cli::session::context_gathering::IsolationContext`, registers MCP via `codelet_tools::init_mcp_session`, and broadcasts metadata at line 3791. Confirm `broadcast_metadata_update` lives in `codelet_tools` (NAPI-free, line 924/3982/etc.).

## Acceptance criteria

1. `codelet/sessions/src/session_manager.rs` contains the full `SessionManager` struct + impl + `Default`.
2. No `napi::` references in moved code.
3. `cargo build -p codelet-sessions` passes.
4. `cargo build -p codelet-napi` passes (re-export keeps `#[napi]` wrappers working).
5. `cargo test -p codelet-napi` passes (existing NAPI tests).
6. The new `chunks_tx`, `logs_tx`, `status_changes_tx` fields exist on `SessionManager` but are not yet rewired into `BackgroundSession::handle_output` — that's RPC-041.

## Risks

- `SessionManager::instance()` is a footgun: NAPI startup currently calls it implicitly. The Rust frontend (RPC-044) must NOT use it — RPC-044 constructs a fresh `Arc<SessionManager>`. Both code paths must coexist during the transition.
- `ChainOfCommand` (line 3144) — confirm its location. If it's in `codelet/napi/src/`, this card must also lift it (likely to `codelet_core::chain_of_command` or `codelet_sessions::chain_of_command`).
- `IsolatedSessionResult` is `#[napi(object)]` — this struct must NOT move to `codelet-sessions` with the `napi(object)` decoration. Lift the data fields into `codelet_rpc_types::IsolatedSessionInfo` (already done in RPC-036). NAPI wrapper functions construct the `#[napi(object)]` decorated type at the boundary.

## Out of scope

- Replacing `GLOBAL_CHUNK_CALLBACK` → RPC-041.
- Implementing `SessionManagerHandle` → RPC-042.
- Reducing NAPI to a thin adapter → RPC-043.
