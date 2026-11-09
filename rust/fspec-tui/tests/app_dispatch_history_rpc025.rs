//! RPC-025 — App::dispatch routing for Shift+↑/↓ history recall and
//! fire-and-forget submission persistence.
//!
//! Feature: spec/features/rpc025-app-dispatch.feature
//!
//! Drives the App::dispatch state machine with a programmable
//! HistoryMockBackend that scripts persistence_get_history return values
//! and records persistence_add_history call captures with optional
//! latches.

use codelet_test_helpers::OrFail;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{
    CheckpointCounts, HealthInfo, HistoryMatch, LogRecord, ModelInfo, ProviderInfo, SessionId,
    SessionInfo, StreamChunk, ThinkingLevel, WorkUnitInfo, WorkspaceInfo,
};
use tokio::sync::{broadcast, Notify};

/// Programmable fake backend for App::dispatch history tests.
/// Captures call counts and lets tests script the
/// `persistence_get_history` return value per-session.
pub struct HistoryMockBackend {
    work_units_tx: broadcast::Sender<Vec<WorkUnitInfo>>,
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    logs_tx: broadcast::Sender<LogRecord>,
    pub send_input_calls: Mutex<Vec<(SessionId, String)>>,
    /// BUG-331: every `submit_input` call with its delivery mode.
    pub submitted_inputs: Mutex<Vec<(SessionId, String, codelet_rpc_types::InputMode)>>,
    pub persistence_add_history_calls: Mutex<Vec<(SessionId, String)>>,
    pub persistence_get_history_calls: Mutex<Vec<(SessionId, u32)>>,
    /// Per-session scripted return for persistence_get_history.
    scripted_history: Mutex<HashMap<SessionId, Vec<String>>>,
    /// When `Some(notify)`, persistence_add_history blocks on `notify.notified().await`
    /// before completing — lets the latch test prove fire-and-forget.
    add_history_latch: Mutex<Option<Arc<Notify>>>,
}

impl Default for HistoryMockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryMockBackend {
    pub fn new() -> Self {
        let (work_units_tx, _) = broadcast::channel(8);
        let (chunks_tx, _) = broadcast::channel(8);
        let (logs_tx, _) = broadcast::channel(8);
        Self {
            work_units_tx,
            chunks_tx,
            logs_tx,
            send_input_calls: Mutex::new(Vec::new()),
            submitted_inputs: Mutex::new(Vec::new()),
            persistence_add_history_calls: Mutex::new(Vec::new()),
            persistence_get_history_calls: Mutex::new(Vec::new()),
            scripted_history: Mutex::new(HashMap::new()),
            add_history_latch: Mutex::new(None),
        }
    }

    pub fn script_history(
        &self,
        session: SessionId,
        entries: Vec<String>,
    ) -> Result<(), codelet_test_helpers::TestError> {
        self.scripted_history
            .lock()
            .or_fail("scripted_history mutex")?
            .insert(session, entries);
        Ok(())
    }

    pub fn install_add_history_latch(
        &self,
        latch: Arc<Notify>,
    ) -> Result<(), codelet_test_helpers::TestError> {
        *self.add_history_latch.lock().or_fail("latch mutex")? = Some(latch);
        Ok(())
    }

    pub fn get_history_call_count(&self) -> Result<usize, codelet_test_helpers::TestError> {
        Ok(self
            .persistence_get_history_calls
            .lock()
            .or_fail("get_history calls mutex")?
            .len())
    }
}

#[async_trait]
impl FspecBackend for HistoryMockBackend {
    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>> {
        Ok(Vec::new())
    }
    async fn list_sessions(&self, _project_path: String) -> Result<Vec<SessionInfo>> {
        Ok(Vec::new())
    }
    async fn create_session(&self, _role: Option<String>) -> Result<SessionId> {
        Ok(SessionId::new("s-mock"))
    }
    async fn send_input(&self, id: SessionId, text: String) -> Result<()> {
        self.send_input_calls
            .lock()
            .or_fail("send_input mutex")?
            .push((id, text));
        Ok(())
    }
    async fn submit_input(
        &self,
        id: SessionId,
        text: String,
        mode: codelet_rpc_types::InputMode,
    ) -> Result<()> {
        self.submitted_inputs
            .lock()
            .or_fail("submitted_inputs mutex")?
            .push((id, text, mode));
        Ok(())
    }
    async fn configure_session_mcp_servers(
        &self,
        id: SessionId,
        servers: Vec<codelet_rpc_types::SessionMcpServer>,
    ) -> Result<Vec<codelet_rpc_types::SessionMcpServerTools>> {
        let _ = (id, servers);
        Err(anyhow::anyhow!(
            "MCP server configuration is not available on this test backend"
        ))
    }
    async fn control_exec_stdin(
        &self,
        id: SessionId,
        exec_session_id: String,
        control: codelet_rpc_types::ExecStdinControl,
    ) -> Result<()> {
        Err(anyhow::anyhow!(
            "exec stdin control ({control:?} for {exec_session_id} in {}) is not available on this test backend",
            id.value
        ))
    }
    async fn interrupt(&self, _id: SessionId) -> Result<()> {
        Ok(())
    }
    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        self.work_units_tx.subscribe()
    }
    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.chunks_tx.subscribe()
    }
    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.logs_tx.subscribe()
    }
    async fn health(&self) -> Result<HealthInfo> {
        Ok(HealthInfo {
            uptime_secs: 0,
            connected_clients: 0,
            last_watcher_event_secs_ago: None,
            lag_chunks: 0,
            lag_logs: 0,
            lag_work_units: 0,
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
    async fn checkpoint_counts(&self) -> Result<CheckpointCounts> {
        Ok(CheckpointCounts::default())
    }
    async fn move_work_unit_up(&self, _workspace: String, _id: String) -> Result<()> {
        Ok(())
    }
    async fn move_work_unit_down(&self, _workspace: String, _id: String) -> Result<()> {
        Ok(())
    }
    async fn get_model_info(&self, _session_id: SessionId) -> Result<ModelInfo> {
        Ok(ModelInfo::default())
    }
    async fn get_thinking_level(&self, _session_id: SessionId) -> Result<ThinkingLevel> {
        Ok(ThinkingLevel::Off)
    }
    async fn get_thinking_options(
        &self,
        _session_id: codelet_rpc_types::SessionId,
    ) -> Result<codelet_rpc_types::ThinkingOptions> {
        Err(anyhow::anyhow!("this backend offers no thinking options"))
    }
    async fn get_workspace_info(&self) -> Result<WorkspaceInfo> {
        Ok(WorkspaceInfo::default())
    }
    async fn search_files(&self, _prefix: String, _limit: u32) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
    async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()> {
        let latch = self
            .add_history_latch
            .lock()
            .or_fail("latch mutex")?
            .clone();
        if let Some(notify) = latch {
            notify.notified().await;
        }
        self.persistence_add_history_calls
            .lock()
            .or_fail("add_history mutex")?
            .push((session, text));
        Ok(())
    }
    async fn persistence_get_history(&self, session: SessionId, limit: u32) -> Result<Vec<String>> {
        self.persistence_get_history_calls
            .lock()
            .or_fail("get_history mutex")?
            .push((session.clone(), limit));
        let scripted = self
            .scripted_history
            .lock()
            .or_fail("scripted_history mutex")?
            .get(&session)
            .cloned()
            .unwrap_or_default();
        Ok(scripted)
    }
    async fn persistence_search_history(&self, _query: String) -> Result<Vec<HistoryMatch>> {
        Ok(Vec::new())
    }
    async fn persistence_delete_session(&self, _id: SessionId) -> Result<()> {
        Ok(())
    }
    async fn list_providers(&self) -> Result<Vec<ProviderInfo>> {
        Ok(Vec::new())
    }
    async fn set_session_model(
        &self,
        _session_id: SessionId,
        _provider_id: String,
        _model_id: String,
    ) -> Result<()> {
        Ok(())
    }
    async fn set_thinking_level(
        &self,
        _session_id: SessionId,
        _level: ThinkingLevel,
    ) -> Result<()> {
        Ok(())
    }
    async fn get_session_role(&self, _session_id: SessionId) -> Result<Option<String>> {
        Ok(None)
    }
    async fn set_session_role(&self, _session_id: SessionId, _role: Option<String>) -> Result<()> {
        Ok(())
    }
}

fn fresh_app() -> (App, Arc<HistoryMockBackend>) {
    let mock = Arc::new(HistoryMockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}
#[path = "rpc025_history/recall.rs"]
mod recall;

#[path = "rpc025_history/submission.rs"]
mod submission;
