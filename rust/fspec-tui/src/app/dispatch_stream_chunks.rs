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
                let new_status = session_status_from_state(*state);
                let old = self
                    .agent_view_store
                    .session_status_for(session_id)
                    .copied();
                self.agent_view_store
                    .set_session_status(session_id.clone(), new_status);
                tracing::debug!(
                    session_id = %session_id,
                    old = ?old,
                    new = ?new_status,
                    "[compaction-status] TUI store updated (SessionStateChange chunk)"
                );
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
                // RPC-045 + WT-002: collapse onto the single-slot `workspace`
                // field. is_git_repo=false → blank branch; is_git_repo=true
                // + branch=None is a DETACHED-HEAD worktree → "(detached)".
                let git_branch = match (*is_git_repo, branch.clone()) {
                    (true, Some(b)) => Some(b),
                    (true, None) => Some("(detached)".to_string()),
                    (false, _) => None,
                };
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
                // CMPCT-049: the arm body (clear progress, persist the
                // reduction badge, arm the auto-hide timer, emit the
                // single user-facing notice — RPC-421 / RPC-417 /
                // RPC-100) is factored into
                // `dispatch_compaction_complete.rs` so this file stays
                // under the 300-LoC ceiling.
                self.apply_compaction_complete(session_id, compaction_result);
            }
            StreamChunk::ContinueStateUpdate { continue_state } => {
                // CONT-007: fold the live counter snapshot into the chrome
                // cache. The `(enabled, budget)` pair keeps the existing
                // slash-dispatch slot coherent; the live slot carries the
                // real nudge counter + display budget for the footer.
                self.agent_view_store.set_continue_state(
                    session_id.clone(),
                    continue_state.enabled,
                    continue_state.budget,
                );
                self.agent_view_store.set_continue_live(
                    session_id.clone(),
                    crate::store::agent_view::chrome_state::ContinueLiveState {
                        nudges_used: continue_state.nudges_used,
                        effective_budget: continue_state.effective_budget,
                        goal_active: continue_state.goal_active,
                        done_rejections: continue_state.done_rejections,
                    },
                );
                // CONT-008: the engine cleared a satisfied goal — drop the
                // cached goal so the footer 🎯 disappears and bare /goal
                // reports "no goal set". Keyed off the DEDICATED flag,
                // never off an incidental goal_active:false.
                if continue_state.goal_cleared {
                    self.agent_view_store
                        .set_goal_state(session_id.clone(), None);
                }
            }
            // BUG-171: exec-stdin push chunks — the sessions layer emits
            // these from `set_exec_stdin_request` on slot transitions
            // (no status flip, so the Paused-chunk probe never fires).
            // Route them into the existing exec-stdin reducer: request →
            // `ExecStdinPromptFetched` (reuses the HITL-guarded reducer +
            // the HITL > exec-stdin > pause > composer precedence chain),
            // cleared → `ExecStdinDismissed` (slot-only clear; nothing is
            // sent, cancelled, or killed). Re-dispatch via action_tx so
            // the reducers run through the normal dispatch path.
            StreamChunk::ExecStdinRequest { request } => {
                let _ = self.action_tx.send(Action::ExecStdinPromptFetched {
                    agent_session: session_id.clone(),
                    request: request.clone(),
                });
            }
            StreamChunk::ExecStdinRequestCleared => {
                let _ = self.action_tx.send(Action::ExecStdinDismissed {
                    agent_session: session_id.clone(),
                });
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
        let old = self
            .agent_view_store
            .session_status_for(&session_id)
            .copied();
        self.agent_view_store
            .set_session_status(session_id.clone(), status);
        tracing::debug!(
            session_id = %session_id,
            old = ?old,
            new = ?status,
            "[compaction-status] TUI store updated (push channel)"
        );
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
