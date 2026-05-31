//! Session manager handle abstraction (RPC-007).
//!
//! Defines the [`SessionManagerHandle`] trait that the dual-transport RPC
//! layer (codelet/rpc) consumes via dependency injection. The concrete
//! 8,649 LOC `SessionManager` implementation lives in `codelet/napi/src/
//! session_manager.rs` — codelet/core defines only the trait surface so
//! that codelet/rpc never imports codelet/napi (rpc → napi forbidden;
//! rpc → core permitted; napi → core permitted).
//!
//! ## NAPI shared contract invariant
//!
//! The trait surface here, plus the five new types in codelet/rpc-types
//! (SessionId, SessionInfo, SessionStatus, StreamChunk, LogRecord), are
//! the contract that all three frontends consume identically:
//!   * the JS frontend via codelet/napi's #[napi] re-exports,
//!   * the built-in ratatui frontend via EmbeddedTransport calling
//!     Arc<dyn SessionManagerHandle> directly,
//!   * the WebSocket frontend via tarpc-generated FspecServiceClient
//!     over bincode-encoded Envelope.
//!
//! ## Test stub
//!
//! [`StubSessionManagerHandle`] is a minimal in-memory implementation
//! used by integration tests so they can exercise the full RPC + push
//! channel surface without dragging in the real SessionManager and its
//! dependency tree (codelet-cli, codelet-git, codelet-tools,
//! codelet-providers, OAuth, persistence, ghost commits, etc.).

use codelet_rpc_types::{
    ApprovalChoice, BlocklistRuleInfo, CompactionProgress, CompactionResult, FspecResult,
    HitlRequest, HitlResponse, IncomingMessageInput,
    IsolatedSessionInfo, LogRecord, MergeOutcome, MergeStatus, MergeStrategy, ModelInfo,
    PauseState, ProviderInfo,
    ProviderCredentialInfo, ProviderCredentialInput, RegisteredLoop, ScheduledJob, SessionChangesSummary,
    SessionId, SessionInfo, SessionModel, SessionState, SessionStatus, SessionTokens,
    SessionWorktreeInfo, StreamChunk, TestConnectionResult, ThinkingConfig, ThinkingLevel,
    TokenRestoreState, WorkUnitContext,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use tokio::sync::broadcast;

/// RPC-037: broadcast capacity for the per-process status_changes
/// channel. Matches the chunks/logs channel capacities in the stub so
/// a slow subscriber behaves consistently across all three streams.
pub const DEFAULT_STATUS_CHANGES_CAPACITY: usize = 256;

fn status_str(status: SessionStatus) -> &'static str {
    match status {
        SessionStatus::Idle => "idle",
        SessionStatus::Running => "running",
        SessionStatus::Paused => "paused",
        SessionStatus::Compacting => "compacting",
        SessionStatus::Interrupted => "interrupted",
        SessionStatus::Cleared => "cleared",
    }
}

/// Trait implemented by the concrete `SessionManager` in codelet/napi
/// and by [`StubSessionManagerHandle`] in tests.
///
/// All methods are synchronous and non-blocking — the actual session
/// machinery (LLM streams, tool execution, compaction) is owned by
/// the implementation and runs on the host runtime.
pub trait SessionManagerHandle: Send + Sync + 'static {
    /// Return public metadata for every session currently tracked.
    fn list_sessions(&self) -> Vec<SessionInfo>;

    /// Create a new session with an optional role. Returns the
    /// freshly-minted [`SessionId`].
    fn create_session(&self, role: Option<String>) -> SessionId;

    /// Send user input to a session. Returns immediately — the actual
    /// streaming response arrives on the chunks broadcast subscribed
    /// via [`SessionManagerHandle::chunks_rx`].
    fn send_input(&self, session_id: &SessionId, text: String);

    /// Interrupt a running session. Returns immediately. A subsequent
    /// `StreamChunk::Interrupted` will arrive on the chunks broadcast.
    fn interrupt(&self, session_id: &SessionId);

    /// Return the current lifecycle state of a session.
    fn get_session_status(&self, session_id: &SessionId) -> SessionStatus;

    /// Subscribe to the per-process StreamChunk broadcast. Every send_input
    /// pushes its streaming output here as `(SessionId, StreamChunk)` tuples.
    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)>;

    /// Subscribe to the per-process LogRecord broadcast. The host's
    /// tracing::Layer pushes structured events here.
    fn logs_rx(&self) -> broadcast::Receiver<LogRecord>;

    /// Return a cloneable handle to the chunks broadcast sender so the
    /// host's tracing layer / NAPI ThreadsafeFunction co-listener can
    /// publish or co-subscribe directly.
    fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)>;

    /// Return a cloneable handle to the logs broadcast sender so the
    /// host's tracing::Layer can push records onto the same broadcast
    /// that other listeners observe via `logs_rx`.
    fn logs_tx(&self) -> broadcast::Sender<LogRecord>;

    /// RPC-018: return the display + capability metadata for the model
    /// currently bound to `session_id`. Default implementation returns
    /// `ModelInfo::default()` (empty display name, all-false caps,
    /// context_window = 0) so handles that don't yet know how to
    /// resolve provider/model state — including `StubSessionManagerHandle`
    /// — compile without per-test wiring. The concrete codelet/napi
    /// `SessionManager` overrides this in RPC-022 once the ModelSelector
    /// modal dialog needs live data.
    fn get_model_info(&self, session_id: &SessionId) -> ModelInfo {
        let _ = session_id;
        ModelInfo::default()
    }

    /// RPC-018: return the per-session thinking/reasoning level.
    /// Mirrors `get_model_info` in shape — default returns
    /// `ThinkingLevel::Off`; the codelet/napi `SessionManager` overrides
    /// this in RPC-022 (ThinkingLevel modal dialog).
    fn get_thinking_level(&self, session_id: &SessionId) -> ThinkingLevel {
        let _ = session_id;
        ThinkingLevel::Off
    }

    /// RPC-022: return the available provider/model registry for the
    /// /model modal dialog. Default returns `Vec::new()` so handles
    /// that have not yet wired the model registry — including the
    /// `StubSessionManagerHandle` used by integration tests — compile
    /// without per-test wiring. The concrete codelet/napi
    /// `SessionManager` overrides this to read the cached
    /// `ModelRegistry` and map each provider/model into the
    /// transport-portable `ProviderInfo` / `ModelEntry` shape.
    fn list_providers(&self) -> Vec<ProviderInfo> {
        Vec::new()
    }

    /// RPC-022: set the model bound to a session. Default returns
    /// `Ok(())` (silent no-op) so handles that have not yet wired
    /// model selection — including the stub used by tests — compile
    /// without per-test wiring. The codelet/napi `SessionManager`
    /// overrides this to delegate to the existing
    /// `session_set_model`-style flow (model_string parsing +
    /// `ProviderManager::select_model`).
    fn set_model(
        &self,
        session_id: &SessionId,
        provider_id: &str,
        model_id: &str,
    ) -> Result<(), String> {
        let _ = (session_id, provider_id, model_id);
        Ok(())
    }

    /// RPC-022: set the base thinking/reasoning level for a session.
    /// Default returns `Ok(())` (silent no-op). The codelet/napi
    /// override forwards to the existing
    /// `session_set_base_thinking_level` flow.
    fn set_thinking_level(
        &self,
        session_id: &SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        let _ = (session_id, level);
        Ok(())
    }

    /// RPC-027: set the PER-USER DEFAULT thinking/reasoning level.
    /// Unlike `set_thinking_level` (which is session-scoped), this
    /// persists the level so new sessions inherit it. Default returns
    /// `Ok(())` (silent no-op). The codelet/napi override forwards
    /// to the future `session_set_default_thinking_level` flow.
    fn set_thinking_level_default(
        &self,
        session_id: &SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        let _ = (session_id, level);
        Ok(())
    }

    /// RPC-022: read the session's current role overlay text. Default
    /// returns `None` so handles that have not yet wired role state —
    /// including the stub — compile without per-test wiring. The
    /// codelet/napi override forwards to the existing
    /// `session_get_role` flow (which returns
    /// `Option<SupervisorRoleInfo>` on the JS surface).
    fn get_role(&self, session_id: &SessionId) -> Option<String> {
        let _ = session_id;
        None
    }

    /// RPC-022: set or clear the session's role overlay. Passing
    /// `None` clears. Default returns `Ok(())` (silent no-op). The
    /// codelet/napi override forwards to the existing
    /// `session_set_role` / `session.clear_role` flow.
    fn set_role(
        &self,
        session_id: &SessionId,
        role: Option<String>,
    ) -> Result<(), String> {
        let _ = (session_id, role);
        Ok(())
    }

    // ========================================================================
    // RPC-037: Widened surface for AgentView parity.
    // Every new method declares a default body so existing handles compile
    // unchanged. The codelet/napi `SessionManager` will override these in
    // Phase 4 (RPC-042) when the agent loop is extracted into codelet-sessions.
    // ========================================================================

    /// RPC-037: send user input with an optional provider-specific
    /// thinking config. The default delegates to `send_input` so
    /// implementers only need to override one method.
    fn send_input_with_thinking(
        &self,
        session_id: &SessionId,
        text: String,
        thinking: Option<ThinkingConfig>,
    ) {
        let _ = thinking;
        self.send_input(session_id, text);
    }

    /// RPC-037: per-session input + output token totals.
    fn get_session_tokens(&self, session_id: &SessionId) -> SessionTokens {
        let _ = session_id;
        SessionTokens {
            input_tokens: 0,
            output_tokens: 0,
        }
    }

    /// RPC-037: per-session model binding (provider + model + limits).
    fn get_session_model(&self, session_id: &SessionId) -> SessionModel {
        let _ = session_id;
        SessionModel {
            provider_id: String::new(),
            model_id: String::new(),
            context_window: 0,
            max_output_tokens: 0,
            compaction_threshold: 0,
        }
    }

    /// RPC-037: in-flight compaction progress, if any.
    fn get_compaction_progress(&self, session_id: &SessionId) -> Option<CompactionProgress> {
        let _ = session_id;
        None
    }

    /// RPC-037: replay-buffer of recent stream chunks for a session.
    fn get_buffered_output(&self, session_id: &SessionId, limit: u32) -> Vec<StreamChunk> {
        let _ = (session_id, limit);
        Vec::new()
    }

    /// RPC-037: clear session history.
    fn clear_history(&self, session_id: &SessionId) -> Result<(), String> {
        let _ = session_id;
        Ok(())
    }

    /// RPC-037: compact session history; returns the canned result so
    /// the AgentView can display compression statistics.
    fn compact_session(&self, session_id: &SessionId) -> Result<CompactionResult, String> {
        let _ = session_id;
        Ok(CompactionResult {
            original_tokens: 0,
            compacted_tokens: 0,
            compression_ratio: 0.0,
            turns_summarized: 0,
            turns_kept: 0,
        })
    }

    /// RPC-037: restore session messages from raw JSONL envelopes (used by `/resume`).
    fn restore_session_messages(
        &self,
        session_id: &SessionId,
        envelopes: Vec<String>,
    ) -> Result<(), String> {
        let _ = (session_id, envelopes);
        Ok(())
    }

    /// RPC-037: restore the cumulative-billed counters + cache totals (used by `/resume`).
    fn restore_session_token_state(
        &self,
        session_id: &SessionId,
        state: TokenRestoreState,
    ) -> Result<(), String> {
        let _ = (session_id, state);
        Ok(())
    }

    /// RPC-049: durable-restore round-trip used by `/resume`. Loads the
    /// on-disk session manifest + envelopes via
    /// `codelet_core::persistence`, builds a `TokenRestoreState` from
    /// the manifest's `token_usage`, then calls
    /// [`Self::restore_session_messages`] +
    /// [`Self::restore_session_token_state`].
    ///
    /// Errors at any step propagate as `Result<(), String>` so the
    /// caller (TUI `Action::AttachToSession` handler) can surface them
    /// via `Action::EmitSessionNotice`.
    fn resume_session(&self, session_id: &SessionId) -> Result<(), String> {
        let uuid = uuid::Uuid::parse_str(&session_id.value)
            .map_err(|e| format!("invalid session id: {e}"))?;
        let manifest = crate::persistence::load_session(uuid)?;
        let envelopes = crate::persistence::get_session_message_envelopes(uuid)?;
        let state = TokenRestoreState {
            current_context: manifest.token_usage.current_context_tokens as i64,
            cumulative_billed_output: manifest.token_usage.cumulative_billed_output as i64,
            cache_read: manifest.token_usage.cache_read_tokens as i64,
            cache_creation: manifest.token_usage.cache_creation_tokens as i64,
            cumulative_billed_input: manifest.token_usage.cumulative_billed_input as i64,
            cumulative_billed_output_second: manifest.token_usage.cumulative_billed_output as i64,
        };
        self.restore_session_messages(session_id, envelopes)?;
        self.restore_session_token_state(session_id, state)?;
        Ok(())
    }

    /// RPC-037: read the work-unit context bound to a session.
    fn get_work_unit_context(&self, session_id: &SessionId) -> Option<WorkUnitContext> {
        let _ = session_id;
        None
    }

    /// RPC-037: bind (or detach) a work unit on a session.
    fn set_work_unit_context(
        &self,
        session_id: &SessionId,
        ctx: Option<WorkUnitContext>,
    ) -> Result<(), String> {
        let _ = (session_id, ctx);
        Ok(())
    }

    /// RPC-037: read the per-session pending input draft.
    fn get_pending_input(&self, session_id: &SessionId) -> Option<String> {
        let _ = session_id;
        None
    }

    /// RPC-037: write the per-session pending input draft.
    fn set_pending_input(&self, session_id: &SessionId, text: Option<String>) {
        let _ = (session_id, text);
    }

    /// RPC-037: set the active session for the application.
    fn set_active_session(&self, session_id: &SessionId) {
        let _ = session_id;
    }

    /// RPC-037: clear the active session.
    fn clear_active_session(&self) {}

    /// RPC-037: read the active session, if any.
    fn get_active_session(&self) -> Option<SessionId> {
        None
    }

    /// RPC-037: effective cwd for a session (worktree-aware).
    fn get_effective_cwd(&self, session_id: &SessionId) -> PathBuf {
        let _ = session_id;
        PathBuf::new()
    }

    /// RPC-037: list supervisor session ids for a subordinate.
    fn get_supervisors(&self, session_id: &SessionId) -> Vec<SessionId> {
        let _ = session_id;
        Vec::new()
    }

    /// RPC-061: register `supervisor_id` as a supervisor of
    /// `subordinate_id`. Default is `Ok(())` so handles that don't
    /// track supervisor links (the embedded stub used by some unit
    /// tests, future read-only handles, etc.) compile unchanged.
    /// Production handles (`StubSessionManagerHandle` +
    /// `codelet_sessions::SessionManager`) override this to delegate
    /// into `ChainOfCommand::add_supervisor` and surface its
    /// "circular supervision not allowed" / "subordinate already
    /// registered under this supervisor" error strings verbatim.
    fn add_supervisor(
        &self,
        subordinate_id: &SessionId,
        supervisor_id: &SessionId,
    ) -> Result<(), String> {
        let _ = (subordinate_id, supervisor_id);
        Ok(())
    }

    /// RPC-061: remove every link in which `supervisor_id` is the
    /// supervisor. Default is `Ok(())` so handles without a chain of
    /// command compile unchanged.
    fn remove_supervisor(&self, supervisor_id: &SessionId) -> Result<(), String> {
        let _ = supervisor_id;
        Ok(())
    }

    /// RPC-061: return the first subordinate registered to a
    /// supervisor. `None` when no subordinate is bound. Backward-
    /// compatible accessor that mirrors `ChainOfCommand::get_subordinate`.
    fn get_subordinate(&self, supervisor_id: &SessionId) -> Option<SessionId> {
        let _ = supervisor_id;
        None
    }

    /// RPC-061: list every subordinate of a supervisor. Mirrors
    /// `ChainOfCommand::get_subordinates`.
    fn get_subordinates(&self, supervisor_id: &SessionId) -> Vec<SessionId> {
        let _ = supervisor_id;
        Vec::new()
    }

    /// RPC-061: queue an incoming supervisor message for a subordinate
    /// session. The production handle wraps this onto
    /// `BackgroundSession::receive_incoming_message`; the stub records
    /// the payload in `recorded_incoming_messages` so cross-transport
    /// parity tests can assert both transports reach the same in-memory
    /// state.
    fn receive_incoming_message(
        &self,
        subordinate_id: &SessionId,
        message: IncomingMessageInput,
    ) -> Result<(), String> {
        let _ = (subordinate_id, message);
        Ok(())
    }

    /// RPC-037: debug-capture toggle reader.
    fn get_debug_enabled(&self, session_id: &SessionId) -> bool {
        let _ = session_id;
        false
    }

    /// RPC-037: debug-capture toggle writer.
    fn set_debug_enabled(&self, session_id: &SessionId, enabled: bool) {
        let _ = (session_id, enabled);
    }

    /// RPC-037: toggle debug capture; returns the resolved path string.
    fn toggle_debug(
        &self,
        session_id: &SessionId,
        debug_dir: &str,
    ) -> Result<String, String> {
        let _ = (session_id, debug_dir);
        Ok(String::new())
    }

    /// RPC-055: set the global debug-capture directory used by the
    /// pre-session toggle path. Mirrors the NAPI
    /// `toggle_debug(Option<String>)` global helper. Default returns
    /// `Ok(())` so handles that don't yet wire a global manager — the
    /// stub used by integration tests, plus any future minimal handles
    /// — compile unchanged. The production `codelet-sessions`
    /// `SessionManager` overrides this to delegate into
    /// `codelet_common::debug_capture::DebugCaptureManager::set_debug_directory`.
    fn set_debug_directory(&self, path: PathBuf) -> Result<(), String> {
        let _ = path;
        Ok(())
    }

    /// RPC-037: resume a paused session.
    fn pause_resume(&self, session_id: &SessionId) -> Result<(), String> {
        let _ = session_id;
        Ok(())
    }

    /// RPC-037: respond to a two-choice confirm pause.
    fn pause_confirm(
        &self,
        session_id: &SessionId,
        accept: bool,
    ) -> Result<(), String> {
        let _ = (session_id, accept);
        Ok(())
    }

    /// RPC-037: respond to a three-choice (Approve / ApproveSession / Deny) pause.
    fn pause_triple(
        &self,
        session_id: &SessionId,
        choice: ApprovalChoice,
    ) -> Result<(), String> {
        let _ = (session_id, choice);
        Ok(())
    }

    /// RPC-037: send a Human-In-The-Loop response.
    fn send_hitl_response(
        &self,
        session_id: &SessionId,
        response: HitlResponse,
    ) -> Result<(), String> {
        let _ = (session_id, response);
        Ok(())
    }

    /// RPC-037: snapshot of the pause dialog state.
    fn get_pause_state(&self, session_id: &SessionId) -> Option<PauseState> {
        let _ = session_id;
        None
    }

    /// RPC-037: snapshot of the active HITL request, if any.
    fn get_hitl_request(&self, session_id: &SessionId) -> Option<HitlRequest> {
        let _ = session_id;
        None
    }

    /// RPC-037: round-trip an `FspecCommandRequest` reply.
    fn send_fspec_result(
        &self,
        session_id: &SessionId,
        result: FspecResult,
    ) -> Result<(), String> {
        let _ = (session_id, result);
        Ok(())
    }

    /// RPC-037: create an isolated (worktree-backed) session.
    fn create_isolated_session(
        &self,
        role: Option<String>,
    ) -> Result<IsolatedSessionInfo, String> {
        let _ = role;
        Err("create_isolated_session not implemented for this handle".to_string())
    }

    /// RPC-037: subscribe to (SessionId, SessionStatus) status pushes.
    /// Default returns a fresh broadcast whose sender is dropped
    /// immediately so subscribers observe `Closed` — handles that
    /// don't yet push status updates degrade to polling
    /// `get_session_status`.
    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        let (tx, rx) = broadcast::channel(DEFAULT_STATUS_CHANGES_CAPACITY);
        drop(tx);
        rx
    }

    /// RPC-037: cloneable handle to the status-changes broadcast.
    /// Default returns a fresh sender that nobody subscribes to.
    fn status_changes_tx(&self) -> broadcast::Sender<(SessionId, SessionStatus)> {
        let (tx, _rx) = broadcast::channel(DEFAULT_STATUS_CHANGES_CAPACITY);
        tx
    }

    /// RPC-037: destroy a session, removing it from `list_sessions`.
    fn destroy_session(&self, session_id: &SessionId) -> Result<(), String> {
        let _ = session_id;
        Ok(())
    }

    // ========================================================================
    // RPC-054: Provider credentials surface. Powers the new Rust ratatui
    // `ProviderSettingsView` (`/provider` slash command). Each method has a
    // default impl so existing handles compile unchanged; the production
    // `codelet-sessions` SessionManager overrides them by delegating to
    // `codelet-providers`, and `StubSessionManagerHandle` overrides them
    // with deterministic state for cross-transport parity tests.
    // ========================================================================

    /// RPC-054: list all known providers with their configured /
    /// credential-type / model-count metadata. Drives the left pane of
    /// `ProviderSettingsView`.
    fn list_provider_credentials(&self) -> Vec<ProviderCredentialInfo> {
        Vec::new()
    }

    /// RPC-054: return the credential summary for a single provider, or
    /// `None` when the provider id is unknown.
    fn get_provider_credential(&self, provider_id: &str) -> Option<ProviderCredentialInfo> {
        let _ = provider_id;
        None
    }

    /// RPC-054: persist credentials for a provider. Returns `Err`
    /// when the input is malformed (e.g. `kind: "api_key"` with no
    /// `api_key` value), when the provider id is unknown, or on I/O
    /// failure. The TUI view surfaces the error inline in its status
    /// area.
    fn set_provider_credentials(
        &self,
        provider_id: &str,
        creds: ProviderCredentialInput,
    ) -> Result<(), String> {
        let _ = (provider_id, creds);
        Ok(())
    }

    /// RPC-054: clear credentials for a provider. Idempotent on
    /// already-cleared rows.
    fn delete_provider_credentials(&self, provider_id: &str) -> Result<(), String> {
        let _ = provider_id;
        Ok(())
    }

    /// RPC-054: perform a network round-trip to the provider's base URL
    /// and return latency + success metadata. Used by the `t` key on a
    /// focused provider row.
    fn test_provider_connection(
        &self,
        provider_id: &str,
    ) -> Result<TestConnectionResult, String> {
        let _ = provider_id;
        Ok(TestConnectionResult {
            success: true,
            error: None,
            latency_ms: 0,
        })
    }

    /// RPC-054: refresh the provider's cached model list (no-op when
    /// the provider does not publish a dynamic catalog) and return the
    /// fresh `ModelEntry` list. Used by the `r` key on a focused
    /// provider row.
    fn refresh_models_cache(
        &self,
        provider_id: &str,
    ) -> Result<Vec<codelet_rpc_types::ModelEntry>, String> {
        let _ = provider_id;
        Ok(Vec::new())
    }

    // ========================================================================
    // RPC-056: Blocklist surface. Default impl returns an empty list so
    // existing handles compile unchanged. The production `codelet-sessions`
    // SessionManager overrides this by re-loading the system + project
    // blocklist configs separately via `codelet_tools::blocklist`.
    // `StubSessionManagerHandle` overrides this with a deterministic
    // in-memory snapshot + per-call counter for cross-transport parity
    // tests.
    // ========================================================================

    /// RPC-056: list every blocklist rule with its `source` provenance
    /// ("system" | "project"). Drives the left pane of
    /// `BlocklistView` in the Rust ratatui frontend. The TS frontend
    /// reaches the same data via the legacy `blocklistLoad(cwd)` NAPI
    /// call.
    fn blocklist_list(&self) -> Vec<BlocklistRuleInfo> {
        Vec::new()
    }

    // ========================================================================
    // RPC-057: Merge/worktree surface. Default impls return safe values so
    // existing handles compile unchanged. The production `codelet-sessions`
    // SessionManager overrides each by delegating to `codelet-git`.
    // `StubSessionManagerHandle` overrides them with per-call counters +
    // deterministic seeded payloads for cross-transport parity tests.
    // ========================================================================

    /// RPC-057: merge a session's worktree changes back to the base
    /// branch. The strategy is reserved for future evolution — the
    /// underlying codelet-git layer ignores it today.
    fn merge_session_worktree(
        &self,
        session_id: &SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeOutcome, String> {
        let _ = (session_id, strategy);
        Ok(MergeOutcome {
            status: MergeStatus::NoChanges,
            conflicts: Vec::new(),
            merge_commit: None,
        })
    }

    /// RPC-057: discard a session's worktree changes.
    fn discard_session_worktree(
        &self,
        session_id: &SessionId,
    ) -> Result<(), String> {
        let _ = session_id;
        Ok(())
    }

    /// RPC-057: prune any orphaned worktrees that no longer belong to a
    /// known session. Returns the session ids that were pruned.
    fn prune_orphaned_worktrees(&self) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }

    /// RPC-057: list every known session worktree with its base/head
    /// commits and a dirty heuristic.
    fn list_session_worktrees(&self) -> Vec<SessionWorktreeInfo> {
        Vec::new()
    }

    /// RPC-057: inspect a session's pending changes summary
    /// (files_changed / insertions / deletions / commits).
    fn inspect_session_changes(
        &self,
        session_id: &SessionId,
    ) -> Result<SessionChangesSummary, String> {
        let _ = session_id;
        Ok(SessionChangesSummary {
            files_changed: 0,
            insertions: 0,
            deletions: 0,
            commits: Vec::new(),
        })
    }

    // ========================================================================
    // RPC-058: /schedule surface. Default impls return safe values so existing
    // handles compile unchanged. The production `codelet-sessions`
    // SessionManager overrides each by delegating to
    // `codelet_core::scheduler::crud`. `StubSessionManagerHandle` overrides
    // them with per-call counters + deterministic seeded payloads for
    // cross-transport parity tests.
    // ========================================================================

    /// RPC-058: persist a new scheduled job to `spec/schedules.json`.
    fn schedule_add(&self, job: ScheduledJob) -> Result<ScheduledJob, String> {
        Ok(job)
    }

    /// RPC-058: list every persisted scheduled job.
    fn schedule_list(&self) -> Vec<ScheduledJob> {
        Vec::new()
    }

    /// RPC-058: flip the `status` of a job to `paused`.
    fn schedule_pause(&self, name: &str) -> Result<ScheduledJob, String> {
        let _ = name;
        Ok(ScheduledJob::default())
    }

    /// RPC-058: flip the `status` of a job to `active`.
    fn schedule_resume(&self, name: &str) -> Result<ScheduledJob, String> {
        let _ = name;
        Ok(ScheduledJob::default())
    }

    /// RPC-058: remove a job from `spec/schedules.json`.
    fn schedule_remove(&self, name: &str) -> Result<(), String> {
        let _ = name;
        Ok(())
    }

    // ========================================================================
    // RPC-059 — /loop subcommand surface. Each method has a safe default
    // (returns a blank `RegisteredLoop`, `false`, or an empty `Vec`) so
    // existing handle implementations compile unchanged. The production
    // impl lives in `codelet/sessions/src/handle_impl.rs` and routes
    // through the shared `codelet_core::loops::LoopStore` singleton.
    // `StubSessionManagerHandle` overrides them with per-call counters
    // + deterministic seeded payloads for cross-transport parity tests.
    // ========================================================================

    /// RPC-059: register a session-scoped recurring prompt on the
    /// shared `LoopStore` singleton.
    fn loop_add(
        &self,
        session_id: &SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop, String> {
        let _ = session_id;
        let _ = interval_seconds;
        let _ = prompt;
        Ok(RegisteredLoop::default())
    }

    /// RPC-059: cancel a registered loop by id. Returns `true` when a
    /// matching loop was removed.
    fn loop_cancel(&self, id: &str) -> Result<bool, String> {
        let _ = id;
        Ok(false)
    }

    /// RPC-059: list every loop registered against a session.
    fn loop_list(&self, session_id: &SessionId) -> Vec<RegisteredLoop> {
        let _ = session_id;
        Vec::new()
    }
}

// ============================================================================
// StubSessionManagerHandle — minimal in-memory implementation used by tests
// ============================================================================

/// Minimal in-memory implementation used by integration tests.
///
/// Holds an internal session table and emits a deterministic
/// `[StreamChunk::Text("hi back"), StreamChunk::Done]` sequence on
/// `send_input` regardless of input. Replaces the heavy
/// `SessionManager` for cross-transport tests so the tests don't
/// depend on the full provider/tool dependency tree.
pub struct StubSessionManagerHandle {
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    logs_tx: broadcast::Sender<LogRecord>,
    status_changes_tx: broadcast::Sender<(SessionId, SessionStatus)>,
    sessions: Arc<Mutex<Vec<SessionRecord>>>,
    next_id: AtomicU64,
    next_iso_id: AtomicU64,
    providers: Arc<Mutex<Vec<ProviderInfo>>>,
    // RPC-037 deterministic state seeds — keyed by SessionId.
    tokens: Arc<Mutex<HashMap<SessionId, SessionTokens>>>,
    models: Arc<Mutex<HashMap<SessionId, SessionModel>>>,
    work_unit_ctx: Arc<Mutex<HashMap<SessionId, WorkUnitContext>>>,
    pending_input: Arc<Mutex<HashMap<SessionId, String>>>,
    debug_enabled: Arc<Mutex<HashMap<SessionId, bool>>>,
    pause_state: Arc<Mutex<HashMap<SessionId, PauseState>>>,
    hitl_request: Arc<Mutex<HashMap<SessionId, HitlRequest>>>,
    active_session: Arc<Mutex<Option<SessionId>>>,
    // RPC-049: per-stub counter of `resume_session` calls. Used by
    // cross-transport parity tests to assert the round-trip lands on
    // the underlying handle once per transport.
    resume_session_calls: AtomicU64,
    // RPC-050: per-stub counters for the work-unit context RPCs.
    // Used by cross-transport parity tests to assert each transport
    // lands one call on the underlying handle.
    set_work_unit_context_calls: AtomicU64,
    get_work_unit_context_calls: AtomicU64,
    // RPC-054: per-stub state + counters for the provider credentials
    // surface. The map is keyed by provider_id and persists across
    // get/set/delete calls so cross-transport parity tests can assert
    // both transports observe the same in-memory state.
    provider_credentials: Arc<Mutex<HashMap<String, ProviderCredentialInfo>>>,
    set_provider_credentials_calls: AtomicU64,
    delete_provider_credentials_calls: AtomicU64,
    test_provider_connection_calls: AtomicU64,
    refresh_models_cache_calls: AtomicU64,
    list_provider_credentials_calls: AtomicU64,
    // RPC-055: per-stub counters for the debug-capture surface.
    // Cross-transport parity tests use these to assert each transport
    // lands one call on the underlying handle.
    toggle_debug_calls: AtomicU64,
    set_debug_directory_calls: AtomicU64,
    // RPC-056: per-stub seeded blocklist rules + per-call counter for
    // the `blocklist_list` RPC method. Cross-transport parity tests
    // assert each transport lands one call here and returns the same
    // seeded payload.
    blocklist_rules: Arc<Mutex<Vec<BlocklistRuleInfo>>>,
    blocklist_list_calls: AtomicU64,
    // RPC-057: per-stub seeded payloads + per-call counters for the
    // merge/discard/prune/list/inspect worktree RPC methods.
    merge_outcome: Arc<Mutex<MergeOutcome>>,
    pruned_sessions: Arc<Mutex<Vec<String>>>,
    session_worktrees: Arc<Mutex<Vec<SessionWorktreeInfo>>>,
    session_changes_summary: Arc<Mutex<SessionChangesSummary>>,
    merge_session_worktree_calls: AtomicU64,
    discard_session_worktree_calls: AtomicU64,
    prune_orphaned_worktrees_calls: AtomicU64,
    list_session_worktrees_calls: AtomicU64,
    inspect_session_changes_calls: AtomicU64,
    // RPC-058: per-stub seeded payloads + per-call counters for the
    // /schedule RPC surface (add / list / pause / resume / remove).
    scheduled_jobs: Arc<Mutex<Vec<ScheduledJob>>>,
    schedule_add_calls: AtomicU64,
    schedule_list_calls: AtomicU64,
    schedule_pause_calls: AtomicU64,
    schedule_resume_calls: AtomicU64,
    schedule_remove_calls: AtomicU64,
    // RPC-059: per-stub seeded payloads + per-call counters for the
    // /loop RPC surface (add / cancel / list).
    registered_loops: Arc<Mutex<Vec<RegisteredLoop>>>,
    loop_cancel_result: std::sync::atomic::AtomicBool,
    loop_add_calls: AtomicU64,
    loop_cancel_calls: AtomicU64,
    loop_list_calls: AtomicU64,
    // RPC-061: ChainOfCommand-equivalent maps + per-method call
    // counters + recorded IncomingMessageInput payloads for the
    // supervisor / subordinate surface. add_supervisor implements
    // BFS cycle detection that mirrors `codelet_sessions::ChainOfCommand`.
    sub_to_sup: Arc<Mutex<HashMap<SessionId, Vec<SessionId>>>>,
    sup_to_subs: Arc<Mutex<HashMap<SessionId, Vec<SessionId>>>>,
    recorded_incoming_messages: Arc<Mutex<Vec<(SessionId, IncomingMessageInput)>>>,
    add_supervisor_calls: AtomicU64,
    remove_supervisor_calls: AtomicU64,
    get_supervisors_calls: AtomicU64,
    get_subordinate_calls: AtomicU64,
    get_subordinates_calls: AtomicU64,
    receive_incoming_message_calls: AtomicU64,
}

#[derive(Debug, Clone)]
struct SessionRecord {
    id: SessionId,
    role: Option<String>,
    status: SessionStatus,
    /// RPC-037: track whether this session was minted by
    /// `create_isolated_session` so `list_sessions` can faithfully
    /// report `is_isolated: true` for those rows. Defaults to false
    /// for plain `create_session` rows.
    is_isolated: bool,
    /// RPC-037: matching worktree path string for isolated rows.
    /// Empty / None for non-isolated sessions.
    worktree_path: Option<String>,
}

impl Default for StubSessionManagerHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl StubSessionManagerHandle {
    /// Construct a new stub backed by a deterministic ` [Text, Done]`
    /// emission policy.
    pub fn new() -> Self {
        Self::with_capacity(256, 1024)
    }

    /// Construct a stub matching what an external StubProvider would do —
    /// parameter is ignored; kept for API parity with the test harness.
    pub fn with_provider<P>(_provider: Arc<P>) -> Self {
        Self::new()
    }

    /// Construct a stub with custom broadcast capacities (mostly useful
    /// for stress tests that need bigger buffers).
    pub fn with_capacity(chunks_capacity: usize, logs_capacity: usize) -> Self {
        let (chunks_tx, _) = broadcast::channel(chunks_capacity);
        let (logs_tx, _) = broadcast::channel(logs_capacity);
        let (status_changes_tx, _) =
            broadcast::channel(DEFAULT_STATUS_CHANGES_CAPACITY);
        Self {
            chunks_tx,
            logs_tx,
            status_changes_tx,
            sessions: Arc::new(Mutex::new(Vec::new())),
            next_id: AtomicU64::new(1),
            next_iso_id: AtomicU64::new(1),
            providers: Arc::new(Mutex::new(Vec::new())),
            tokens: Arc::new(Mutex::new(HashMap::new())),
            models: Arc::new(Mutex::new(HashMap::new())),
            work_unit_ctx: Arc::new(Mutex::new(HashMap::new())),
            pending_input: Arc::new(Mutex::new(HashMap::new())),
            debug_enabled: Arc::new(Mutex::new(HashMap::new())),
            pause_state: Arc::new(Mutex::new(HashMap::new())),
            hitl_request: Arc::new(Mutex::new(HashMap::new())),
            active_session: Arc::new(Mutex::new(None)),
            resume_session_calls: AtomicU64::new(0),
            set_work_unit_context_calls: AtomicU64::new(0),
            get_work_unit_context_calls: AtomicU64::new(0),
            provider_credentials: Arc::new(Mutex::new(HashMap::new())),
            set_provider_credentials_calls: AtomicU64::new(0),
            delete_provider_credentials_calls: AtomicU64::new(0),
            test_provider_connection_calls: AtomicU64::new(0),
            refresh_models_cache_calls: AtomicU64::new(0),
            list_provider_credentials_calls: AtomicU64::new(0),
            toggle_debug_calls: AtomicU64::new(0),
            set_debug_directory_calls: AtomicU64::new(0),
            blocklist_rules: Arc::new(Mutex::new(Vec::new())),
            blocklist_list_calls: AtomicU64::new(0),
            merge_outcome: Arc::new(Mutex::new(MergeOutcome {
                status: MergeStatus::NoChanges,
                conflicts: Vec::new(),
                merge_commit: None,
            })),
            pruned_sessions: Arc::new(Mutex::new(Vec::new())),
            session_worktrees: Arc::new(Mutex::new(Vec::new())),
            session_changes_summary: Arc::new(Mutex::new(SessionChangesSummary {
                files_changed: 0,
                insertions: 0,
                deletions: 0,
                commits: Vec::new(),
            })),
            merge_session_worktree_calls: AtomicU64::new(0),
            discard_session_worktree_calls: AtomicU64::new(0),
            prune_orphaned_worktrees_calls: AtomicU64::new(0),
            list_session_worktrees_calls: AtomicU64::new(0),
            inspect_session_changes_calls: AtomicU64::new(0),
            scheduled_jobs: Arc::new(Mutex::new(Vec::new())),
            schedule_add_calls: AtomicU64::new(0),
            schedule_list_calls: AtomicU64::new(0),
            schedule_pause_calls: AtomicU64::new(0),
            schedule_resume_calls: AtomicU64::new(0),
            schedule_remove_calls: AtomicU64::new(0),
            registered_loops: Arc::new(Mutex::new(Vec::new())),
            loop_cancel_result: std::sync::atomic::AtomicBool::new(true),
            loop_add_calls: AtomicU64::new(0),
            loop_cancel_calls: AtomicU64::new(0),
            loop_list_calls: AtomicU64::new(0),
            // RPC-061: supervisor / subordinate surface state.
            sub_to_sup: Arc::new(Mutex::new(HashMap::new())),
            sup_to_subs: Arc::new(Mutex::new(HashMap::new())),
            recorded_incoming_messages: Arc::new(Mutex::new(Vec::new())),
            add_supervisor_calls: AtomicU64::new(0),
            remove_supervisor_calls: AtomicU64::new(0),
            get_supervisors_calls: AtomicU64::new(0),
            get_subordinate_calls: AtomicU64::new(0),
            get_subordinates_calls: AtomicU64::new(0),
            receive_incoming_message_calls: AtomicU64::new(0),
        }
    }

    /// RPC-049: how many times `resume_session` has been called on this
    /// stub. Used by cross-transport parity tests to assert the
    /// round-trip lands on the underlying handle once per transport.
    pub fn resume_session_calls(&self) -> u64 {
        self.resume_session_calls.load(Ordering::SeqCst)
    }

    /// RPC-050: how many times `set_work_unit_context` has been called
    /// on this stub. Used by cross-transport parity tests.
    pub fn set_work_unit_context_calls(&self) -> u64 {
        self.set_work_unit_context_calls.load(Ordering::SeqCst)
    }

    /// RPC-050: how many times `get_work_unit_context` has been called
    /// on this stub. Used by cross-transport parity tests.
    pub fn get_work_unit_context_calls(&self) -> u64 {
        self.get_work_unit_context_calls.load(Ordering::SeqCst)
    }

    /// RPC-054: how many times `set_provider_credentials` has been
    /// called on this stub. Drives cross-transport parity tests.
    pub fn set_provider_credentials_calls(&self) -> u64 {
        self.set_provider_credentials_calls.load(Ordering::SeqCst)
    }

    /// RPC-054: how many times `delete_provider_credentials` has been
    /// called on this stub.
    pub fn delete_provider_credentials_calls(&self) -> u64 {
        self.delete_provider_credentials_calls.load(Ordering::SeqCst)
    }

    /// RPC-054: how many times `test_provider_connection` has been
    /// called on this stub.
    pub fn test_provider_connection_calls(&self) -> u64 {
        self.test_provider_connection_calls.load(Ordering::SeqCst)
    }

    /// RPC-054: how many times `refresh_models_cache` has been called
    /// on this stub.
    pub fn refresh_models_cache_calls(&self) -> u64 {
        self.refresh_models_cache_calls.load(Ordering::SeqCst)
    }

    /// RPC-054: how many times `list_provider_credentials` has been
    /// called on this stub.
    pub fn list_provider_credentials_calls(&self) -> u64 {
        self.list_provider_credentials_calls.load(Ordering::SeqCst)
    }

    /// RPC-055: how many times `toggle_debug` has been called on this
    /// stub. Used by cross-transport parity tests to assert each
    /// transport lands one call on the underlying handle.
    pub fn toggle_debug_calls(&self) -> u64 {
        self.toggle_debug_calls.load(Ordering::SeqCst)
    }

    /// RPC-055: how many times `set_debug_directory` has been called on
    /// this stub.
    pub fn set_debug_directory_calls(&self) -> u64 {
        self.set_debug_directory_calls.load(Ordering::SeqCst)
    }

    /// RPC-056: how many times `blocklist_list` has been called on this
    /// stub. Used by cross-transport parity tests to assert each
    /// transport lands one call on the underlying handle.
    pub fn blocklist_list_calls(&self) -> u64 {
        self.blocklist_list_calls.load(Ordering::SeqCst)
    }

    /// RPC-056: replace the in-memory blocklist rule list returned by
    /// `blocklist_list`. Used by the cross-transport parity test to
    /// seed deterministic data both transports can read identically.
    pub fn seed_blocklist_rules(&self, rules: Vec<BlocklistRuleInfo>) {
        if let Ok(mut g) = self.blocklist_rules.lock() {
            *g = rules;
        }
    }

    /// RPC-057: per-call counter — `merge_session_worktree`.
    pub fn merge_session_worktree_calls(&self) -> u64 {
        self.merge_session_worktree_calls.load(Ordering::SeqCst)
    }

    /// RPC-057: per-call counter — `discard_session_worktree`.
    pub fn discard_session_worktree_calls(&self) -> u64 {
        self.discard_session_worktree_calls.load(Ordering::SeqCst)
    }

    /// RPC-057: per-call counter — `prune_orphaned_worktrees`.
    pub fn prune_orphaned_worktrees_calls(&self) -> u64 {
        self.prune_orphaned_worktrees_calls.load(Ordering::SeqCst)
    }

    /// RPC-057: per-call counter — `list_session_worktrees`.
    pub fn list_session_worktrees_calls(&self) -> u64 {
        self.list_session_worktrees_calls.load(Ordering::SeqCst)
    }

    /// RPC-057: per-call counter — `inspect_session_changes`.
    pub fn inspect_session_changes_calls(&self) -> u64 {
        self.inspect_session_changes_calls.load(Ordering::SeqCst)
    }

    /// RPC-057: seed the [`MergeOutcome`] returned by every subsequent
    /// `merge_session_worktree` call.
    pub fn seed_merge_outcome(&self, outcome: MergeOutcome) {
        if let Ok(mut g) = self.merge_outcome.lock() {
            *g = outcome;
        }
    }

    /// RPC-057: seed the pruned-session-id list returned by every
    /// subsequent `prune_orphaned_worktrees` call.
    pub fn seed_pruned_sessions(&self, ids: Vec<String>) {
        if let Ok(mut g) = self.pruned_sessions.lock() {
            *g = ids;
        }
    }

    /// RPC-057: seed the session worktree rows returned by every
    /// subsequent `list_session_worktrees` call.
    pub fn seed_session_worktrees(&self, rows: Vec<SessionWorktreeInfo>) {
        if let Ok(mut g) = self.session_worktrees.lock() {
            *g = rows;
        }
    }

    /// RPC-057: seed the changes summary returned by every subsequent
    /// `inspect_session_changes` call.
    pub fn seed_session_changes_summary(&self, summary: SessionChangesSummary) {
        if let Ok(mut g) = self.session_changes_summary.lock() {
            *g = summary;
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule stub state + seeds.
    // ─────────────────────────────────────────────────────────────────

    /// RPC-058: seed a single scheduled job as the only row the stub
    /// will return for `schedule_list` / `schedule_add` / etc.
    pub fn seed_scheduled_job(&self, job: ScheduledJob) {
        if let Ok(mut g) = self.scheduled_jobs.lock() {
            *g = vec![job];
        }
    }

    /// RPC-058: seed a list of scheduled jobs the stub will return for
    /// `schedule_list`.
    pub fn seed_scheduled_jobs(&self, jobs: Vec<ScheduledJob>) {
        if let Ok(mut g) = self.scheduled_jobs.lock() {
            *g = jobs;
        }
    }

    /// RPC-058: how many times `schedule_add` has been called on this
    /// stub.
    pub fn schedule_add_calls(&self) -> u64 {
        self.schedule_add_calls.load(Ordering::SeqCst)
    }

    /// RPC-058: how many times `schedule_list` has been called on this
    /// stub.
    pub fn schedule_list_calls(&self) -> u64 {
        self.schedule_list_calls.load(Ordering::SeqCst)
    }

    /// RPC-058: how many times `schedule_pause` has been called on this
    /// stub.
    pub fn schedule_pause_calls(&self) -> u64 {
        self.schedule_pause_calls.load(Ordering::SeqCst)
    }

    /// RPC-058: how many times `schedule_resume` has been called on this
    /// stub.
    pub fn schedule_resume_calls(&self) -> u64 {
        self.schedule_resume_calls.load(Ordering::SeqCst)
    }

    /// RPC-058: how many times `schedule_remove` has been called on this
    /// stub.
    pub fn schedule_remove_calls(&self) -> u64 {
        self.schedule_remove_calls.load(Ordering::SeqCst)
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop stub state + seeds.
    // ─────────────────────────────────────────────────────────────────

    /// RPC-059: seed a single registered loop as the only row the stub
    /// will return for `loop_list` / `loop_add` / etc.
    pub fn seed_registered_loop(&self, entry: RegisteredLoop) {
        if let Ok(mut g) = self.registered_loops.lock() {
            *g = vec![entry];
        }
    }

    /// RPC-059: seed a list of registered loops the stub will return
    /// for `loop_list`.
    pub fn seed_registered_loops(&self, entries: Vec<RegisteredLoop>) {
        if let Ok(mut g) = self.registered_loops.lock() {
            *g = entries;
        }
    }

    /// RPC-059: seed the boolean returned by `loop_cancel`. Defaults to
    /// `true`; tests that need a "not found" outcome pass `false`.
    pub fn seed_loop_cancel_result(&self, result: bool) {
        self.loop_cancel_result
            .store(result, Ordering::SeqCst);
    }

    /// RPC-059: how many times `loop_add` has been called on this stub.
    pub fn loop_add_calls(&self) -> u64 {
        self.loop_add_calls.load(Ordering::SeqCst)
    }

    /// RPC-059: how many times `loop_cancel` has been called on this stub.
    pub fn loop_cancel_calls(&self) -> u64 {
        self.loop_cancel_calls.load(Ordering::SeqCst)
    }

    /// RPC-059: how many times `loop_list` has been called on this stub.
    pub fn loop_list_calls(&self) -> u64 {
        self.loop_list_calls.load(Ordering::SeqCst)
    }

    /// RPC-061: per-call counter for `add_supervisor`.
    pub fn add_supervisor_calls(&self) -> u64 {
        self.add_supervisor_calls.load(Ordering::SeqCst)
    }

    /// RPC-061: per-call counter for `remove_supervisor`.
    pub fn remove_supervisor_calls(&self) -> u64 {
        self.remove_supervisor_calls.load(Ordering::SeqCst)
    }

    /// RPC-061: per-call counter for `get_supervisors`.
    pub fn get_supervisors_calls(&self) -> u64 {
        self.get_supervisors_calls.load(Ordering::SeqCst)
    }

    /// RPC-061: per-call counter for `get_subordinate`.
    pub fn get_subordinate_calls(&self) -> u64 {
        self.get_subordinate_calls.load(Ordering::SeqCst)
    }

    /// RPC-061: per-call counter for `get_subordinates`.
    pub fn get_subordinates_calls(&self) -> u64 {
        self.get_subordinates_calls.load(Ordering::SeqCst)
    }

    /// RPC-061: per-call counter for `receive_incoming_message`.
    pub fn receive_incoming_message_calls(&self) -> u64 {
        self.receive_incoming_message_calls.load(Ordering::SeqCst)
    }

    /// RPC-061: snapshot of every `(subordinate_id, IncomingMessageInput)`
    /// payload recorded by `receive_incoming_message`. Cross-transport
    /// parity tests assert both transports land their respective
    /// payloads in this Vec.
    pub fn recorded_incoming_messages(&self) -> Vec<(SessionId, IncomingMessageInput)> {
        self.recorded_incoming_messages
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// RPC-054: seed a [`ProviderCredentialInfo`] row so a test can
    /// observe `list_provider_credentials` / `get_provider_credential`
    /// returning a non-empty list without going through `set_...`.
    pub fn seed_provider_credential(&self, info: ProviderCredentialInfo) {
        if let Ok(mut guard) = self.provider_credentials.lock() {
            guard.insert(info.provider_id.clone(), info);
        }
    }

    /// RPC-037: seed a `SessionTokens` value for a session id.
    pub fn seed_session_tokens(&self, session_id: SessionId, tokens: SessionTokens) {
        if let Ok(mut guard) = self.tokens.lock() {
            guard.insert(session_id, tokens);
        }
    }

    /// RPC-037: seed a `SessionModel` value for a session id.
    pub fn seed_session_model(&self, session_id: SessionId, model: SessionModel) {
        if let Ok(mut guard) = self.models.lock() {
            guard.insert(session_id, model);
        }
    }

    /// RPC-037: seed a `PauseState` for a session id.
    pub fn seed_pause_state(&self, session_id: SessionId, state: PauseState) {
        if let Ok(mut guard) = self.pause_state.lock() {
            guard.insert(session_id, state);
        }
    }

    /// RPC-037: seed a `HitlRequest` for a session id.
    pub fn seed_hitl_request(&self, session_id: SessionId, request: HitlRequest) {
        if let Ok(mut guard) = self.hitl_request.lock() {
            guard.insert(session_id, request);
        }
    }

    /// RPC-022: pre-seed the provider/model registry returned by
    /// `list_providers`. Used by cross-transport parity tests that need
    /// the stub to return a non-empty registry without dragging in the
    /// real `SessionManager` + `ProviderManager` dependency tree.
    pub fn set_providers(&self, providers: Vec<ProviderInfo>) {
        if let Ok(mut guard) = self.providers.lock() {
            *guard = providers;
        }
    }

    /// Get a clonable handle to the chunks broadcast sender so the host
    /// can dual-fanout chunks (e.g. NAPI ThreadsafeFunction co-listener).
    pub fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)> {
        self.chunks_tx.clone()
    }

    /// Get a clonable handle to the logs broadcast sender so the host's
    /// tracing::Layer can push records.
    pub fn logs_tx(&self) -> broadcast::Sender<LogRecord> {
        self.logs_tx.clone()
    }

    fn set_status(&self, session_id: &SessionId, status: SessionStatus) {
        if let Ok(mut sessions) = self.sessions.lock() {
            for record in sessions.iter_mut() {
                if record.id == *session_id {
                    record.status = status;
                    return;
                }
            }
        }
    }
}

impl SessionManagerHandle for StubSessionManagerHandle {
    fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = match self.sessions.lock() {
            Ok(sessions) => sessions,
            Err(_) => return Vec::new(),
        };
        sessions
            .iter()
            .map(|r| SessionInfo {
                id: r.id.value.clone(),
                name: r.id.value.clone(),
                status: status_str(r.status).to_string(),
                project: String::new(),
                message_count: 0,
                provider_id: None,
                model_id: None,
                is_isolated: r.is_isolated,
                worktree_path: r.worktree_path.clone(),
                role: r.role.clone(),
            })
            .collect()
    }

    fn create_session(&self, role: Option<String>) -> SessionId {
        let id = SessionId::new(format!(
            "stub-{}",
            self.next_id.fetch_add(1, Ordering::SeqCst)
        ));
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.push(SessionRecord {
                id: id.clone(),
                role,
                status: SessionStatus::Idle,
                is_isolated: false,
                worktree_path: None,
            });
        }
        id
    }

    fn send_input(&self, session_id: &SessionId, _text: String) {
        self.set_status(session_id, SessionStatus::Running);
        let _ = self
            .status_changes_tx
            .send((session_id.clone(), SessionStatus::Running));

        let chunks_tx = self.chunks_tx.clone();
        let status_tx = self.status_changes_tx.clone();
        let sid = session_id.clone();
        let sessions = Arc::clone(&self.sessions);
        tokio::spawn(async move {
            // Deterministic stub-provider sequence: [Text("hi back"), Done].
            let _ = chunks_tx.send((sid.clone(), StreamChunk::text("hi back".to_string())));
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            let _ = chunks_tx.send((sid.clone(), StreamChunk::done()));

            // Flip state back to Idle once the stream has completed.
            if let Ok(mut sessions) = sessions.lock() {
                for record in sessions.iter_mut() {
                    if record.id == sid {
                        record.status = SessionStatus::Idle;
                        break;
                    }
                }
            }
            let _ = status_tx.send((sid, SessionStatus::Idle));
        });
    }

    fn interrupt(&self, session_id: &SessionId) {
        self.set_status(session_id, SessionStatus::Interrupted);
        let _ = self
            .chunks_tx
            .send((session_id.clone(), StreamChunk::interrupted(Vec::new())));
    }

    fn get_session_status(&self, session_id: &SessionId) -> SessionStatus {
        let sessions = match self.sessions.lock() {
            Ok(sessions) => sessions,
            Err(_) => return SessionStatus::Idle,
        };
        sessions
            .iter()
            .find(|r| r.id == *session_id)
            .map(|r| r.status)
            .unwrap_or(SessionStatus::Idle)
    }

    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.chunks_tx.subscribe()
    }

    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.logs_tx.subscribe()
    }

    fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)> {
        self.chunks_tx.clone()
    }

    fn logs_tx(&self) -> broadcast::Sender<LogRecord> {
        self.logs_tx.clone()
    }

    fn list_providers(&self) -> Vec<ProviderInfo> {
        match self.providers.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => Vec::new(),
        }
    }

    // ========================================================================
    // RPC-037: deterministic stub overrides used by cross-transport parity
    // tests. Every override is small, synchronous, and idempotent so the
    // embedded and WebSocket paths produce byte-identical results.
    // ========================================================================

    fn send_input_with_thinking(
        &self,
        session_id: &SessionId,
        text: String,
        _thinking: Option<ThinkingConfig>,
    ) {
        // Discard the thinking config (deterministic stub behaviour) and
        // delegate to the same chunk-emitting path as send_input.
        self.send_input(session_id, text);
    }

    fn get_session_tokens(&self, session_id: &SessionId) -> SessionTokens {
        match self.tokens.lock() {
            Ok(guard) => guard.get(session_id).cloned().unwrap_or(SessionTokens {
                input_tokens: 0,
                output_tokens: 0,
            }),
            Err(_) => SessionTokens {
                input_tokens: 0,
                output_tokens: 0,
            },
        }
    }

    fn get_session_model(&self, session_id: &SessionId) -> SessionModel {
        match self.models.lock() {
            Ok(guard) => guard.get(session_id).cloned().unwrap_or(SessionModel {
                provider_id: String::new(),
                model_id: String::new(),
                context_window: 0,
                max_output_tokens: 0,
                compaction_threshold: 0,
            }),
            Err(_) => SessionModel {
                provider_id: String::new(),
                model_id: String::new(),
                context_window: 0,
                max_output_tokens: 0,
                compaction_threshold: 0,
            },
        }
    }

    fn get_compaction_progress(&self, _session_id: &SessionId) -> Option<CompactionProgress> {
        None
    }

    fn get_buffered_output(&self, _session_id: &SessionId, _limit: u32) -> Vec<StreamChunk> {
        Vec::new()
    }

    fn clear_history(&self, session_id: &SessionId) -> Result<(), String> {
        // RPC-074 (TS parity): mirror `BackgroundSession::clear_history` —
        // emit a `SessionStateChange { state: Cleared }` chunk so any
        // subscriber (TUI, cross-transport parity tests) drives its UI
        // reset off the same chunk on both the stub and the real impl.
        // The previous `UserNotification { message: ... }` broadcast
        // for /clear was a Rust-side invention with no counterpart in the
        // TypeScript reference (`src/tui/components/AgentView.tsx:1554-1564`,
        // handleClearCommand → TUI-066 contract) and has been removed.
        let _ = self.chunks_tx.send((
            session_id.clone(),
            StreamChunk::session_state_change(SessionState::Cleared),
        ));
        Ok(())
    }

    fn compact_session(&self, session_id: &SessionId) -> Result<CompactionResult, String> {
        let canned = CompactionResult {
            original_tokens: 1000,
            compacted_tokens: 500,
            compression_ratio: 0.5,
            turns_summarized: 4,
            turns_kept: 2,
        };
        let _ = self.chunks_tx.send((
            session_id.clone(),
            StreamChunk::compaction_complete(canned.clone()),
        ));
        Ok(canned)
    }

    fn restore_session_messages(
        &self,
        _session_id: &SessionId,
        _envelopes: Vec<String>,
    ) -> Result<(), String> {
        Ok(())
    }

    fn restore_session_token_state(
        &self,
        _session_id: &SessionId,
        _state: TokenRestoreState,
    ) -> Result<(), String> {
        Ok(())
    }

    /// RPC-049: deterministic stub override — increments the per-stub
    /// call counter and returns `Ok(())` WITHOUT reaching for the
    /// on-disk persistence layer (the default trait impl would call
    /// `codelet_core::persistence::load_session` which doesn't make
    /// sense against the stub's in-memory session table).
    fn resume_session(&self, _session_id: &SessionId) -> Result<(), String> {
        self.resume_session_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn get_work_unit_context(&self, session_id: &SessionId) -> Option<WorkUnitContext> {
        self.get_work_unit_context_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.work_unit_ctx.lock() {
            Ok(guard) => guard.get(session_id).cloned(),
            Err(_) => None,
        }
    }

    fn set_work_unit_context(
        &self,
        session_id: &SessionId,
        ctx: Option<WorkUnitContext>,
    ) -> Result<(), String> {
        self.set_work_unit_context_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Ok(mut guard) = self.work_unit_ctx.lock() {
            match ctx {
                Some(c) => {
                    guard.insert(session_id.clone(), c);
                }
                None => {
                    guard.remove(session_id);
                }
            }
        }
        Ok(())
    }

    fn get_pending_input(&self, session_id: &SessionId) -> Option<String> {
        match self.pending_input.lock() {
            Ok(guard) => guard.get(session_id).cloned(),
            Err(_) => None,
        }
    }

    fn set_pending_input(&self, session_id: &SessionId, text: Option<String>) {
        if let Ok(mut guard) = self.pending_input.lock() {
            match text {
                Some(t) => {
                    guard.insert(session_id.clone(), t);
                }
                None => {
                    guard.remove(session_id);
                }
            }
        }
    }

    fn set_active_session(&self, session_id: &SessionId) {
        if let Ok(mut guard) = self.active_session.lock() {
            *guard = Some(session_id.clone());
        }
    }

    fn clear_active_session(&self) {
        if let Ok(mut guard) = self.active_session.lock() {
            *guard = None;
        }
    }

    fn get_active_session(&self) -> Option<SessionId> {
        self.active_session.lock().ok().and_then(|g| g.clone())
    }

    fn get_effective_cwd(&self, _session_id: &SessionId) -> PathBuf {
        PathBuf::new()
    }

    fn get_supervisors(&self, session_id: &SessionId) -> Vec<SessionId> {
        self.get_supervisors_calls.fetch_add(1, Ordering::SeqCst);
        self.sub_to_sup
            .lock()
            .ok()
            .and_then(|g| g.get(session_id).cloned())
            .unwrap_or_default()
    }

    fn add_supervisor(
        &self,
        subordinate_id: &SessionId,
        supervisor_id: &SessionId,
    ) -> Result<(), String> {
        self.add_supervisor_calls.fetch_add(1, Ordering::SeqCst);
        let mut sup2subs = self
            .sup_to_subs
            .lock()
            .map_err(|_| "sup_to_subs lock poisoned".to_string())?;
        // Duplicate detection.
        if let Some(existing) = sup2subs.get(supervisor_id) {
            if existing.iter().any(|s| s == subordinate_id) {
                return Err(
                    "subordinate already registered under this supervisor".to_string(),
                );
            }
        }
        // BFS cycle detection mirroring
        // `codelet_sessions::ChainOfCommand::add_supervisor`.
        {
            let mut visited: std::collections::HashSet<SessionId> =
                std::collections::HashSet::new();
            let mut queue: std::collections::VecDeque<SessionId> =
                std::collections::VecDeque::new();
            queue.push_back(subordinate_id.clone());
            visited.insert(subordinate_id.clone());
            while let Some(current) = queue.pop_front() {
                if let Some(subs) = sup2subs.get(&current) {
                    for s in subs.iter() {
                        if s == supervisor_id {
                            return Err("circular supervision not allowed".to_string());
                        }
                        if visited.insert(s.clone()) {
                            queue.push_back(s.clone());
                        }
                    }
                }
            }
        }
        sup2subs
            .entry(supervisor_id.clone())
            .or_default()
            .push(subordinate_id.clone());
        drop(sup2subs);
        let mut sub2sup = self
            .sub_to_sup
            .lock()
            .map_err(|_| "sub_to_sup lock poisoned".to_string())?;
        sub2sup
            .entry(subordinate_id.clone())
            .or_default()
            .push(supervisor_id.clone());
        Ok(())
    }

    fn remove_supervisor(&self, supervisor_id: &SessionId) -> Result<(), String> {
        self.remove_supervisor_calls.fetch_add(1, Ordering::SeqCst);
        let subordinate_ids = {
            let mut sup2subs = self
                .sup_to_subs
                .lock()
                .map_err(|_| "sup_to_subs lock poisoned".to_string())?;
            sup2subs.remove(supervisor_id).unwrap_or_default()
        };
        if !subordinate_ids.is_empty() {
            let mut sub2sup = self
                .sub_to_sup
                .lock()
                .map_err(|_| "sub_to_sup lock poisoned".to_string())?;
            for sid in subordinate_ids {
                if let Some(supervisors) = sub2sup.get_mut(&sid) {
                    supervisors.retain(|s| s != supervisor_id);
                    if supervisors.is_empty() {
                        sub2sup.remove(&sid);
                    }
                }
            }
        }
        Ok(())
    }

    fn get_subordinate(&self, supervisor_id: &SessionId) -> Option<SessionId> {
        self.get_subordinate_calls.fetch_add(1, Ordering::SeqCst);
        self.sup_to_subs
            .lock()
            .ok()
            .and_then(|g| g.get(supervisor_id).and_then(|v| v.first().cloned()))
    }

    fn get_subordinates(&self, supervisor_id: &SessionId) -> Vec<SessionId> {
        self.get_subordinates_calls.fetch_add(1, Ordering::SeqCst);
        self.sup_to_subs
            .lock()
            .ok()
            .and_then(|g| g.get(supervisor_id).cloned())
            .unwrap_or_default()
    }

    fn receive_incoming_message(
        &self,
        subordinate_id: &SessionId,
        message: IncomingMessageInput,
    ) -> Result<(), String> {
        self.receive_incoming_message_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Ok(mut guard) = self.recorded_incoming_messages.lock() {
            guard.push((subordinate_id.clone(), message));
        }
        Ok(())
    }

    fn get_debug_enabled(&self, session_id: &SessionId) -> bool {
        match self.debug_enabled.lock() {
            Ok(guard) => guard.get(session_id).copied().unwrap_or(false),
            Err(_) => false,
        }
    }

    fn set_debug_enabled(&self, session_id: &SessionId, enabled: bool) {
        if let Ok(mut guard) = self.debug_enabled.lock() {
            guard.insert(session_id.clone(), enabled);
        }
    }

    fn toggle_debug(
        &self,
        session_id: &SessionId,
        debug_dir: &str,
    ) -> Result<String, String> {
        self.toggle_debug_calls.fetch_add(1, Ordering::SeqCst);
        let current = self.get_debug_enabled(session_id);
        let next = !current;
        self.set_debug_enabled(session_id, next);
        let _ = self
            .chunks_tx
            .send((session_id.clone(), StreamChunk::debug_state_change(next)));
        Ok(debug_dir.to_string())
    }

    fn set_debug_directory(&self, path: PathBuf) -> Result<(), String> {
        self.set_debug_directory_calls.fetch_add(1, Ordering::SeqCst);
        let _ = path;
        Ok(())
    }

    fn pause_resume(&self, session_id: &SessionId) -> Result<(), String> {
        if let Ok(mut guard) = self.pause_state.lock() {
            guard.remove(session_id);
        }
        self.set_status(session_id, SessionStatus::Running);
        let _ = self.status_changes_tx.send((session_id.clone(), SessionStatus::Running));
        let _ = self.chunks_tx.send((
            session_id.clone(),
            StreamChunk::session_state_change(SessionState::Running),
        ));
        Ok(())
    }

    fn pause_confirm(
        &self,
        session_id: &SessionId,
        _accept: bool,
    ) -> Result<(), String> {
        self.pause_resume(session_id)
    }

    fn pause_triple(
        &self,
        session_id: &SessionId,
        _choice: ApprovalChoice,
    ) -> Result<(), String> {
        self.pause_resume(session_id)
    }

    fn send_hitl_response(
        &self,
        session_id: &SessionId,
        _response: HitlResponse,
    ) -> Result<(), String> {
        if let Ok(mut guard) = self.hitl_request.lock() {
            guard.remove(session_id);
        }
        Ok(())
    }

    fn get_pause_state(&self, session_id: &SessionId) -> Option<PauseState> {
        match self.pause_state.lock() {
            Ok(guard) => guard.get(session_id).cloned(),
            Err(_) => None,
        }
    }

    fn get_hitl_request(&self, session_id: &SessionId) -> Option<HitlRequest> {
        match self.hitl_request.lock() {
            Ok(guard) => guard.get(session_id).cloned(),
            Err(_) => None,
        }
    }

    fn send_fspec_result(
        &self,
        _session_id: &SessionId,
        _result: FspecResult,
    ) -> Result<(), String> {
        Ok(())
    }

    fn create_isolated_session(
        &self,
        role: Option<String>,
    ) -> Result<IsolatedSessionInfo, String> {
        let seq = self.next_iso_id.fetch_add(1, Ordering::SeqCst);
        let id = SessionId::new(format!("stub-iso-{seq}"));
        let worktree_path = format!("/tmp/stub-wt-{seq}");
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.push(SessionRecord {
                id: id.clone(),
                role,
                status: SessionStatus::Idle,
                is_isolated: true,
                worktree_path: Some(worktree_path.clone()),
            });
        }
        Ok(IsolatedSessionInfo {
            session_id: id,
            worktree_path,
            base_commit: "abc1234".to_string(),
        })
    }

    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        self.status_changes_tx.subscribe()
    }

    fn status_changes_tx(&self) -> broadcast::Sender<(SessionId, SessionStatus)> {
        self.status_changes_tx.clone()
    }

    fn destroy_session(&self, session_id: &SessionId) -> Result<(), String> {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.retain(|r| r.id != *session_id);
        }
        // Clean up keyed state.
        if let Ok(mut g) = self.tokens.lock() {
            g.remove(session_id);
        }
        if let Ok(mut g) = self.models.lock() {
            g.remove(session_id);
        }
        if let Ok(mut g) = self.work_unit_ctx.lock() {
            g.remove(session_id);
        }
        if let Ok(mut g) = self.pending_input.lock() {
            g.remove(session_id);
        }
        if let Ok(mut g) = self.debug_enabled.lock() {
            g.remove(session_id);
        }
        if let Ok(mut g) = self.pause_state.lock() {
            g.remove(session_id);
        }
        if let Ok(mut g) = self.hitl_request.lock() {
            g.remove(session_id);
        }
        if let Ok(mut g) = self.active_session.lock() {
            if g.as_ref() == Some(session_id) {
                *g = None;
            }
        }
        Ok(())
    }

    // ========================================================================
    // RPC-054: Provider credentials surface — deterministic in-memory
    // implementation used by cross-transport parity tests. The
    // `provider_credentials` map persists across get/set/delete so the
    // embedded + WebSocket transports observe the same state.
    // ========================================================================

    fn list_provider_credentials(&self) -> Vec<ProviderCredentialInfo> {
        self.list_provider_credentials_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.provider_credentials.lock() {
            Ok(guard) => {
                let mut out: Vec<ProviderCredentialInfo> = guard.values().cloned().collect();
                out.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
                out
            }
            Err(_) => Vec::new(),
        }
    }

    fn get_provider_credential(&self, provider_id: &str) -> Option<ProviderCredentialInfo> {
        match self.provider_credentials.lock() {
            Ok(guard) => guard.get(provider_id).cloned(),
            Err(_) => None,
        }
    }

    fn set_provider_credentials(
        &self,
        provider_id: &str,
        creds: ProviderCredentialInput,
    ) -> Result<(), String> {
        self.set_provider_credentials_calls
            .fetch_add(1, Ordering::SeqCst);
        // Light validation: each kind requires its primary field.
        match creds.kind.as_str() {
            "api_key" => {
                if creds.api_key.as_deref().unwrap_or("").is_empty() {
                    return Err("api_key input requires a non-empty api_key".to_string());
                }
            }
            "oauth" => {
                if creds.oauth_token.as_deref().unwrap_or("").is_empty() {
                    return Err("oauth input requires a non-empty oauth_token".to_string());
                }
            }
            "custom" => {
                if creds.custom_endpoint.as_deref().unwrap_or("").is_empty() {
                    return Err(
                        "custom input requires a non-empty custom_endpoint".to_string(),
                    );
                }
            }
            other => return Err(format!("unknown credential kind: {other}")),
        }
        if let Ok(mut guard) = self.provider_credentials.lock() {
            let existing = guard
                .get(provider_id)
                .cloned()
                .unwrap_or(ProviderCredentialInfo {
                    provider_id: provider_id.to_string(),
                    display_name: provider_id.to_string(),
                    configured: false,
                    credential_type: creds.kind.clone(),
                    model_count: 0,
                });
            guard.insert(
                provider_id.to_string(),
                ProviderCredentialInfo {
                    configured: true,
                    credential_type: creds.kind,
                    ..existing
                },
            );
        }
        Ok(())
    }

    fn delete_provider_credentials(&self, provider_id: &str) -> Result<(), String> {
        self.delete_provider_credentials_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Ok(mut guard) = self.provider_credentials.lock() {
            if let Some(existing) = guard.get(provider_id).cloned() {
                guard.insert(
                    provider_id.to_string(),
                    ProviderCredentialInfo {
                        configured: false,
                        model_count: 0,
                        ..existing
                    },
                );
            }
        }
        Ok(())
    }

    fn test_provider_connection(
        &self,
        _provider_id: &str,
    ) -> Result<TestConnectionResult, String> {
        self.test_provider_connection_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(TestConnectionResult {
            success: true,
            error: None,
            latency_ms: 7,
        })
    }

    fn refresh_models_cache(
        &self,
        provider_id: &str,
    ) -> Result<Vec<codelet_rpc_types::ModelEntry>, String> {
        self.refresh_models_cache_calls
            .fetch_add(1, Ordering::SeqCst);
        // Bump the model_count on the matching row so downstream
        // assertions can observe the refresh landing on the stub.
        if let Ok(mut guard) = self.provider_credentials.lock() {
            if let Some(existing) = guard.get(provider_id).cloned() {
                guard.insert(
                    provider_id.to_string(),
                    ProviderCredentialInfo {
                        model_count: existing.model_count.saturating_add(1),
                        ..existing
                    },
                );
            }
        }
        Ok(Vec::new())
    }

    fn blocklist_list(&self) -> Vec<BlocklistRuleInfo> {
        self.blocklist_list_calls.fetch_add(1, Ordering::SeqCst);
        match self.blocklist_rules.lock() {
            Ok(g) => g.clone(),
            Err(_) => Vec::new(),
        }
    }

    // ========================================================================
    // RPC-057: Merge/worktree surface — deterministic stub overrides used
    // by cross-transport parity tests. Each method increments a per-call
    // counter and returns the seeded value (or a safe default).
    // ========================================================================

    fn merge_session_worktree(
        &self,
        _session_id: &SessionId,
        _strategy: MergeStrategy,
    ) -> Result<MergeOutcome, String> {
        self.merge_session_worktree_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.merge_outcome.lock() {
            Ok(g) => Ok(g.clone()),
            Err(_) => Ok(MergeOutcome {
                status: MergeStatus::NoChanges,
                conflicts: Vec::new(),
                merge_commit: None,
            }),
        }
    }

    fn discard_session_worktree(
        &self,
        _session_id: &SessionId,
    ) -> Result<(), String> {
        self.discard_session_worktree_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn prune_orphaned_worktrees(&self) -> Result<Vec<String>, String> {
        self.prune_orphaned_worktrees_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.pruned_sessions.lock() {
            Ok(g) => Ok(g.clone()),
            Err(_) => Ok(Vec::new()),
        }
    }

    fn list_session_worktrees(&self) -> Vec<SessionWorktreeInfo> {
        self.list_session_worktrees_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.session_worktrees.lock() {
            Ok(g) => g.clone(),
            Err(_) => Vec::new(),
        }
    }

    fn inspect_session_changes(
        &self,
        _session_id: &SessionId,
    ) -> Result<SessionChangesSummary, String> {
        self.inspect_session_changes_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.session_changes_summary.lock() {
            Ok(g) => Ok(g.clone()),
            Err(_) => Ok(SessionChangesSummary {
                files_changed: 0,
                insertions: 0,
                deletions: 0,
                commits: Vec::new(),
            }),
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule stub impls.
    // ─────────────────────────────────────────────────────────────────

    fn schedule_add(&self, job: ScheduledJob) -> Result<ScheduledJob, String> {
        self.schedule_add_calls.fetch_add(1, Ordering::SeqCst);
        match self.scheduled_jobs.lock() {
            Ok(g) => {
                if let Some(first) = g.first() {
                    Ok(first.clone())
                } else {
                    Ok(job)
                }
            }
            Err(_) => Ok(job),
        }
    }

    fn schedule_list(&self) -> Vec<ScheduledJob> {
        self.schedule_list_calls.fetch_add(1, Ordering::SeqCst);
        match self.scheduled_jobs.lock() {
            Ok(g) => g.clone(),
            Err(_) => Vec::new(),
        }
    }

    fn schedule_pause(&self, name: &str) -> Result<ScheduledJob, String> {
        self.schedule_pause_calls.fetch_add(1, Ordering::SeqCst);
        match self.scheduled_jobs.lock() {
            Ok(g) => {
                if let Some(first) = g.first() {
                    Ok(first.clone())
                } else {
                    Ok(ScheduledJob {
                        name: name.to_string(),
                        status: "paused".to_string(),
                        ..ScheduledJob::default()
                    })
                }
            }
            Err(_) => Ok(ScheduledJob {
                name: name.to_string(),
                status: "paused".to_string(),
                ..ScheduledJob::default()
            }),
        }
    }

    fn schedule_resume(&self, name: &str) -> Result<ScheduledJob, String> {
        self.schedule_resume_calls.fetch_add(1, Ordering::SeqCst);
        match self.scheduled_jobs.lock() {
            Ok(g) => {
                if let Some(first) = g.first() {
                    Ok(first.clone())
                } else {
                    Ok(ScheduledJob {
                        name: name.to_string(),
                        status: "active".to_string(),
                        ..ScheduledJob::default()
                    })
                }
            }
            Err(_) => Ok(ScheduledJob {
                name: name.to_string(),
                status: "active".to_string(),
                ..ScheduledJob::default()
            }),
        }
    }

    fn schedule_remove(&self, _name: &str) -> Result<(), String> {
        self.schedule_remove_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop stub impls.
    // ─────────────────────────────────────────────────────────────────

    fn loop_add(
        &self,
        session_id: &SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop, String> {
        self.loop_add_calls.fetch_add(1, Ordering::SeqCst);
        match self.registered_loops.lock() {
            Ok(g) => {
                if let Some(first) = g.first() {
                    Ok(first.clone())
                } else {
                    Ok(RegisteredLoop {
                        id: "stub-loop".to_string(),
                        session_id: session_id.clone(),
                        prompt,
                        interval_seconds,
                        created_at: String::new(),
                        expires_at: String::new(),
                        last_run_at: None,
                    })
                }
            }
            Err(_) => Ok(RegisteredLoop {
                id: "stub-loop".to_string(),
                session_id: session_id.clone(),
                prompt,
                interval_seconds,
                created_at: String::new(),
                expires_at: String::new(),
                last_run_at: None,
            }),
        }
    }

    fn loop_cancel(&self, _id: &str) -> Result<bool, String> {
        self.loop_cancel_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.loop_cancel_result.load(Ordering::SeqCst))
    }

    fn loop_list(&self, _session_id: &SessionId) -> Vec<RegisteredLoop> {
        self.loop_list_calls.fetch_add(1, Ordering::SeqCst);
        match self.registered_loops.lock() {
            Ok(g) => g.clone(),
            Err(_) => Vec::new(),
        }
    }
}
