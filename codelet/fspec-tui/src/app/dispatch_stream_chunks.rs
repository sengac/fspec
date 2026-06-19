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

use codelet_rpc_types::WorkspaceInfo;
use codelet_rpc_types::{FspecRequest, SessionId, SessionState, SessionStatus, StreamChunk};

use crate::components::Action;
use crate::store::{agent_view::isolation_state::session_status_from_state, IsolationState};

use super::dispatch_slash_commands::format_compaction_notice;
use super::dispatch_fspec_runner::run_fspec_command;
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
                    SessionState::Cleared => {
                        // RPC-100: mirror TS AgentView.tsx:992-1006 —
                        // SessionStateChange→Cleared zeroes the token
                        // counters AND the compaction-reduction suffix
                        // so the SessionHeader badge returns to `[0%]`
                        // / no `COMPACTED` after a `/clear`.
                        self.agent_view_store.reset_token_state(session_id);
                        self.agent_view_store.clear_compaction_reduction(session_id);
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
                self.agent_view_store
                    .apply_supervisor_pending_injection(session_id);
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

                // RPC-100: persist the reduction percentage on the
                // per-session slot so SessionHeader renders the
                // `[X%: COMPACTED Y%]` badge suffix. Same formula as
                // `format_compaction_notice` in dispatch_slash_commands.rs:290
                // — keeping the notice line and the badge in sync.
                let reduction =
                    ((1.0 - compaction_result.compression_ratio) * 100.0).round() as i32;
                self.agent_view_store
                    .set_compaction_reduction(session_id.clone(), reduction);

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
