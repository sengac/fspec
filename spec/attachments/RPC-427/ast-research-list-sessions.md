# AST Research: list_sessions function signatures across the Rust codebase

## SessionManager::list_sessions
- File: `codelet/sessions/src/session_manager.rs:364`
- Signature: `pub fn list_sessions(&self) -> Vec<SessionInfo>`
- Calls `codelet_core::persistence::list_all_sessions()` at line 379
- No project filtering

## FspecService::list_sessions (tarpc trait)
- File: `codelet/rpc/src/lib.rs:69`
- Signature: `async fn list_sessions() -> Vec<SessionInfo>;`
- No project parameter

## FspecServiceImpl::list_sessions
- File: `codelet/rpc/src/lib.rs:945`
- Signature: `async fn list_sessions(self, _ctx: Context) -> Vec<SessionInfo>`
- Delegates to `handle.list_sessions()`

## FspecBackend trait
- File: `codelet/fspec-tui/src/transport/mod.rs`
- Signature: `async fn list_sessions(&self) -> Result<Vec<SessionInfo>>`
- No project parameter

## Call sites in fspec-tui
- `dispatch_resume_search_views.rs:38` — `spawn_list_sessions()` calls `backend.list_sessions().await`
- `dispatch_resume_search_views.rs:289` — `handle_confirm_delete_session()` calls `backend.list_sessions().await`
- `app_bootstrap_rpc009.rs` — tests call `backend.list_sessions()`

## Persistence functions available
- `list_sessions_for_project(project: &Path)` — filters by project
- `list_all_sessions()` — returns all sessions (currently used)

## Required changes
1. SessionManager::list_sessions(&self) → SessionManager::list_sessions(&self, project_path: &str)
2. FspecService trait → async fn list_sessions(project_path: String)
3. FspecServiceImpl → async fn list_sessions(self, _ctx: Context, project_path: String)
4. FspecBackend trait → async fn list_sessions(&self, project_path: String)
5. EmbeddedFspecBackend → pass project_path to client.list_sessions()
6. WebSocketFspecBackend → pass project_path to client.list_sessions()
7. TUI call sites → resolve std::env::current_dir() and pass it
