use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

use anyhow::Result;
use async_trait::async_trait;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::*;
use tokio::sync::{broadcast, Semaphore};

use super::MockBackend;

pub struct ContinueBackend {
    inner: MockBackend,
    pub state: Mutex<(bool, u32)>,
    pub writes: Mutex<Vec<(SessionId, bool, u32)>>,
    pub fail_get: AtomicBool,
    pub fail_set: AtomicBool,
    pub hold_read: AtomicBool,
    pub reads: AtomicUsize,
    pub release_read: Semaphore,
    pub hold_write: AtomicBool,
    pub release_write: Semaphore,
}

impl ContinueBackend {
    pub fn state(&self) -> (bool, u32) {
        *self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn writes(&self) -> Vec<(SessionId, bool, u32)> {
        self.writes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn new(state: (bool, u32)) -> Self {
        Self {
            inner: MockBackend::new(),
            state: Mutex::new(state),
            writes: Mutex::new(Vec::new()),
            fail_get: AtomicBool::new(false),
            fail_set: AtomicBool::new(false),
            hold_read: AtomicBool::new(false),
            reads: AtomicUsize::new(0),
            release_read: Semaphore::new(0),
            hold_write: AtomicBool::new(false),
            release_write: Semaphore::new(0),
        }
    }

    pub async fn wait_for_read(&self) {
        for _attempt in 0..1000 {
            if self.reads.load(Ordering::SeqCst) > 0 {
                return;
            }
            tokio::task::yield_now().await;
        }
        assert!(self.reads.load(Ordering::SeqCst) > 0, "getter started");
    }
}

macro_rules! delegate {
    ($(async fn $method:ident($($argument:ident: $kind:ty),*) -> $output:ty;)*; $($extra:item)*) => {
        #[async_trait]
        impl FspecBackend for ContinueBackend {
        $(async fn $method(&self, $($argument: $kind),*) -> Result<$output> {
            FspecBackend::$method(&self.inner, $($argument),*).await
        })*
        $($extra)*
        }
    };
}

delegate! {
        async fn list_work_units() -> Vec<WorkUnitInfo>;
        async fn list_sessions(project_path: String) -> Vec<SessionInfo>;
        async fn create_session(role: Option<String>) -> SessionId;
        async fn send_input(id: SessionId, text: String) -> ();
        async fn interrupt(id: SessionId) -> ();
        async fn health() -> HealthInfo;
        async fn checkpoint_counts() -> CheckpointCounts;
        async fn move_work_unit_up(id: String) -> ();
        async fn move_work_unit_down(id: String) -> ();
        async fn get_model_info(session_id: SessionId) -> ModelInfo;
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
        async fn set_session_model(session_id: SessionId, registry_provider: String, model_id: String) -> ();
    ;

    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        self.inner.work_units_rx()
    }

    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.inner.chunks_rx()
    }

    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.inner.logs_rx()
    }

    async fn get_continue_state(&self, _session_id: SessionId) -> Result<(bool, u32)> {
        let state = self.state();
        let fail = self.fail_get.load(Ordering::SeqCst);
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.hold_read.swap(false, Ordering::SeqCst) {
            self.release_read.acquire().await?.forget();
        }
        anyhow::ensure!(!fail, "scripted getter failure");
        Ok(state)
    }

    async fn set_continue_state(
        &self,
        session_id: SessionId,
        enabled: bool,
        budget: u32,
    ) -> Result<()> {
        self.writes.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push((session_id, enabled, budget));
        if self.hold_write.swap(false, Ordering::SeqCst) {
            self.release_write.acquire().await?.forget();
        }
        anyhow::ensure!(!self.fail_set.load(Ordering::SeqCst), "scripted setter failure");
        *self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = (enabled, budget);
        Ok(())
    }
}

pub async fn responses(app: &mut App) -> Vec<Action> {
    while let Some(task) = app.next_pending_task() {
        let result = task.await;
        assert!(result.is_ok(), "app task failed: {result:?}");
    }
    let mut actions = Vec::new();
    while let Some(action) = app.try_recv_action() {
        actions.push(action);
    }
    actions
}

pub async fn settle(app: &mut App) {
    loop {
        let actions = responses(app).await;
        if actions.is_empty() {
            return;
        }
        for action in actions {
            app.dispatch(action);
        }
    }
}
