//! Embedded (in-process) [`FspecBackend`] implementation.
//!
//! Feature: spec/features/fspec-tui-embedded-backend.feature
//! Architecture note 6 (RPC-008): wraps `codelet_rpc_embedded::EmbeddedTransport`
//! and preserves the RPC-005 Q9 host-supplied-Handle invariant at the trait
//! boundary — `new` takes a non-defaulted `tokio::runtime::Handle` plus
//! `Arc<SharedFspecService>`.
//!
//! Construction immediately spawns the in-process tarpc server task on the
//! supplied runtime handle (via `EmbeddedTransport::client()`), so RPC method
//! bodies are one-line delegates to the cached client. Subscription methods
//! are zero-cost passthroughs to the underlying `EmbeddedTransport::*_rx`.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use codelet_rpc::{FspecServiceClient, SharedFspecService};
use codelet_rpc_embedded::EmbeddedTransport;
use codelet_rpc_types::{
    ApprovalChoice, BlocklistRuleInfo, ChangedFile, CheckpointCounts, CheckpointInfo,
    CompactionProgress, CompactionResult, CustomModelDefinition, FspecResult, HealthInfo,
    HistoryMatch, HitlRequest, HitlResponse, IncomingMessageInput, IsolatedSessionInfo, LogRecord,
    MergeOutcome, MergeStrategy, ModelEntry, ModelInfo, OAuthDeviceStart, OAuthHeadlessStart,
    PauseState, ProviderCredentialInfo, ProviderCredentialInput, ProviderInfo, RegisteredLoop,
    ScheduledJob, SessionChangesSummary, SessionId, SessionInfo, SessionModel, SessionStatus,
    SessionTokens, SessionWorktreeInfo, StreamChunk, TestConnectionResult, ThinkingConfig,
    ThinkingLevel, TokenRestoreState, WorkUnitContext, WorkUnitInfo, WorkspaceInfo,
};
use tarpc::context;
use tokio::sync::broadcast;

use super::FspecBackend;

/// In-process [`FspecBackend`] backed by `codelet_rpc_embedded::EmbeddedTransport`.
///
/// Holds the underlying transport (so subscription receivers stay alive)
/// alongside a cached tarpc client whose worker task was spawned at
/// construction time on the host-supplied runtime handle.
pub struct EmbeddedFspecBackend {
    transport: EmbeddedTransport,
    client: FspecServiceClient,
    /// RPC-037: cached reference to the underlying service so
    /// `status_changes_rx` can subscribe to the session manager's
    /// broadcast channel without going through tarpc.
    service: Arc<SharedFspecService>,
}

impl EmbeddedFspecBackend {
    /// Build an embedded backend bound to the supplied tokio runtime handle
    /// and shared service.
    ///
    /// The `handle` argument is intentionally NON-DEFAULTED so the
    /// RPC-005 Q9 invariant ("EmbeddedTransport requires a tokio Handle
    /// at construction") propagates to this trait boundary. See
    /// `rust/rpc-embedded/tests/architecture_invariants.rs::scenario_7_*`
    /// (widened by RPC-008 to scan `rust/fspec-tui/src/` too).
    pub fn new(handle: tokio::runtime::Handle, service: Arc<SharedFspecService>) -> Self {
        let transport = EmbeddedTransport::new(handle, Arc::clone(&service));
        let client = transport.client();
        Self {
            transport,
            client,
            service,
        }
    }
}

#[async_trait]
impl FspecBackend for EmbeddedFspecBackend {
    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>> {
        Ok(self.client.list_work_units(context::current()).await?)
    }

    async fn list_sessions(&self, project_path: String) -> Result<Vec<SessionInfo>> {
        Ok(self
            .client
            .list_sessions(context::current(), project_path)
            .await?)
    }

    async fn create_session(&self, role: Option<String>) -> Result<SessionId> {
        Ok(self.client.create_session(context::current(), role).await?)
    }

    async fn send_input(&self, id: SessionId, text: String) -> Result<()> {
        self.client.send_input(context::current(), id, text).await?;
        Ok(())
    }

    async fn interrupt(&self, id: SessionId) -> Result<()> {
        self.client.interrupt(context::current(), id).await?;
        Ok(())
    }

    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        self.transport.work_units_rx()
    }

    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.transport.chunks_rx()
    }

    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.transport.logs_rx()
    }

    async fn health(&self) -> Result<HealthInfo> {
        // RPC-011: embedded backend routes through the same tarpc
        // FspecService::health() method — both transports share the
        // single `FspecServiceImpl` implementation per RPC-005 rule.
        Ok(self.client.health(context::current()).await?)
    }

    async fn checkpoint_counts(&self) -> Result<CheckpointCounts> {
        // RPC-015: route through the shared tarpc method so the
        // embedded transport produces the same result as the WS
        // transport against the same SharedFspecService.
        Ok(self.client.checkpoint_counts(context::current()).await?)
    }

    async fn changed_files(&self) -> Result<Vec<ChangedFile>> {
        // RPC-355: one-line delegate to the shared tarpc method.
        Ok(self.client.changed_files(context::current()).await?)
    }

    async fn file_diff(&self, path: String) -> Result<Option<String>> {
        Ok(self.client.file_diff(context::current(), path).await?)
    }

    async fn list_checkpoints(&self) -> Result<Vec<CheckpointInfo>> {
        // RPC-362: one-line delegate to the shared tarpc method.
        Ok(self.client.list_checkpoints(context::current()).await?)
    }

    async fn checkpoint_diff_files(
        &self,
        work_unit_id: String,
        name: String,
    ) -> Result<Vec<ChangedFile>> {
        Ok(self
            .client
            .checkpoint_diff_files(context::current(), work_unit_id, name)
            .await?)
    }

    async fn checkpoint_file_diff(
        &self,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<Option<String>> {
        Ok(self
            .client
            .checkpoint_file_diff(context::current(), work_unit_id, name, path)
            .await?)
    }

    async fn restore_checkpoint_file(
        &self,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<()> {
        self.client
            .restore_checkpoint_file(context::current(), work_unit_id, name, path)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn restore_checkpoint_all(&self, work_unit_id: String, name: String) -> Result<()> {
        self.client
            .restore_checkpoint_all(context::current(), work_unit_id, name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_checkpoint(&self, work_unit_id: String, name: String) -> Result<()> {
        self.client
            .delete_checkpoint(context::current(), work_unit_id, name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_all_checkpoints(&self) -> Result<()> {
        self.client
            .delete_all_checkpoints(context::current())
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn move_work_unit_up(&self, id: String) -> Result<()> {
        // RPC-017: delegate to the shared tarpc method. The service
        // implementation maps any helper-level error to a `String` so
        // both transports surface identical diagnostics; we lift that
        // back into anyhow::Error here.
        self.client
            .move_work_unit_up(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn move_work_unit_down(&self, id: String) -> Result<()> {
        self.client
            .move_work_unit_down(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_model_info(&self, session_id: SessionId) -> Result<ModelInfo> {
        // RPC-018: one-line delegate to the shared tarpc method.
        Ok(self
            .client
            .get_model_info(context::current(), session_id)
            .await?)
    }

    async fn get_thinking_level(&self, session_id: SessionId) -> Result<ThinkingLevel> {
        Ok(self
            .client
            .get_thinking_level(context::current(), session_id)
            .await?)
    }

    async fn get_workspace_info(&self) -> Result<WorkspaceInfo> {
        Ok(self.client.get_workspace_info(context::current()).await?)
    }

    async fn search_files(&self, prefix: String, limit: u32) -> Result<Vec<String>> {
        // RPC-020: one-line delegate to the shared tarpc method.
        Ok(self
            .client
            .search_files(context::current(), prefix, limit)
            .await?)
    }

    async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()> {
        // RPC-025: one-line delegate to the shared tarpc method.
        self.client
            .persistence_add_history(context::current(), session, text)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn persistence_get_history(&self, session: SessionId, limit: u32) -> Result<Vec<String>> {
        self.client
            .persistence_get_history(context::current(), session, limit)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn persistence_search_history(&self, query: String) -> Result<Vec<HistoryMatch>> {
        self.client
            .persistence_search_history(context::current(), query)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn persistence_delete_session(&self, id: SessionId) -> Result<()> {
        // RPC-026: one-line delegate to the shared tarpc method.
        self.client
            .persistence_delete_session(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn list_providers(&self) -> Result<Vec<ProviderInfo>> {
        // RPC-022: one-line delegate to the shared tarpc method.
        Ok(self.client.list_providers(context::current()).await?)
    }

    async fn set_session_model(
        &self,
        session_id: SessionId,
        provider_id: String,
        model_id: String,
    ) -> Result<()> {
        // RPC-022: one-line delegate, lifting the String error into anyhow.
        self.client
            .set_session_model(context::current(), session_id, provider_id, model_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// PROV-118: set the in-process default model — one-line delegate.
    async fn set_default_model(&self, model: String) -> Result<()> {
        self.client
            .set_default_model(context::current(), model)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // RPC-347: custom-model write surface — one-line delegates.
    async fn add_custom_model(
        &self,
        provider_id: String,
        profile_name: String,
        definition: CustomModelDefinition,
    ) -> Result<()> {
        self.client
            .add_custom_model(context::current(), provider_id, profile_name, definition)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn update_custom_model(
        &self,
        provider_id: String,
        profile_name: String,
        original_model_id: String,
        definition: CustomModelDefinition,
    ) -> Result<()> {
        self.client
            .update_custom_model(
                context::current(),
                provider_id,
                profile_name,
                original_model_id,
                definition,
            )
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_custom_model(
        &self,
        provider_id: String,
        profile_name: String,
        model_id: String,
    ) -> Result<()> {
        self.client
            .delete_custom_model(context::current(), provider_id, profile_name, model_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // PROV-109: profile write surface — one-line delegates.
    async fn save_profile(
        &self,
        provider_id: String,
        profile_name: String,
        definition: codelet_rpc_types::ProfileDefinition,
    ) -> Result<()> {
        self.client
            .save_profile(context::current(), provider_id, profile_name, definition)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_profile(&self, provider_id: String, profile_name: String) -> Result<()> {
        self.client
            .delete_profile(context::current(), provider_id, profile_name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // PROV-136: rename delegate.
    async fn rename_profile(
        &self,
        provider_id: String,
        old_name: String,
        new_name: String,
        definition: codelet_rpc_types::ProfileDefinition,
    ) -> Result<()> {
        self.client
            .rename_profile(
                context::current(),
                provider_id,
                old_name,
                new_name,
                definition,
            )
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn set_thinking_level(&self, session_id: SessionId, level: ThinkingLevel) -> Result<()> {
        self.client
            .set_thinking_level(context::current(), session_id, level)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// RPC-037: per-user default thinking level — routed through the
    /// matching tarpc method now that the gap noted in the attachment
    /// (set_thinking_level_default missing from FspecService) is closed.
    async fn set_thinking_level_default(
        &self,
        session_id: SessionId,
        level: ThinkingLevel,
    ) -> Result<()> {
        self.client
            .set_thinking_level_default(context::current(), session_id, level)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // CONT-002: auto-continue state — routed through the matching tarpc
    // methods (mirrors set_thinking_level in shape).
    async fn set_continue_state(
        &self,
        session_id: SessionId,
        enabled: bool,
        budget: u32,
    ) -> Result<()> {
        self.client
            .set_continue_state(context::current(), session_id, enabled, budget)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_continue_state(&self, session_id: SessionId) -> Result<(bool, u32)> {
        Ok(self
            .client
            .get_continue_state(context::current(), session_id)
            .await?)
    }

    // CONT-003: goal chrome state — routed through the matching tarpc
    // methods (mirrors set_continue_state in shape).
    async fn set_goal_state(
        &self,
        session_id: SessionId,
        goal: Option<(String, Option<String>)>,
    ) -> Result<()> {
        self.client
            .set_goal_state(context::current(), session_id, goal)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_goal_state(
        &self,
        session_id: SessionId,
    ) -> Result<Option<(String, Option<String>)>> {
        Ok(self
            .client
            .get_goal_state(context::current(), session_id)
            .await?)
    }

    async fn get_session_role(&self, session_id: SessionId) -> Result<Option<String>> {
        Ok(self
            .client
            .get_session_role(context::current(), session_id)
            .await?)
    }

    async fn set_session_role(&self, session_id: SessionId, role: Option<String>) -> Result<()> {
        self.client
            .set_session_role(context::current(), session_id, role)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // ========================================================================
    // RPC-037: one-line delegates for the widened tarpc surface.
    // ========================================================================

    async fn send_input_with_thinking(
        &self,
        session_id: SessionId,
        text: String,
        thinking: Option<ThinkingConfig>,
    ) -> Result<()> {
        self.client
            .send_input_with_thinking(context::current(), session_id, text, thinking)
            .await?;
        Ok(())
    }

    async fn get_session_tokens(&self, session_id: SessionId) -> Result<SessionTokens> {
        Ok(self
            .client
            .get_session_tokens(context::current(), session_id)
            .await?)
    }

    async fn get_session_model(&self, session_id: SessionId) -> Result<SessionModel> {
        Ok(self
            .client
            .get_session_model(context::current(), session_id)
            .await?)
    }

    async fn get_compaction_progress(
        &self,
        session_id: SessionId,
    ) -> Result<Option<CompactionProgress>> {
        Ok(self
            .client
            .get_compaction_progress(context::current(), session_id)
            .await?)
    }

    async fn get_buffered_output(
        &self,
        session_id: SessionId,
        limit: u32,
    ) -> Result<Vec<StreamChunk>> {
        Ok(self
            .client
            .get_buffered_output(context::current(), session_id, limit)
            .await?)
    }

    async fn clear_history(&self, session_id: SessionId) -> Result<()> {
        self.client
            .clear_history(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn compact_session(&self, session_id: SessionId) -> Result<CompactionResult> {
        self.client
            .compact_session(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn restore_session_messages(
        &self,
        session_id: SessionId,
        envelopes: Vec<String>,
    ) -> Result<()> {
        self.client
            .restore_session_messages(context::current(), session_id, envelopes)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn restore_session_token_state(
        &self,
        session_id: SessionId,
        state: TokenRestoreState,
    ) -> Result<()> {
        self.client
            .restore_session_token_state(context::current(), session_id, state)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn resume_session(&self, session_id: SessionId) -> Result<()> {
        self.client
            .resume_session(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_work_unit_context(
        &self,
        session_id: SessionId,
    ) -> Result<Option<WorkUnitContext>> {
        Ok(self
            .client
            .get_work_unit_context(context::current(), session_id)
            .await?)
    }

    async fn set_work_unit_context(
        &self,
        session_id: SessionId,
        context: Option<WorkUnitContext>,
    ) -> Result<()> {
        self.client
            .set_work_unit_context(context::current(), session_id, context)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_pending_input(&self, session_id: SessionId) -> Result<Option<String>> {
        Ok(self
            .client
            .get_pending_input(context::current(), session_id)
            .await?)
    }

    async fn set_pending_input(&self, session_id: SessionId, text: Option<String>) -> Result<()> {
        self.client
            .set_pending_input(context::current(), session_id, text)
            .await?;
        Ok(())
    }

    async fn set_active_session(&self, session_id: SessionId) -> Result<()> {
        self.client
            .set_active_session(context::current(), session_id)
            .await?;
        Ok(())
    }

    async fn clear_active_session(&self) -> Result<()> {
        self.client.clear_active_session(context::current()).await?;
        Ok(())
    }

    async fn get_active_session(&self) -> Result<Option<SessionId>> {
        Ok(self.client.get_active_session(context::current()).await?)
    }

    async fn get_effective_cwd(&self, session_id: SessionId) -> Result<String> {
        Ok(self
            .client
            .get_effective_cwd(context::current(), session_id)
            .await?)
    }

    async fn get_supervisors(&self, session_id: SessionId) -> Result<Vec<SessionId>> {
        Ok(self
            .client
            .get_supervisors(context::current(), session_id)
            .await?)
    }

    async fn add_supervisor(
        &self,
        subordinate_id: SessionId,
        supervisor_id: SessionId,
    ) -> Result<()> {
        self.client
            .add_supervisor(context::current(), subordinate_id, supervisor_id)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn remove_supervisor(&self, supervisor_id: SessionId) -> Result<()> {
        self.client
            .remove_supervisor(context::current(), supervisor_id)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn get_subordinate(&self, supervisor_id: SessionId) -> Result<Option<SessionId>> {
        Ok(self
            .client
            .get_subordinate(context::current(), supervisor_id)
            .await?)
    }

    async fn get_subordinates(&self, supervisor_id: SessionId) -> Result<Vec<SessionId>> {
        Ok(self
            .client
            .get_subordinates(context::current(), supervisor_id)
            .await?)
    }

    async fn receive_incoming_message(
        &self,
        subordinate_id: SessionId,
        message: IncomingMessageInput,
    ) -> Result<()> {
        self.client
            .receive_incoming_message(context::current(), subordinate_id, message)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn get_debug_enabled(&self, session_id: SessionId) -> Result<bool> {
        Ok(self
            .client
            .get_debug_enabled(context::current(), session_id)
            .await?)
    }

    async fn set_debug_enabled(&self, session_id: SessionId, enabled: bool) -> Result<()> {
        self.client
            .set_debug_enabled(context::current(), session_id, enabled)
            .await?;
        Ok(())
    }

    async fn toggle_debug(&self, session_id: SessionId, debug_dir: String) -> Result<String> {
        self.client
            .toggle_debug(context::current(), session_id, debug_dir)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn set_debug_directory(&self, path: String) -> Result<()> {
        self.client
            .set_debug_directory(context::current(), path)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn pause_resume(&self, session_id: SessionId) -> Result<()> {
        self.client
            .pause_resume(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn pause_confirm(&self, session_id: SessionId, accept: bool) -> Result<()> {
        self.client
            .pause_confirm(context::current(), session_id, accept)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn pause_triple(&self, session_id: SessionId, choice: ApprovalChoice) -> Result<()> {
        self.client
            .pause_triple(context::current(), session_id, choice)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn send_hitl_response(
        &self,
        session_id: SessionId,
        response: HitlResponse,
    ) -> Result<()> {
        self.client
            .send_hitl_response(context::current(), session_id, response)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_pause_state(&self, session_id: SessionId) -> Result<Option<PauseState>> {
        Ok(self
            .client
            .get_pause_state(context::current(), session_id)
            .await?)
    }

    async fn get_hitl_request(&self, session_id: SessionId) -> Result<Option<HitlRequest>> {
        Ok(self
            .client
            .get_hitl_request(context::current(), session_id)
            .await?)
    }

    async fn send_fspec_result(&self, session_id: SessionId, result: FspecResult) -> Result<()> {
        self.client
            .send_fspec_result(context::current(), session_id, result)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn create_isolated_session(&self, role: Option<String>) -> Result<IsolatedSessionInfo> {
        self.client
            .create_isolated_session(context::current(), role)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn destroy_session(&self, session_id: SessionId) -> Result<()> {
        self.client
            .destroy_session(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // ========================================================================
    // RPC-054: Provider credentials surface — one-line forwarders.
    // ========================================================================

    async fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialInfo>> {
        Ok(self
            .client
            .list_provider_credentials(context::current())
            .await?)
    }

    async fn get_provider_credential(
        &self,
        provider_id: String,
    ) -> Result<Option<ProviderCredentialInfo>> {
        Ok(self
            .client
            .get_provider_credential(context::current(), provider_id)
            .await?)
    }

    async fn set_provider_credentials(
        &self,
        provider_id: String,
        creds: ProviderCredentialInput,
    ) -> Result<()> {
        self.client
            .set_provider_credentials(context::current(), provider_id, creds)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_provider_credentials(&self, provider_id: String) -> Result<()> {
        self.client
            .delete_provider_credentials(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn test_provider_connection(&self, provider_id: String) -> Result<TestConnectionResult> {
        self.client
            .test_provider_connection(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn refresh_models_cache(&self, provider_id: String) -> Result<Vec<ModelEntry>> {
        self.client
            .refresh_models_cache(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn oauth_clear_tokens(&self, provider_id: String) -> Result<()> {
        // PROV-112: napi-direct via the providers-backed RPC method.
        self.client
            .oauth_clear_tokens(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn oauth_get_tokens(&self, provider_id: String) -> Result<bool> {
        self.client
            .oauth_get_tokens(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    fn supports_browser_oauth(&self) -> bool {
        // PROV-113: the providers-layer local HTTP server runs in-process on
        // the embedded transport, so the browser login rows are available.
        true
    }

    async fn oauth_browser_login(&self, provider_id: String) -> Result<()> {
        self.client
            .oauth_browser_login(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn oauth_headless_start(&self, provider_id: String) -> Result<OAuthHeadlessStart> {
        self.client
            .oauth_headless_start(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn oauth_headless_complete(
        &self,
        provider_id: String,
        code_with_state: String,
        pkce_verifier: String,
    ) -> Result<()> {
        self.client
            .oauth_headless_complete(
                context::current(),
                provider_id,
                code_with_state,
                pkce_verifier,
            )
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn oauth_device_start(&self, provider_id: String) -> Result<OAuthDeviceStart> {
        self.client
            .oauth_device_start(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn oauth_device_poll(
        &self,
        provider_id: String,
        device_auth_id: String,
        interval: u64,
    ) -> Result<()> {
        self.client
            .oauth_device_poll(context::current(), provider_id, device_auth_id, interval)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn oauth_copilot_device_start(
        &self,
        enterprise_host: Option<String>,
    ) -> Result<OAuthDeviceStart> {
        self.client
            .oauth_copilot_device_start(context::current(), enterprise_host)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>> {
        Ok(self.client.blocklist_list(context::current()).await?)
    }

    async fn merge_session_worktree(
        &self,
        session_id: SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeOutcome> {
        self.client
            .merge_session_worktree(context::current(), session_id, strategy)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn discard_session_worktree(&self, session_id: SessionId) -> Result<()> {
        self.client
            .discard_session_worktree(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn prune_orphaned_worktrees(&self) -> Result<Vec<String>> {
        self.client
            .prune_orphaned_worktrees(context::current())
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn list_session_worktrees(&self) -> Result<Vec<SessionWorktreeInfo>> {
        Ok(self
            .client
            .list_session_worktrees(context::current())
            .await?)
    }

    async fn inspect_session_changes(
        &self,
        session_id: SessionId,
    ) -> Result<SessionChangesSummary> {
        self.client
            .inspect_session_changes(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule
    // ─────────────────────────────────────────────────────────────────

    async fn schedule_add(&self, job: ScheduledJob) -> Result<ScheduledJob> {
        self.client
            .schedule_add(context::current(), job)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_list(&self) -> Result<Vec<ScheduledJob>> {
        Ok(self.client.schedule_list(context::current()).await?)
    }

    async fn schedule_pause(&self, name: String) -> Result<ScheduledJob> {
        self.client
            .schedule_pause(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_resume(&self, name: String) -> Result<ScheduledJob> {
        self.client
            .schedule_resume(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_remove(&self, name: String) -> Result<()> {
        self.client
            .schedule_remove(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop
    // ─────────────────────────────────────────────────────────────────

    async fn loop_add(
        &self,
        session_id: SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop> {
        self.client
            .loop_add(context::current(), session_id, interval_seconds, prompt)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn loop_cancel(&self, id: String) -> Result<bool> {
        self.client
            .loop_cancel(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn loop_list(&self, session_id: SessionId) -> Result<Vec<RegisteredLoop>> {
        Ok(self
            .client
            .loop_list(context::current(), session_id)
            .await?)
    }

    /// RPC-037: subscribe to the session manager's status broadcast.
    /// On the embedded path we read it directly from the
    /// `SharedFspecService`'s attached `SessionManagerHandle` so the
    /// receiver bypasses tarpc entirely (zero-cost path per RPC-002 §5.1).
    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        match self.service.session_manager() {
            Some(handle) => handle.status_changes_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// RPC-385: subscribe to the session manager's session-created broadcast.
    /// Read directly from the `SharedFspecService`'s attached
    /// `SessionManagerHandle` so the embedded TUI is notified of every newly
    /// created session — including spawned subordinates that bypass the
    /// TUI-initiated creation paths — bypassing tarpc entirely (the same
    /// zero-cost path `status_changes_rx` uses).
    fn session_created_rx(&self) -> broadcast::Receiver<codelet_rpc_types::SessionInfo> {
        match self.service.session_manager() {
            Some(handle) => handle.session_created_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// TUI-109: subscribe to the checkpoint-enumeration progress
    /// broadcast. The channel lives directly on the
    /// `SharedFspecService` (not the session manager), so the embedded
    /// path forwards it with zero cost — `FspecServiceImpl::
    /// list_checkpoints` publishes a frame per collected item while the
    /// final Vec still returns through the tarpc RPC.
    fn checkpoints_progress_rx(
        &self,
    ) -> broadcast::Receiver<codelet_rpc_types::CheckpointsProgress> {
        self.service.checkpoints_progress_rx()
    }
}
