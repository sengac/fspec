use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Mutex,
};

use anyhow::Result;
use async_trait::async_trait;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::*;
use tokio::sync::{broadcast, Semaphore};

use crate::common::MockBackend;

pub struct ModelBackend {
    inner: MockBackend,
    model: Mutex<String>,
    pub fail_read: AtomicBool,
    pub hold_write: AtomicBool,
    pub hold_read: AtomicBool,
    pub started: AtomicUsize,
    pub release: Semaphore,
    pub writes: Mutex<Vec<String>>,
}

impl ModelBackend {
    pub fn new() -> Self {
        Self {
            inner: MockBackend::new(),
            model: Mutex::new("A".into()),
            fail_read: AtomicBool::new(false),
            hold_write: AtomicBool::new(false),
            hold_read: AtomicBool::new(false),
            started: AtomicUsize::new(0),
            release: Semaphore::new(0),
            writes: Mutex::new(Vec::new()),
        }
    }

    fn model(&self) -> String {
        self.model
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub async fn wait_started(&self) {
        for _attempt in 0..1000 {
            if self.started.load(Ordering::SeqCst) > 0 {
                return;
            }
            tokio::task::yield_now().await;
        }
        assert!(self.started.load(Ordering::SeqCst) > 0);
    }
}

macro_rules! delegate {
    ($(async fn $method:ident($($argument:ident: $kind:ty),*) -> $output:ty;)*; $($extra:item)*) => {
        #[async_trait]
        impl FspecBackend for ModelBackend {
        $(async fn $method(&self, $($argument: $kind),*) -> Result<$output> {
            FspecBackend::$method(&self.inner, $($argument),*).await
        })*
        $($extra)*
        }
    };
}

delegate! {
    async fn list_work_units() -> Vec<WorkUnitInfo>;
    async fn create_session(role: Option<String>) -> SessionId;
    async fn send_input(id: SessionId, text: String) -> ();
    async fn interrupt(id: SessionId) -> ();
    async fn health() -> HealthInfo;
    async fn checkpoint_counts() -> CheckpointCounts;
    async fn move_work_unit_up(id: String) -> ();
    async fn move_work_unit_down(id: String) -> ();
    async fn get_thinking_level(session_id: SessionId) -> ThinkingLevel;
    async fn get_workspace_info() -> WorkspaceInfo;
    async fn search_files(prefix: String, limit: u32) -> Vec<String>;
    async fn persistence_add_history(session: SessionId, text: String) -> ();
    async fn persistence_get_history(session: SessionId, limit: u32) -> Vec<String>;
    async fn persistence_search_history(query: String) -> Vec<HistoryMatch>;
    async fn persistence_delete_session(id: SessionId) -> ();
    async fn list_providers() -> Vec<ProviderInfo>;
    async fn set_thinking_level(session_id: SessionId, level: ThinkingLevel) -> ();
    async fn get_session_role(session_id: SessionId) -> Option<String>;
    async fn set_session_role(session_id: SessionId, role: Option<String>) -> ();
    ;
    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> { self.inner.work_units_rx() }
    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> { self.inner.chunks_rx() }
    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> { self.inner.logs_rx() }

    async fn list_sessions(&self, _project: String) -> Result<Vec<SessionInfo>> {
        anyhow::ensure!(!self.fail_read.load(Ordering::SeqCst), "metadata unavailable");
        Ok(vec![SessionInfo {
            id: "origin".into(), name: "origin".into(), status: "idle".into(), project: String::new(),
            message_count: 0, provider_id: Some("openai".into()), model_id: Some(self.model()),
            is_isolated: false, worktree_path: None, role: None, updated_at_ms: None,
            work_unit_focus: Default::default(),
        }])
    }

    async fn get_model_info(&self, _session: SessionId) -> Result<ModelInfo> {
        let model = self.model();
        if self.hold_read.swap(false, Ordering::SeqCst) {
            self.started.fetch_add(1, Ordering::SeqCst);
            self.release.acquire().await?.forget();
        }
        anyhow::ensure!(!self.fail_read.load(Ordering::SeqCst), "metadata unavailable");
        Ok(ModelInfo { display_name: model, ..ModelInfo::default() })
    }

    async fn set_session_model(&self, _session: SessionId, _provider: String, model: String) -> Result<()> {
        self.writes.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(model.clone());
        if self.hold_write.swap(false, Ordering::SeqCst) {
            self.started.fetch_add(1, Ordering::SeqCst);
            self.release.acquire().await?.forget();
        }
        anyhow::ensure!(model != "reject", "session busy");
        *self.model.lock().unwrap_or_else(std::sync::PoisonError::into_inner) =
            if model == "alias" { "B".into() } else { model };
        Ok(())
    }
}

pub async fn responses(app: &mut App) -> Vec<Action> {
    while let Some(task) = app.next_pending_task() {
        assert!(task.await.is_ok());
    }
    let mut actions = Vec::new();
    while let Some(action) = app.try_recv_action() {
        actions.push(action);
    }
    actions
}

pub async fn settle(app: &mut App) {
    for action in responses(app).await {
        app.dispatch(action);
    }
}
