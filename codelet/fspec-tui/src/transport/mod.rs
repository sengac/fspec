//! Transport-agnostic backend surface for the fspec ratatui frontend.
//!
//! Feature: spec/features/fspec-tui-trait-surface.feature
//!
//! The [`FspecBackend`] trait is intentionally a near-1:1 of the tarpc
//! `FspecService` surface. It exists ONLY to let RPC-009/RPC-010 consumers
//! hold an `Arc<dyn FspecBackend>` and swap between the embedded and
//! WebSocket implementations without changing call sites — not to add
//! abstraction or transformation.
//!
//! Both implementations forward RPC method bodies as one-line delegates to
//! the underlying tarpc client (`self.client.<rpc>(context::current(),
//! ...).await`) and forward subscription methods as zero-cost passthroughs
//! of `broadcast::Receiver` returned by the inner transport. Envelope
//! framing for the WebSocket path stays entirely encapsulated in
//! `codelet-rpc-server`.

use anyhow::Result;
use async_trait::async_trait;
use codelet_rpc_types::{
    ApprovalChoice, BlocklistRuleInfo, CheckpointCounts, CompactionProgress, CompactionResult,
    FspecResult, HealthInfo, HistoryMatch, HitlRequest, HitlResponse, IncomingMessageInput,
    IsolatedSessionInfo,
    LogRecord, MergeOutcome, MergeStrategy, ModelEntry, ModelInfo, PauseState,
    ProviderCredentialInfo, ProviderCredentialInput, ProviderInfo, RegisteredLoop, ScheduledJob,
    SessionChangesSummary, SessionId, SessionInfo, SessionModel, SessionStatus, SessionTokens,
    SessionWorktreeInfo, StreamChunk, TestConnectionResult, ThinkingConfig, ThinkingLevel,
    TokenRestoreState, WorkUnitContext, WorkUnitInfo, WorkspaceInfo,
};
use thiserror::Error;
use tokio::sync::broadcast;

pub mod embedded;
pub mod websocket;

pub use embedded::EmbeddedFspecBackend;
pub use websocket::WebSocketFspecBackend;

/// RPC-011: structured error variants returned by `FspecBackend` impls.
///
/// `Disconnected` is the sentinel that the WebSocket transport's RPC
/// methods return once their internal client slot is `None` — i.e.
/// after the supervisor task has observed a WS drop and is currently
/// retrying. The App run loop renders the DisconnectDialog @
/// Priority::Critical in response, so user-visible behaviour is
/// always "a dialog, never a panic or hang".
#[derive(Debug, Error)]
pub enum BackendError {
    /// The underlying transport has lost its connection. RPC methods
    /// return this variant rather than panicking or hanging until the
    /// supervisor task reconnects.
    #[error("backend disconnected")]
    Disconnected,
}

/// Transport-agnostic surface holding both the embedded and WebSocket
/// fspec backends behind a single `Arc<dyn FspecBackend>`.
///
/// Method semantics are identical to the underlying tarpc
/// `FspecService` surface — the trait only exists to enable
/// transport-agnostic consumers in RPC-009 (real list view + REPL) and
/// RPC-010 (binary entry points).
#[async_trait]
pub trait FspecBackend: Send + Sync {
    /// List all known work units. Mirrors `FspecService::list_work_units`.
    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>>;

    /// List all known sessions. Mirrors `FspecService::list_sessions`.
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>>;

    /// Create a new session with an optional role overlay.
    async fn create_session(&self, role: Option<String>) -> Result<SessionId>;

    /// Append user input to the session with the given id.
    async fn send_input(&self, id: SessionId, text: String) -> Result<()>;

    /// Interrupt an in-flight session generation.
    async fn interrupt(&self, id: SessionId) -> Result<()>;

    /// Subscribe to broadcasted work-units snapshots (RPC-006). Each call
    /// returns a fresh receiver; senders fan out to all live receivers.
    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>>;

    /// Subscribe to broadcasted session stream chunks (RPC-007).
    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)>;

    /// Subscribe to broadcasted log records (RPC-007).
    fn logs_rx(&self) -> broadcast::Receiver<LogRecord>;

    /// RPC-011: return a live snapshot of the daemon's runtime health.
    /// Embedded backends short-circuit and read `ServerStats` directly;
    /// the WebSocket backend routes through tarpc `FspecService::health`.
    async fn health(&self) -> Result<HealthInfo>;

    /// RPC-015: return manual + auto checkpoint counts aggregated across
    /// every work unit in the workspace. Both transports delegate to
    /// the shared `FspecService::checkpoint_counts` RPC method which
    /// in turn calls `codelet_git::ghost_commit::count_checkpoints`.
    async fn checkpoint_counts(&self) -> Result<CheckpointCounts>;

    /// RPC-017: move the work unit with `id` one position UP in its
    /// current `states[<column>]` array in `spec/work-units.json`.
    /// No-op at the top boundary. Returns `Err` when the unit lives
    /// in the done column, when no cwd is attached to the shared
    /// service, or on I/O / data-integrity failure.
    ///
    /// Both transports forward to the shared `FspecService` RPC
    /// method, which delegates to
    /// `codelet_core::work_units_write::move_work_unit`. After
    /// persistence the workspace's `WorkUnitsWatcher` fires a fresh
    /// snapshot — the App's existing subscriber task converts that
    /// into `Action::WorkUnitsLoaded` and re-seeds the BoardStore,
    /// keeping the focused-column selection on the moved unit (via
    /// RPC-016's auto-scroll math).
    async fn move_work_unit_up(&self, id: String) -> Result<()>;

    /// RPC-017: mirror of [`move_work_unit_up`] for the DOWN direction.
    async fn move_work_unit_down(&self, id: String) -> Result<()>;

    /// RPC-018: return the display + capability metadata for the model
    /// currently bound to `session_id`. Both transports delegate to
    /// `FspecService::get_model_info`; the AgentView's SessionHeader
    /// reads the response via `Action::ModelInfoLoaded`.
    async fn get_model_info(&self, session_id: SessionId) -> Result<ModelInfo>;

    /// RPC-018: return the per-session thinking/reasoning level.
    /// Both transports delegate to `FspecService::get_thinking_level`.
    async fn get_thinking_level(&self, session_id: SessionId) -> Result<ThinkingLevel>;

    /// RPC-018: return the workspace snapshot (cwd + optional git
    /// branch) for the workspace this shared service was constructed
    /// against. Both transports delegate to
    /// `FspecService::get_workspace_info` which in turn reads
    /// `codelet_git::status::get_current_branch(cwd)`.
    async fn get_workspace_info(&self) -> Result<WorkspaceInfo>;

    /// RPC-020: search the workspace for files whose path matches the
    /// case-insensitive substring `prefix`. Returns at most `limit`
    /// paths sorted by modification time desc. Both transports delegate
    /// to `FspecService::search_files` which in turn calls
    /// `codelet_core::file_search::search(cwd, prefix, limit)`. Returns
    /// an empty Vec when no cwd is attached to the shared service or
    /// when no files match.
    async fn search_files(&self, prefix: String, limit: u32) -> Result<Vec<String>>;

    /// RPC-025: append a submitted input to the session's command
    /// history. Both transports forward to
    /// `FspecService::persistence_add_history`. Fire-and-forget at the
    /// App dispatch layer; the underlying tarpc call still returns
    /// Result so transport-level failures can be logged.
    async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()>;

    /// RPC-025: return the most recent `limit` history entries for the
    /// supplied session, newest-first. Used by App::dispatch to
    /// snapshot the per-session history before walking with Shift+↑.
    async fn persistence_get_history(&self, session: SessionId, limit: u32) -> Result<Vec<String>>;

    /// RPC-025: case-insensitive substring search across the full
    /// history JSONL. Returns `HistoryMatch` values with an
    /// RFC3339-formatted timestamp so the @search popup can render
    /// "<text>  <relative time>" lines without a chrono dep.
    async fn persistence_search_history(&self, query: String) -> Result<Vec<HistoryMatch>>;

    /// RPC-026: delete an on-disk session manifest by id. Both
    /// transports forward to `FspecService::persistence_delete_session`
    /// which in turn calls `codelet_core::persistence::delete_session`.
    /// Idempotent — deleting an unknown id silently succeeds.
    async fn persistence_delete_session(&self, id: SessionId) -> Result<()>;

    /// RPC-022: list providers and their models. Both transports
    /// delegate to `FspecService::list_providers` which in turn
    /// delegates to the optional `SessionManagerHandle`. Returns
    /// an empty Vec when no session manager is attached.
    async fn list_providers(&self) -> Result<Vec<ProviderInfo>>;

    /// RPC-022: set the model bound to a session. Both transports
    /// delegate to `FspecService::set_session_model`. Returns
    /// `Ok(())` (silent no-op) when no session manager is attached;
    /// `Err` when the underlying handle reports a failure.
    async fn set_session_model(
        &self,
        session_id: SessionId,
        provider_id: String,
        model_id: String,
    ) -> Result<()>;

    /// RPC-022: set the per-session thinking/reasoning level.
    /// Mirrors `set_session_model` in shape.
    async fn set_thinking_level(
        &self,
        session_id: SessionId,
        level: ThinkingLevel,
    ) -> Result<()>;

    /// RPC-027: set the per-user DEFAULT thinking/reasoning level.
    /// Sister of `set_thinking_level`; persists the level so subsequent
    /// sessions inherit it. Default impl is `Ok(())` (silent no-op)
    /// so embedded transports without a session manager attached
    /// compile unchanged.
    async fn set_thinking_level_default(
        &self,
        session_id: SessionId,
        level: ThinkingLevel,
    ) -> Result<()> {
        let _ = (session_id, level);
        Ok(())
    }


    /// RPC-022: read the session's current role overlay text. Both
    /// transports delegate to `FspecService::get_session_role`.
    /// Returns `Ok(None)` when no role is active OR when no session
    /// manager is attached.
    async fn get_session_role(&self, session_id: SessionId) -> Result<Option<String>>;

    /// RPC-022: set or clear the session's role overlay. Passing
    /// `None` clears.
    async fn set_session_role(
        &self,
        session_id: SessionId,
        role: Option<String>,
    ) -> Result<()>;

    /// RPC-011 rule [4]: trigger the transport's manual-reconnect signal
    /// (resets the backoff schedule + cancels any in-flight backoff
    /// sleep). Wired to the App's `r`-press handler from the
    /// DisconnectDialog so pressing `r` while disconnected immediately
    /// attempts reconnect rather than waiting for the next backoff tick.
    ///
    /// Default impl is a no-op so embedded and other transports without
    /// a reconnect supervisor (where the call has no meaning) don't need
    /// to override.
    fn request_manual_reconnect(&self) {}

    // ========================================================================
    // RPC-037: Widened FspecBackend surface for AgentView parity. Every
    // method below has a peer on `FspecService` (codelet/rpc); both
    // backends delegate as one-line wrappers. WebSocket variants follow the
    // existing `client.read().await + BackendError::Disconnected` guard
    // pattern established by the earlier RPC cards.
    // ========================================================================

    /// RPC-037: send user input with optional thinking config.
    /// Default delegates to `send_input` when `thinking` is `None`, and
    /// otherwise to the same delegate (test mocks override only the
    /// fields they care about).
    async fn send_input_with_thinking(
        &self,
        session_id: SessionId,
        text: String,
        _thinking: Option<ThinkingConfig>,
    ) -> Result<()> {
        self.send_input(session_id, text).await
    }

    /// RPC-037: per-session input/output token totals.
    async fn get_session_tokens(&self, _session_id: SessionId) -> Result<SessionTokens> {
        Ok(SessionTokens {
            input_tokens: 0,
            output_tokens: 0,
        })
    }

    /// RPC-037: per-session model binding.
    async fn get_session_model(&self, _session_id: SessionId) -> Result<SessionModel> {
        Ok(SessionModel {
            provider_id: String::new(),
            model_id: String::new(),
            context_window: 0,
            max_output_tokens: 0,
            compaction_threshold: 0,
        })
    }

    /// RPC-037: in-flight compaction progress.
    async fn get_compaction_progress(
        &self,
        _session_id: SessionId,
    ) -> Result<Option<CompactionProgress>> {
        Ok(None)
    }

    /// RPC-037: replay-buffer of recent stream chunks for a session.
    async fn get_buffered_output(
        &self,
        _session_id: SessionId,
        _limit: u32,
    ) -> Result<Vec<StreamChunk>> {
        Ok(Vec::new())
    }

    /// RPC-037: clear session history.
    async fn clear_history(&self, _session_id: SessionId) -> Result<()> {
        Ok(())
    }

    /// RPC-037: compact session history.
    async fn compact_session(&self, _session_id: SessionId) -> Result<CompactionResult> {
        Ok(CompactionResult {
            original_tokens: 0,
            compacted_tokens: 0,
            compression_ratio: 0.0,
            turns_summarized: 0,
            turns_kept: 0,
        })
    }

    /// RPC-037: restore session messages from raw JSONL envelopes.
    async fn restore_session_messages(
        &self,
        _session_id: SessionId,
        _envelopes: Vec<String>,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-037: restore cumulative-billed counters and cache totals.
    async fn restore_session_token_state(
        &self,
        _session_id: SessionId,
        _state: TokenRestoreState,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-049: durable-restore aggregate used by the TUI `/resume`
    /// flow. Default returns Ok(()) so embedded and other transports
    /// without a real session manager attached don't need to override.
    async fn resume_session(&self, _session_id: SessionId) -> Result<()> {
        Ok(())
    }

    /// RPC-037: read the work-unit context bound to a session.
    async fn get_work_unit_context(
        &self,
        _session_id: SessionId,
    ) -> Result<Option<WorkUnitContext>> {
        Ok(None)
    }

    /// RPC-037: bind (or detach) a work unit on a session.
    async fn set_work_unit_context(
        &self,
        _session_id: SessionId,
        _context: Option<WorkUnitContext>,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-037: read the per-session pending input draft.
    async fn get_pending_input(&self, _session_id: SessionId) -> Result<Option<String>> {
        Ok(None)
    }

    /// RPC-037: write the per-session pending input draft.
    async fn set_pending_input(
        &self,
        _session_id: SessionId,
        _text: Option<String>,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-037: set the active session for the application.
    async fn set_active_session(&self, _session_id: SessionId) -> Result<()> {
        Ok(())
    }

    /// RPC-037: clear the active session.
    async fn clear_active_session(&self) -> Result<()> {
        Ok(())
    }

    /// RPC-037: read the active session, if any.
    async fn get_active_session(&self) -> Result<Option<SessionId>> {
        Ok(None)
    }

    /// RPC-037: effective cwd for a session (worktree-aware).
    async fn get_effective_cwd(&self, _session_id: SessionId) -> Result<String> {
        Ok(String::new())
    }

    /// RPC-037: list supervisor session ids for a subordinate.
    async fn get_supervisors(&self, _session_id: SessionId) -> Result<Vec<SessionId>> {
        Ok(Vec::new())
    }

    /// RPC-061: register `supervisor_id` as a supervisor of
    /// `subordinate_id`. Default is `Ok(())` so mock backends that
    /// don't care about the supervisor surface compile unchanged.
    async fn add_supervisor(
        &self,
        _subordinate_id: SessionId,
        _supervisor_id: SessionId,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-061: remove every link in which `supervisor_id` is the
    /// supervisor.
    async fn remove_supervisor(&self, _supervisor_id: SessionId) -> Result<()> {
        Ok(())
    }

    /// RPC-061: first subordinate of the supervisor, or None.
    async fn get_subordinate(
        &self,
        _supervisor_id: SessionId,
    ) -> Result<Option<SessionId>> {
        Ok(None)
    }

    /// RPC-061: every subordinate of the supervisor.
    async fn get_subordinates(
        &self,
        _supervisor_id: SessionId,
    ) -> Result<Vec<SessionId>> {
        Ok(Vec::new())
    }

    /// RPC-061: queue a supervisor message onto a subordinate session.
    async fn receive_incoming_message(
        &self,
        _subordinate_id: SessionId,
        _message: IncomingMessageInput,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-037: debug-capture toggle reader.
    async fn get_debug_enabled(&self, _session_id: SessionId) -> Result<bool> {
        Ok(false)
    }

    /// RPC-037: debug-capture toggle writer.
    async fn set_debug_enabled(&self, _session_id: SessionId, _enabled: bool) -> Result<()> {
        Ok(())
    }

    /// RPC-037: toggle debug capture; returns the resolved path string.
    async fn toggle_debug(
        &self,
        _session_id: SessionId,
        _debug_dir: String,
    ) -> Result<String> {
        Ok(String::new())
    }

    /// RPC-055: set the global debug-capture directory used by the
    /// pre-session toggle path. Default is `Ok(())` so transports
    /// without a session manager attached compile unchanged.
    async fn set_debug_directory(&self, _path: String) -> Result<()> {
        Ok(())
    }

    /// RPC-037: resume a paused session.
    async fn pause_resume(&self, _session_id: SessionId) -> Result<()> {
        Ok(())
    }

    /// RPC-037: respond to a two-choice confirm pause.
    async fn pause_confirm(&self, _session_id: SessionId, _accept: bool) -> Result<()> {
        Ok(())
    }

    /// RPC-037: respond to a three-choice approval pause.
    async fn pause_triple(
        &self,
        _session_id: SessionId,
        _choice: ApprovalChoice,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-037: send a Human-In-The-Loop response.
    async fn send_hitl_response(
        &self,
        _session_id: SessionId,
        _response: HitlResponse,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-037: snapshot of the pause dialog state.
    async fn get_pause_state(&self, _session_id: SessionId) -> Result<Option<PauseState>> {
        Ok(None)
    }

    /// RPC-037: snapshot of the active HITL request, if any.
    async fn get_hitl_request(&self, _session_id: SessionId) -> Result<Option<HitlRequest>> {
        Ok(None)
    }

    /// RPC-037: round-trip an FspecCommandRequest reply.
    async fn send_fspec_result(
        &self,
        _session_id: SessionId,
        _result: FspecResult,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-037: create an isolated (worktree-backed) session.
    async fn create_isolated_session(
        &self,
        _role: Option<String>,
    ) -> Result<IsolatedSessionInfo> {
        Ok(IsolatedSessionInfo {
            session_id: SessionId::new(String::new()),
            worktree_path: String::new(),
            base_commit: String::new(),
        })
    }

    /// RPC-037: destroy a session, removing it from `list_sessions`.
    async fn destroy_session(&self, _session_id: SessionId) -> Result<()> {
        Ok(())
    }

    // ========================================================================
    // RPC-054: Provider credentials surface. Mirrors the trait additions on
    // `SessionManagerHandle` / `FspecService`. Both backends override these
    // with one-line forwarders to the tarpc client; the default impls here
    // keep test doubles and embedded transports without a session manager
    // attached compiling unchanged.
    // ========================================================================

    /// RPC-054: list provider credential summaries.
    async fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialInfo>> {
        Ok(Vec::new())
    }

    /// RPC-054: read a single provider's credential summary.
    async fn get_provider_credential(
        &self,
        _provider_id: String,
    ) -> Result<Option<ProviderCredentialInfo>> {
        Ok(None)
    }

    /// RPC-054: persist credentials for a provider.
    async fn set_provider_credentials(
        &self,
        _provider_id: String,
        _creds: ProviderCredentialInput,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-054: clear credentials for a provider.
    async fn delete_provider_credentials(&self, _provider_id: String) -> Result<()> {
        Ok(())
    }

    /// RPC-054: round-trip a connection test to the provider.
    async fn test_provider_connection(
        &self,
        _provider_id: String,
    ) -> Result<TestConnectionResult> {
        Ok(TestConnectionResult {
            success: true,
            error: None,
            latency_ms: 0,
        })
    }

    /// RPC-054: refresh the provider's cached model list.
    async fn refresh_models_cache(&self, _provider_id: String) -> Result<Vec<ModelEntry>> {
        Ok(Vec::new())
    }

    /// RPC-056: list every blocklist rule with its `source` provenance
    /// ("system" | "project"). Drives the `/blocklist` view.
    async fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>> {
        Ok(Vec::new())
    }

    /// RPC-057: merge a session's worktree changes back to base.
    async fn merge_session_worktree(
        &self,
        _session_id: SessionId,
        _strategy: MergeStrategy,
    ) -> Result<MergeOutcome> {
        Ok(MergeOutcome::default())
    }

    /// RPC-057: discard a session's worktree changes.
    async fn discard_session_worktree(&self, _session_id: SessionId) -> Result<()> {
        Ok(())
    }

    /// RPC-057: prune orphaned session worktrees.
    async fn prune_orphaned_worktrees(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// RPC-057: list every known session worktree.
    async fn list_session_worktrees(&self) -> Result<Vec<SessionWorktreeInfo>> {
        Ok(Vec::new())
    }

    /// RPC-057: inspect a session's pending change summary.
    async fn inspect_session_changes(
        &self,
        _session_id: SessionId,
    ) -> Result<SessionChangesSummary> {
        Ok(SessionChangesSummary::default())
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule.
    // ─────────────────────────────────────────────────────────────────

    /// RPC-058: persist a new scheduled job.
    async fn schedule_add(&self, _job: ScheduledJob) -> Result<ScheduledJob> {
        Ok(ScheduledJob::default())
    }

    /// RPC-058: list every persisted scheduled job.
    async fn schedule_list(&self) -> Result<Vec<ScheduledJob>> {
        Ok(Vec::new())
    }

    /// RPC-058: flip a job's status to `paused`.
    async fn schedule_pause(&self, _name: String) -> Result<ScheduledJob> {
        Ok(ScheduledJob::default())
    }

    /// RPC-058: flip a job's status to `active`.
    async fn schedule_resume(&self, _name: String) -> Result<ScheduledJob> {
        Ok(ScheduledJob::default())
    }

    /// RPC-058: remove a job by name.
    async fn schedule_remove(&self, _name: String) -> Result<()> {
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop trait defaults.
    // ─────────────────────────────────────────────────────────────────

    /// RPC-059: register a new session-scoped recurring prompt.
    async fn loop_add(
        &self,
        _session_id: SessionId,
        _interval_seconds: u32,
        _prompt: String,
    ) -> Result<RegisteredLoop> {
        Ok(RegisteredLoop::default())
    }

    /// RPC-059: cancel a registered loop by id.
    async fn loop_cancel(&self, _id: String) -> Result<bool> {
        Ok(false)
    }

    /// RPC-059: list every loop registered against a session.
    async fn loop_list(&self, _session_id: SessionId) -> Result<Vec<RegisteredLoop>> {
        Ok(Vec::new())
    }

    /// RPC-037: subscribe to push-driven (SessionId, SessionStatus)
    /// transitions. Mirror of `chunks_rx` / `logs_rx` — each call
    /// returns a fresh receiver bound to the per-transport broadcast.
    /// The default returns a closed receiver so transports that don't
    /// yet wire push-status compile unchanged and gracefully degrade.
    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        let (tx, rx) = broadcast::channel(1);
        drop(tx);
        rx
    }
}
