//! WebSocket [`FspecBackend`] implementation.
//!
//! Feature: spec/features/auto-reconnect-supervisor.feature
//! Architecture rule [21]: a SECOND constructor
//! `connect_with_supervisor(url, action_tx)` spawns a transport-layer
//! reconnect supervisor task that publishes `Action::Disconnected /
//! Reconnecting(n) / Reconnected` onto the App's action bus. The
//! existing `connect(url)` constructor stays as a no-supervisor
//! convenience for tests (RPC-005..RPC-010 signatures unchanged per
//! RPC-011 rule [16]).

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use codelet_rpc_server::{ws_client_connect, FspecWsClient};
use codelet_rpc_types::{
    ApprovalChoice, BlocklistRuleInfo, ChangedFile, CheckpointCounts, CheckpointInfo,
    CompactionProgress, CompactionResult, CustomModelDefinition, FspecResult, HealthInfo,
    HistoryMatch, HitlRequest, HitlResponse, IncomingMessageInput, IsolatedSessionInfo, LogRecord,
    MergeOutcome, MergeStrategy, ModelEntry, ModelInfo, PauseState, ProviderCredentialInfo,
    ProviderCredentialInput, ProviderInfo, RegisteredLoop, ScheduledJob, SessionChangesSummary,
    SessionId, SessionInfo, SessionModel, SessionStatus, SessionTokens, SessionWorktreeInfo,
    StreamChunk, TestConnectionResult, ThinkingConfig, ThinkingLevel, TokenRestoreState,
    WorkUnitContext, WorkUnitInfo, WorkspaceInfo,
};
use tarpc::context;
use tokio::sync::{broadcast, mpsc::UnboundedSender, Notify, RwLock};

use super::{BackendError, FspecBackend};
use crate::components::Action;

/// RPC-011 backoff schedule per rule [3] / scenario "Auto-reconnect backoff schedule":
/// 250ms → 500ms → 1s → 2s → 5s cap.
const BACKOFF_SCHEDULE_MS: &[u64] = &[250, 500, 1_000, 2_000, 5_000];

/// WebSocket-backed [`FspecBackend`].
///
/// RPC-011: the inner `FspecWsClient` is now held behind an
/// `Arc<RwLock<Option<FspecWsClient>>>` so the transport-layer
/// supervisor task can atomically swap it out on disconnect /
/// reconnect. When the slot is `None`, RPC trait methods return
/// `Err(BackendError::Disconnected)` so the UI renders the dialog
/// rather than panicking or hanging.
pub struct WebSocketFspecBackend {
    client: Arc<RwLock<Option<FspecWsClient>>>,
    /// RPC-011: manual-reconnect signal. When the user presses 'r'
    /// from the DisconnectDialog the App calls
    /// `FspecBackend::request_manual_reconnect()` which routes here
    /// and notifies the supervisor task to cancel its current backoff
    /// sleep and retry immediately (resetting the schedule).
    manual_reconnect: Arc<Notify>,
}

impl WebSocketFspecBackend {
    /// RPC-005..RPC-010 form: open a WebSocket connection to the
    /// fspec rpc-server at `url` and wrap it in a
    /// [`WebSocketFspecBackend`]. RPC-011 rule [16] preserves this
    /// signature unchanged.
    pub async fn connect(url: url::Url) -> Result<Self> {
        let (ws, _response) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .with_context(|| format!("connect_async failed for {url}"))?;
        let client = ws_client_connect(ws)
            .await
            .context("ws_client_connect failed to build FspecWsClient")?;
        Ok(Self {
            client: Arc::new(RwLock::new(Some(client))),
            manual_reconnect: Arc::new(Notify::new()),
        })
    }

    /// RPC-011 rule [21]: SECOND constructor — opens an initial
    /// WebSocket connection AND spawns a supervisor task that watches
    /// the connection for drops, attempts reconnect with exponential
    /// backoff (250ms → 500 → 1000 → 2000 → 5000 cap), and publishes
    /// `Action::Disconnected / Reconnecting(n) / Reconnected` onto the
    /// supplied `action_tx`.
    ///
    /// If the initial connect_async fails this method STILL returns
    /// Ok(Self) with a `None` client slot; the supervisor task drives
    /// the retry loop in the background so the App can render the
    /// DisconnectDialog from the very first attempt. Callers that
    /// require a successfully-established initial connection should
    /// use the legacy `connect()` constructor instead.
    pub async fn connect_with_supervisor(
        url: url::Url,
        action_tx: UnboundedSender<Action>,
    ) -> Result<Self> {
        // Best-effort initial connect — failure is fine; the supervisor
        // will retry. If it succeeds we seed the client slot so the
        // App's bootstrap RPCs can go through immediately.
        let initial_client = match tokio_tungstenite::connect_async(url.as_str()).await {
            Ok((ws, _response)) => ws_client_connect(ws).await.ok(),
            Err(_) => None,
        };

        let manual_reconnect = Arc::new(Notify::new());
        let initial_present = initial_client.is_some();
        let client_slot: Arc<RwLock<Option<FspecWsClient>>> = Arc::new(RwLock::new(initial_client));

        // If the initial connect failed, surface Action::Disconnected
        // immediately so the App renders the dialog from t=0.
        if !initial_present {
            let _ = action_tx.send(Action::Disconnected);
        }

        let supervisor_url = url.clone();
        let supervisor_client = Arc::clone(&client_slot);
        let supervisor_action_tx = action_tx.clone();
        let supervisor_manual = Arc::clone(&manual_reconnect);
        tokio::spawn(async move {
            run_supervisor(
                supervisor_url,
                supervisor_client,
                supervisor_action_tx,
                supervisor_manual,
                initial_present,
            )
            .await;
        });

        Ok(Self {
            client: client_slot,
            manual_reconnect,
        })
    }

    /// RPC-011 rule [21] / [22]: alternative supervisor wiring that
    /// constructs the backend via plain `connect()` FIRST so the App
    /// can be built around it via `App::new(Arc::new(backend))` per
    /// the RPC-010 source-shape contract, THEN attaches the reconnect
    /// supervisor using an `action_tx` cloned from the App
    /// (`app.action_tx_clone()`).
    ///
    /// Returns a [`SupervisorHandle`] that holds Arc clones of the
    /// backend's internal client slot and manual-reconnect Notify.
    /// Call [`SupervisorHandle::start`] AFTER the App exists, passing
    /// `app.action_tx_clone()`, to spawn the supervisor task.
    pub fn supervisor_handle(&self) -> SupervisorHandle {
        SupervisorHandle {
            client: Arc::clone(&self.client),
            manual_reconnect: Arc::clone(&self.manual_reconnect),
        }
    }
}

/// RPC-011 rule [21] / [22] handle to a not-yet-started reconnect
/// supervisor. Returned by [`WebSocketFspecBackend::supervisor_handle`]
/// before the App exists; the App-bound `start(url, action_tx)` call
/// spawns the supervisor task on the SAME `Arc<RwLock<Option<…>>>`
/// the backend itself holds, so reconnect-driven client-slot swaps
/// are visible to RPC method bodies through the existing backend
/// reference.
pub struct SupervisorHandle {
    client: Arc<RwLock<Option<FspecWsClient>>>,
    manual_reconnect: Arc<Notify>,
}

impl SupervisorHandle {
    /// Spawn the supervisor task. Publishes `Action::Disconnected /
    /// Reconnecting(n) / Reconnected / ManualReconnect-acknowledged`
    /// emissions onto `action_tx`. The supervisor holds Arc clones
    /// of the backend's client slot — so when the supervisor swaps
    /// in a fresh `FspecWsClient` after a successful reconnect, the
    /// backend's existing RPC method bodies (which take a `read()`
    /// guard on the SAME Arc) see the new client immediately.
    pub fn start(self, url: url::Url, action_tx: UnboundedSender<Action>) {
        let SupervisorHandle {
            client,
            manual_reconnect,
        } = self;
        tokio::spawn(async move {
            run_supervisor(url, client, action_tx, manual_reconnect, true).await;
        });
    }
}

#[async_trait]
impl FspecBackend for WebSocketFspecBackend {
    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().list_work_units(context::current()).await?)
    }

    async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().list_sessions(context::current()).await?)
    }

    async fn create_session(&self, role: Option<String>) -> Result<SessionId> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .create_session(context::current(), role)
            .await?)
    }

    async fn send_input(&self, id: SessionId, text: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .send_input(context::current(), id, text)
            .await?;
        Ok(())
    }

    async fn interrupt(&self, id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client.client().interrupt(context::current(), id).await?;
        Ok(())
    }

    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        // Best-effort: read the client slot synchronously via try_read.
        // If the supervisor is currently rebuilding the client this
        // returns a closed receiver — the App's subscriber task observes
        // RecvError::Closed and the supervisor's Action::Reconnected
        // triggers a fresh subscribe.
        match self.client.try_read() {
            Ok(guard) => match guard.as_ref() {
                Some(client) => client.work_units_rx(),
                None => empty_broadcast_rx(),
            },
            Err(_) => empty_broadcast_rx(),
        }
    }

    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        match self.client.try_read() {
            Ok(guard) => match guard.as_ref() {
                Some(client) => client.chunks_rx(),
                None => empty_broadcast_rx(),
            },
            Err(_) => empty_broadcast_rx(),
        }
    }

    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        match self.client.try_read() {
            Ok(guard) => match guard.as_ref() {
                Some(client) => client.logs_rx(),
                None => empty_broadcast_rx(),
            },
            Err(_) => empty_broadcast_rx(),
        }
    }

    /// RPC-037: subscribe to the WebSocket-pushed
    /// `Envelope::StatusUpdate` stream. When the client slot is `None`
    /// (disconnected) or the lock is contended, returns a degenerate
    /// receiver whose sender has been dropped — subscribers observe
    /// `RecvError::Closed` on the next `recv().await`, mirroring the
    /// chunks/logs degenerate-on-disconnect pattern.
    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        match self.client.try_read() {
            Ok(guard) => match guard.as_ref() {
                Some(client) => client.status_changes_rx(),
                None => empty_broadcast_rx(),
            },
            Err(_) => empty_broadcast_rx(),
        }
    }

    async fn health(&self) -> Result<HealthInfo> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().health(context::current()).await?)
    }

    async fn checkpoint_counts(&self) -> Result<CheckpointCounts> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .checkpoint_counts(context::current())
            .await?)
    }

    async fn changed_files(&self) -> Result<Vec<ChangedFile>> {
        // RPC-355: guarded delegate following the standard Disconnected pattern.
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().changed_files(context::current()).await?)
    }

    async fn file_diff(&self, path: String) -> Result<Option<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().file_diff(context::current(), path).await?)
    }

    async fn list_checkpoints(&self) -> Result<Vec<CheckpointInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().list_checkpoints(context::current()).await?)
    }

    async fn checkpoint_diff_files(
        &self,
        work_unit_id: String,
        name: String,
    ) -> Result<Vec<ChangedFile>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .checkpoint_diff_files(context::current(), work_unit_id, name)
            .await?)
    }

    async fn checkpoint_file_diff(
        &self,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<Option<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .checkpoint_file_diff(context::current(), work_unit_id, name, path)
            .await?)
    }

    async fn restore_checkpoint_file(
        &self,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .restore_checkpoint_file(context::current(), work_unit_id, name, path)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn restore_checkpoint_all(&self, work_unit_id: String, name: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .restore_checkpoint_all(context::current(), work_unit_id, name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_checkpoint(&self, work_unit_id: String, name: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .delete_checkpoint(context::current(), work_unit_id, name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_all_checkpoints(&self) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .delete_all_checkpoints(context::current())
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn move_work_unit_up(&self, id: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .move_work_unit_up(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn move_work_unit_down(&self, id: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .move_work_unit_down(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_model_info(&self, session_id: SessionId) -> Result<ModelInfo> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_model_info(context::current(), session_id)
            .await?)
    }

    async fn get_thinking_level(&self, session_id: SessionId) -> Result<ThinkingLevel> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_thinking_level(context::current(), session_id)
            .await?)
    }

    async fn get_workspace_info(&self) -> Result<WorkspaceInfo> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_workspace_info(context::current())
            .await?)
    }

    async fn search_files(&self, prefix: String, limit: u32) -> Result<Vec<String>> {
        // RPC-020: route through the shared tarpc method, returning
        // Disconnected when the supervisor has dropped the client slot.
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .search_files(context::current(), prefix, limit)
            .await?)
    }

    async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()> {
        // RPC-025: route through the shared tarpc method.
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .persistence_add_history(context::current(), session, text)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn persistence_get_history(&self, session: SessionId, limit: u32) -> Result<Vec<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .persistence_get_history(context::current(), session, limit)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn persistence_search_history(&self, query: String) -> Result<Vec<HistoryMatch>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .persistence_search_history(context::current(), query)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn persistence_delete_session(&self, id: SessionId) -> Result<()> {
        // RPC-026: route through the shared tarpc method, returning
        // Disconnected when the supervisor has dropped the client slot.
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .persistence_delete_session(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn list_providers(&self) -> Result<Vec<ProviderInfo>> {
        // RPC-022: route through the shared tarpc method.
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().list_providers(context::current()).await?)
    }

    async fn set_session_model(
        &self,
        session_id: SessionId,
        provider_id: String,
        model_id: String,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_session_model(context::current(), session_id, provider_id, model_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// PROV-118: set the in-process default model — same read().await +
    /// Disconnected guard pattern as the rest of the WebSocket backend.
    async fn set_default_model(&self, model: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_default_model(context::current(), model)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // RPC-347: custom-model write surface — same read().await + Disconnected
    // guard pattern as the rest of the WebSocket backend.
    async fn add_custom_model(
        &self,
        provider_id: String,
        profile_name: String,
        definition: CustomModelDefinition,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
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
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
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
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .delete_custom_model(context::current(), provider_id, profile_name, model_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // PROV-109: profile write surface — guarded delegates.
    async fn save_profile(
        &self,
        provider_id: String,
        profile_name: String,
        definition: codelet_rpc_types::ProfileDefinition,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .save_profile(context::current(), provider_id, profile_name, definition)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_profile(&self, provider_id: String, profile_name: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .delete_profile(context::current(), provider_id, profile_name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn set_thinking_level(&self, session_id: SessionId, level: ThinkingLevel) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
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
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_thinking_level_default(context::current(), session_id, level)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_session_role(&self, session_id: SessionId) -> Result<Option<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_session_role(context::current(), session_id)
            .await?)
    }

    async fn set_session_role(&self, session_id: SessionId, role: Option<String>) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_session_role(context::current(), session_id, role)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// RPC-011 rule [4]: notify the supervisor task to cancel its
    /// current backoff sleep, attempt connect immediately, and reset
    /// the backoff schedule on its next failure. Idempotent; safe to
    /// call even when the supervisor is not currently sleeping.
    fn request_manual_reconnect(&self) {
        self.manual_reconnect.notify_one();
    }

    // ========================================================================
    // RPC-037: Widened FspecBackend surface. Every method follows the
    // existing read-lock + BackendError::Disconnected guard pattern then
    // delegates to the matching tarpc method one line down.
    // ========================================================================

    async fn send_input_with_thinking(
        &self,
        session_id: SessionId,
        text: String,
        thinking: Option<ThinkingConfig>,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .send_input_with_thinking(context::current(), session_id, text, thinking)
            .await?;
        Ok(())
    }

    async fn get_session_tokens(&self, session_id: SessionId) -> Result<SessionTokens> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_session_tokens(context::current(), session_id)
            .await?)
    }

    async fn get_session_model(&self, session_id: SessionId) -> Result<SessionModel> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_session_model(context::current(), session_id)
            .await?)
    }

    async fn get_compaction_progress(
        &self,
        session_id: SessionId,
    ) -> Result<Option<CompactionProgress>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_compaction_progress(context::current(), session_id)
            .await?)
    }

    async fn get_buffered_output(
        &self,
        session_id: SessionId,
        limit: u32,
    ) -> Result<Vec<StreamChunk>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_buffered_output(context::current(), session_id, limit)
            .await?)
    }

    async fn clear_history(&self, session_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .clear_history(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn compact_session(&self, session_id: SessionId) -> Result<CompactionResult> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .compact_session(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn restore_session_messages(
        &self,
        session_id: SessionId,
        envelopes: Vec<String>,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .restore_session_messages(context::current(), session_id, envelopes)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn restore_session_token_state(
        &self,
        session_id: SessionId,
        state: TokenRestoreState,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .restore_session_token_state(context::current(), session_id, state)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn resume_session(&self, session_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .resume_session(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_work_unit_context(
        &self,
        session_id: SessionId,
    ) -> Result<Option<WorkUnitContext>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_work_unit_context(context::current(), session_id)
            .await?)
    }

    async fn set_work_unit_context(
        &self,
        session_id: SessionId,
        context: Option<WorkUnitContext>,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_work_unit_context(context::current(), session_id, context)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_pending_input(&self, session_id: SessionId) -> Result<Option<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_pending_input(context::current(), session_id)
            .await?)
    }

    async fn set_pending_input(&self, session_id: SessionId, text: Option<String>) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_pending_input(context::current(), session_id, text)
            .await?;
        Ok(())
    }

    async fn set_active_session(&self, session_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_active_session(context::current(), session_id)
            .await?;
        Ok(())
    }

    async fn clear_active_session(&self) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .clear_active_session(context::current())
            .await?;
        Ok(())
    }

    async fn get_active_session(&self) -> Result<Option<SessionId>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_active_session(context::current())
            .await?)
    }

    async fn get_effective_cwd(&self, session_id: SessionId) -> Result<String> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_effective_cwd(context::current(), session_id)
            .await?)
    }

    async fn get_supervisors(&self, session_id: SessionId) -> Result<Vec<SessionId>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_supervisors(context::current(), session_id)
            .await?)
    }

    async fn add_supervisor(
        &self,
        subordinate_id: SessionId,
        supervisor_id: SessionId,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .add_supervisor(context::current(), subordinate_id, supervisor_id)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn remove_supervisor(&self, supervisor_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .remove_supervisor(context::current(), supervisor_id)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn get_subordinate(&self, supervisor_id: SessionId) -> Result<Option<SessionId>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_subordinate(context::current(), supervisor_id)
            .await?)
    }

    async fn get_subordinates(&self, supervisor_id: SessionId) -> Result<Vec<SessionId>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_subordinates(context::current(), supervisor_id)
            .await?)
    }

    async fn receive_incoming_message(
        &self,
        subordinate_id: SessionId,
        message: IncomingMessageInput,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .receive_incoming_message(context::current(), subordinate_id, message)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn get_debug_enabled(&self, session_id: SessionId) -> Result<bool> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_debug_enabled(context::current(), session_id)
            .await?)
    }

    async fn set_debug_enabled(&self, session_id: SessionId, enabled: bool) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_debug_enabled(context::current(), session_id, enabled)
            .await?;
        Ok(())
    }

    async fn toggle_debug(&self, session_id: SessionId, debug_dir: String) -> Result<String> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .toggle_debug(context::current(), session_id, debug_dir)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn set_debug_directory(&self, path: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_debug_directory(context::current(), path)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn pause_resume(&self, session_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .pause_resume(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn pause_confirm(&self, session_id: SessionId, accept: bool) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .pause_confirm(context::current(), session_id, accept)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn pause_triple(&self, session_id: SessionId, choice: ApprovalChoice) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .pause_triple(context::current(), session_id, choice)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn send_hitl_response(
        &self,
        session_id: SessionId,
        response: HitlResponse,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .send_hitl_response(context::current(), session_id, response)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_pause_state(&self, session_id: SessionId) -> Result<Option<PauseState>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_pause_state(context::current(), session_id)
            .await?)
    }

    async fn get_hitl_request(&self, session_id: SessionId) -> Result<Option<HitlRequest>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_hitl_request(context::current(), session_id)
            .await?)
    }

    async fn send_fspec_result(&self, session_id: SessionId, result: FspecResult) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .send_fspec_result(context::current(), session_id, result)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn create_isolated_session(&self, role: Option<String>) -> Result<IsolatedSessionInfo> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .create_isolated_session(context::current(), role)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn destroy_session(&self, session_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .destroy_session(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // ========================================================================
    // RPC-054: Provider credentials surface — one-line forwarders with the
    // standard `Disconnected` guard pattern used by every other WebSocket
    // RPC method.
    // ========================================================================

    async fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .list_provider_credentials(context::current())
            .await?)
    }

    async fn get_provider_credential(
        &self,
        provider_id: String,
    ) -> Result<Option<ProviderCredentialInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_provider_credential(context::current(), provider_id)
            .await?)
    }

    async fn set_provider_credentials(
        &self,
        provider_id: String,
        creds: ProviderCredentialInput,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .set_provider_credentials(context::current(), provider_id, creds)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_provider_credentials(&self, provider_id: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .delete_provider_credentials(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn test_provider_connection(&self, provider_id: String) -> Result<TestConnectionResult> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .test_provider_connection(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn refresh_models_cache(&self, provider_id: String) -> Result<Vec<ModelEntry>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .refresh_models_cache(context::current(), provider_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().blocklist_list(context::current()).await?)
    }

    async fn merge_session_worktree(
        &self,
        session_id: SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeOutcome> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .merge_session_worktree(context::current(), session_id, strategy)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn discard_session_worktree(&self, session_id: SessionId) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .discard_session_worktree(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn prune_orphaned_worktrees(&self) -> Result<Vec<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .prune_orphaned_worktrees(context::current())
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn list_session_worktrees(&self) -> Result<Vec<SessionWorktreeInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .list_session_worktrees(context::current())
            .await?)
    }

    async fn inspect_session_changes(
        &self,
        session_id: SessionId,
    ) -> Result<SessionChangesSummary> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .inspect_session_changes(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule.
    // ─────────────────────────────────────────────────────────────────

    async fn schedule_add(&self, job: ScheduledJob) -> Result<ScheduledJob> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .schedule_add(context::current(), job)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_list(&self) -> Result<Vec<ScheduledJob>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().schedule_list(context::current()).await?)
    }

    async fn schedule_pause(&self, name: String) -> Result<ScheduledJob> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .schedule_pause(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_resume(&self, name: String) -> Result<ScheduledJob> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .schedule_resume(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_remove(&self, name: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .schedule_remove(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop.
    // ─────────────────────────────────────────────────────────────────

    async fn loop_add(
        &self,
        session_id: SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .loop_add(context::current(), session_id, interval_seconds, prompt)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn loop_cancel(&self, id: String) -> Result<bool> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .loop_cancel(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn loop_list(&self, session_id: SessionId) -> Result<Vec<RegisteredLoop>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .loop_list(context::current(), session_id)
            .await?)
    }
}

/// Tiny helper that returns a broadcast receiver from a freshly-created
/// channel whose sender has already been dropped, so the returned
/// receiver yields `RecvError::Closed` on first poll. Used by the
/// `*_rx` accessors when the inner client slot is `None`.
fn empty_broadcast_rx<T: Clone + Send + 'static>() -> broadcast::Receiver<T> {
    let (tx, rx) = broadcast::channel::<T>(1);
    drop(tx);
    rx
}

/// RPC-011 supervisor task per architecture note [0]: drives the
/// auto-reconnect lifecycle for a single `WebSocketFspecBackend`.
///
/// `had_initial_connection` controls the first iteration: when the
/// initial connect succeeded the supervisor waits for the connection
/// to drop BEFORE emitting Action::Disconnected; when it failed the
/// connect_with_supervisor caller already emitted Disconnected and we
/// jump straight into the backoff loop.
async fn run_supervisor(
    url: url::Url,
    client_slot: Arc<RwLock<Option<FspecWsClient>>>,
    action_tx: UnboundedSender<Action>,
    manual_reconnect: Arc<Notify>,
    had_initial_connection: bool,
) {
    if had_initial_connection {
        // Wait until the initial client's chunks_rx broadcast closes.
        let initial_chunks_rx = {
            let guard = client_slot.read().await;
            guard
                .as_ref()
                .map(codelet_rpc_server::FspecWsClient::chunks_rx)
        };
        if let Some(mut rx) = initial_chunks_rx {
            loop {
                match rx.recv().await {
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
        // Connection lost. Clear the slot and publish Disconnected.
        {
            let mut guard = client_slot.write().await;
            *guard = None;
        }
        if action_tx.send(Action::Disconnected).is_err() {
            return;
        }
    }

    // Reconnect loop with exponential backoff.
    let mut attempt: u32 = 0;
    loop {
        attempt = attempt.saturating_add(1);
        // Use index = min(attempt-1, last) so attempt 5..N stays at the
        // 5000ms cap per the data-table scenario.
        let idx = (attempt as usize)
            .saturating_sub(1)
            .min(BACKOFF_SCHEDULE_MS.len() - 1);
        let delay = Duration::from_millis(BACKOFF_SCHEDULE_MS[idx]);

        if action_tx.send(Action::Reconnecting(attempt)).is_err() {
            return;
        }

        // Sleep, but allow manual-reconnect to short-circuit.
        tokio::select! {
            _ = tokio::time::sleep(delay) => {}
            _ = manual_reconnect.notified() => {
                // Manual reconnect resets the schedule per rule [4] /
                // scenario "Pressing r ... resets backoff".
                attempt = 0;
            }
        }

        // Attempt connect_async.
        let connect_result = tokio_tungstenite::connect_async(url.as_str()).await;
        let Ok((ws, _)) = connect_result else {
            // Failed — loop and try again with longer backoff.
            continue;
        };
        let new_client = match ws_client_connect(ws).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Subscribe to chunks before swapping in the new client so we
        // don't miss the connection-alive signal.
        let mut rx = new_client.chunks_rx();
        {
            let mut guard = client_slot.write().await;
            *guard = Some(new_client);
        }

        // RPC-011 rule [5]: re-issue bootstrap on the new client. We
        // delegate this to the App by emitting Action::Reconnected —
        // the App.dispatch() handler pops the dialog and triggers a
        // fresh bootstrap via the existing RPC-009 mechanism.
        if action_tx.send(Action::Reconnected).is_err() {
            return;
        }

        // Re-arm the connection-alive watcher for the next drop.
        loop {
            match rx.recv().await {
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }

        // Connection dropped again — clear the slot, publish
        // Disconnected, restart the backoff loop.
        {
            let mut guard = client_slot.write().await;
            *guard = None;
        }
        if action_tx.send(Action::Disconnected).is_err() {
            return;
        }
        attempt = 0;
    }
}
