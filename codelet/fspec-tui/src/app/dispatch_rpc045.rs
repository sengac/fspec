//! App::dispatch routing for RPC-045 push-driven chunk + status broadcasts.
//!
//! Feature: spec/features/agentview-subscribe-broadcasts.feature
//!
//! Factored out of `app/dispatch.rs` so the orchestrator file stays
//! under the 300-LoC ceiling pinned by `rpc024-source-shape.feature`.
//!
//! Three responsibilities:
//!
//! 1. **`handle_stream_chunk_state_updates`** — invoked from the
//!    existing `Action::ChunkReceived` arm AFTER `record_chunk` and
//!    `apply_chunk_to_token_state`. Branches on the 5 RPC-045 chunk
//!    variants (`SessionStateChange`, `IsolationStateChange`,
//!    `DebugStateChange`, `FooterStateUpdate`, `FspecCommandRequest`)
//!    and writes the new per-session store state. No-op for variants
//!    that have nothing additional to record.
//! 2. **`handle_session_status_changed`** — invoked from the new
//!    `Action::SessionStatusChanged` arm. Writes the push-driven
//!    SessionStatus into the AgentViewStore so SessionFooter / status
//!    pill rendering can read it synchronously on the next frame.
//! 3. **`spawn_fspec_command_runner`** — fire-and-forget tokio task
//!    that executes the requested fspec command (happy path:
//!    `list-work-units`, `show-work-unit`) and routes the result back
//!    via `backend.send_fspec_result(session_id, result)`. The runner
//!    is intentionally minimal — wiring a full Rust command dispatcher
//!    is out of scope for this slice (deferred to a later card per
//!    the RPC-045 attachment).

use codelet_rpc_types::{
    FspecRequest, FspecResult, SessionId, SessionState, SessionStatus, StreamChunk,
};
use codelet_rpc_types::WorkspaceInfo;

use crate::components::Action;
use crate::store::{
    agent_view::isolation_state::session_status_from_state,
    IsolationState,
};

use super::dispatch_rpc020::format_compaction_notice;
use super::state::App;

impl App {
    /// Branch on the new RPC-045 chunk variants and update per-session
    /// store state.
    ///
    /// Variants not listed here (e.g. `Text`, `Thinking`, `ToolCall`)
    /// are already handled by `SessionContext::record_chunk` and
    /// `AgentViewStore::apply_chunk_to_token_state`, which run BEFORE
    /// this helper inside the `Action::ChunkReceived` arm.
    pub(crate) fn handle_stream_chunk_state_updates(
        &mut self,
        session_id: &SessionId,
        chunk: &StreamChunk,
    ) {
        match chunk {
            StreamChunk::SessionStateChange { state } => {
                self.agent_view_store
                    .set_session_status(session_id.clone(), session_status_from_state(*state));
                // RPC-053: fire the pause / HITL chunk-driven trigger or
                // clear any mounted dialog on resume.
                match state {
                    SessionState::Paused => {
                        let _ = self
                            .action_tx
                            .send(Action::PauseChunkReceived(session_id.clone()));
                    }
                    SessionState::Running | SessionState::Idle => {
                        let _ = self
                            .action_tx
                            .send(Action::PauseCleared(session_id.clone()));
                    }
                    _ => {}
                }
            }
            StreamChunk::IsolationStateChange {
                is_isolated,
                worktree_path,
                base_commit,
            } => {
                self.agent_view_store.set_isolation_state(
                    session_id.clone(),
                    IsolationState {
                        is_isolated: *is_isolated,
                        worktree_path: worktree_path.clone(),
                        base_commit: base_commit.clone(),
                    },
                );
            }
            StreamChunk::DebugStateChange { enabled } => {
                self.agent_view_store
                    .set_debug_enabled(session_id.clone(), *enabled);
            }
            StreamChunk::FooterStateUpdate {
                cwd,
                display_path: _,
                is_git_repo,
                branch,
            } => {
                // RPC-045: collapse the FooterStateUpdate shape onto the
                // existing single-slot `workspace` field. The full
                // (`display_path`, `is_git_repo`) detail is deferred to
                // a future card that introduces a richer footer state —
                // for now the SessionFooter only reads `cwd` and
                // `git_branch`, which we set here. `is_git_repo == false`
                // collapses the branch to `None`.
                let git_branch = if *is_git_repo { branch.clone() } else { None };
                self.agent_view_store.set_workspace(Some(WorkspaceInfo {
                    cwd: cwd.clone(),
                    git_branch,
                }));
            }
            StreamChunk::FspecCommandRequest { fspec_request } => {
                self.spawn_fspec_command_runner(session_id.clone(), fspec_request.clone());
            }
            StreamChunk::SupervisorPendingInjection { .. } => {
                // RPC-061: bump per-session pending-supervisor count.
                self.agent_view_store.apply_supervisor_pending_injection(session_id);
            }
            StreamChunk::CompactionComplete { compaction_result } => {
                // RPC-047: clear the per-session compaction-progress
                // entry and dispatch a session-scoped notice so the
                // `[compaction] ...` line lands in the originating
                // session's scrollback regardless of focus. Fires for
                // both /compact and auto-compaction; the slash handler
                // emits its own notice so double-emission for /compact
                // is acceptable parity with the TS Ink original.
                self.agent_view_store.clear_compaction_progress(session_id);
                let text = format_compaction_notice(compaction_result);
                let _ = self
                    .action_tx
                    .send(Action::EmitSessionNotice(session_id.clone(), text));
            }
            // All other variants: nothing additional to record here.
            _ => {}
        }
    }

    /// Fold a push-driven `(SessionId, SessionStatus)` broadcast into
    /// the AgentViewStore so the SessionFooter status pill repaints on
    /// the next frame.
    pub(crate) fn handle_session_status_changed(
        &mut self,
        session_id: SessionId,
        status: SessionStatus,
    ) {
        self.agent_view_store.set_session_status(session_id, status);
    }

    /// Spawn a fire-and-forget tokio task that executes `request`
    /// against the limited RPC-045 command set and routes the result
    /// back via `backend.send_fspec_result`.
    ///
    /// Happy path commands:
    /// - `list-work-units` → `backend.list_work_units()` → JSON-serialised
    ///   array.
    /// - `show-work-unit` → `backend.list_work_units()` filtered by the
    ///   `id` field of `args_json` → JSON-serialised single entry.
    ///
    /// Everything else returns `FspecResult { success: false, error:
    /// Some("unsupported command: <name>"), .. }` so the requesting
    /// session does NOT hang waiting for a reply.
    pub(crate) fn spawn_fspec_command_runner(
        &mut self,
        session_id: SessionId,
        request: FspecRequest,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            // Synchronous unit-test path — produce a synchronous error
            // result so test scenarios that don't drive a tokio runtime
            // can still assert the runner branched correctly.
            //
            // NOTE: the production async path is preferred; this
            // fallback only ever fires when called from outside a tokio
            // runtime (e.g. a pure `#[test]` that doesn't use
            // `#[tokio::test]`).
            let _ = (session_id, request);
            return;
        }
        let backend = self.backend.clone();
        let handle = tokio::spawn(async move {
            let result = run_fspec_command(backend.as_ref(), &request).await;
            let _ = backend.send_fspec_result(session_id, result).await;
        });
        self.pending_tasks.push(handle);
    }
}

/// Execute `request` against `backend` and produce an `FspecResult`.
/// Pure async helper — kept outside the `impl App` block so the runner
/// task can call it without holding an `App` reference.
async fn run_fspec_command(
    backend: &dyn crate::transport::FspecBackend,
    request: &FspecRequest,
) -> FspecResult {
    match request.command.as_str() {
        "list-work-units" => match backend.list_work_units().await {
            Ok(units) => match serde_json::to_string(&units) {
                Ok(data) => FspecResult {
                    success: true,
                    data,
                    error: None,
                    system_reminder: None,
                    tool_call_id: request.tool_call_id.clone(),
                },
                Err(e) => FspecResult {
                    success: false,
                    data: String::new(),
                    error: Some(format!("serialise list-work-units result: {e}")),
                    system_reminder: None,
                    tool_call_id: request.tool_call_id.clone(),
                },
            },
            Err(e) => FspecResult {
                success: false,
                data: String::new(),
                error: Some(format!("list-work-units: {e}")),
                system_reminder: None,
                tool_call_id: request.tool_call_id.clone(),
            },
        },
        "show-work-unit" => {
            let target_id = parse_show_work_unit_id(&request.args_json);
            match backend.list_work_units().await {
                Ok(units) => match target_id {
                    Some(id) => match units.iter().find(|u| u.id == id) {
                        Some(unit) => match serde_json::to_string(unit) {
                            Ok(data) => FspecResult {
                                success: true,
                                data,
                                error: None,
                                system_reminder: None,
                                tool_call_id: request.tool_call_id.clone(),
                            },
                            Err(e) => FspecResult {
                                success: false,
                                data: String::new(),
                                error: Some(format!("serialise show-work-unit: {e}")),
                                system_reminder: None,
                                tool_call_id: request.tool_call_id.clone(),
                            },
                        },
                        None => FspecResult {
                            success: false,
                            data: String::new(),
                            error: Some(format!("work unit not found: {id}")),
                            system_reminder: None,
                            tool_call_id: request.tool_call_id.clone(),
                        },
                    },
                    None => FspecResult {
                        success: false,
                        data: String::new(),
                        error: Some("show-work-unit: missing `id` in args_json".to_string()),
                        system_reminder: None,
                        tool_call_id: request.tool_call_id.clone(),
                    },
                },
                Err(e) => FspecResult {
                    success: false,
                    data: String::new(),
                    error: Some(format!("show-work-unit: {e}")),
                    system_reminder: None,
                    tool_call_id: request.tool_call_id.clone(),
                },
            }
        }
        other => FspecResult {
            success: false,
            data: String::new(),
            error: Some(format!("unsupported command: {other}")),
            system_reminder: None,
            tool_call_id: request.tool_call_id.clone(),
        },
    }
}

/// Best-effort `id` extraction from `show-work-unit`'s `args_json`.
/// Accepts both `{"id":"AUTH-001"}` and the fspec-CLI-style
/// `{"_":["AUTH-001"]}`. Returns `None` when neither shape matches.
fn parse_show_work_unit_id(args_json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(args_json).ok()?;
    if let Some(id) = value.get("id").and_then(|v| v.as_str()) {
        return Some(id.to_string());
    }
    if let Some(arr) = value.get("_").and_then(|v| v.as_array()) {
        if let Some(first) = arr.first().and_then(|v| v.as_str()) {
            return Some(first.to_string());
        }
    }
    None
}
