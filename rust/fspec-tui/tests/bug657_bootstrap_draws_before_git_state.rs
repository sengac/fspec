//! BUG-657: the TUI bootstrap seeds the board without waiting for
//! git-derived values.
//!
//! Feature: spec/features/git-captures-read-each-tree-once-and-the-tui-draws-before-git-state.feature
//!
//! `App::run` draws its first frame only after `App::bootstrap` returns, so
//! a bootstrap that awaited the checkpoint counts and the workspace info
//! (git branch) kept the TUI blank while git was slow. `HeldGitBackend`
//! holds both calls back until the test releases them (a semaphore, never
//! a clock), so the order is exact.

mod common;

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::*;
use codelet_test_helpers::{deadlock_guard, TestResult};
use tokio::sync::{broadcast, Semaphore};

use common::MockBackend;

/// A MockBackend whose `checkpoint_counts` and `get_workspace_info` answer
/// only once the test adds a permit each.
struct HeldGitBackend {
    inner: MockBackend,
    release: Semaphore,
}

macro_rules! delegate {
    ($(async fn $method:ident($($argument:ident: $kind:ty),*) -> $output:ty;)*; $($extra:item)*) => {
        #[async_trait]
        impl FspecBackend for HeldGitBackend {
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
    async fn submit_input(id: SessionId, text: String, mode: InputMode) -> ();
    async fn configure_session_mcp_servers(id: SessionId, servers: Vec<SessionMcpServer>) -> Vec<SessionMcpServerTools>;
    async fn control_exec_stdin(id: SessionId, exec_session_id: String, control: ExecStdinControl) -> ();
    async fn set_session_model(session_id: SessionId, provider_id: String, model_id: String) -> ();
    async fn interrupt(id: SessionId) -> ();
    async fn health() -> HealthInfo;
    async fn move_work_unit_up(workspace: String, id: String) -> ();
    async fn move_work_unit_down(workspace: String, id: String) -> ();
    async fn get_model_info(session_id: SessionId) -> ModelInfo;
    async fn get_thinking_level(session_id: SessionId) -> ThinkingLevel;
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

    async fn checkpoint_counts(&self) -> Result<CheckpointCounts> {
        let counts = FspecBackend::checkpoint_counts(&self.inner).await;
        self.release.acquire().await?.forget();
        counts
    }

    async fn get_workspace_info(&self) -> Result<WorkspaceInfo> {
        let info = FspecBackend::get_workspace_info(&self.inner).await;
        self.release.acquire().await?.forget();
        info
    }
}

fn work_unit(id: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "bug".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

#[tokio::test]
async fn the_tui_bootstrap_seeds_the_board_without_waiting_for_git_derived_values() -> TestResult {
    // @step Given an App whose backend holds checkpoint_counts and get_workspace_info back
    let inner = MockBackend::new();
    inner.seed_work_units(vec![work_unit("BUG-657")]);
    inner.set_checkpoint_counts(CheckpointCounts { manual: 2, auto: 3 });
    let info = WorkspaceInfo {
        cwd: "/tmp/bug657".to_string(),
        git_branch: Some("main".to_string()),
    };
    inner.set_workspace_info(info.clone());
    let held = Arc::new(HeldGitBackend {
        inner,
        release: Semaphore::new(0),
    });
    let backend: Arc<dyn FspecBackend> = held.clone();
    let mut app = App::new(backend);

    // @step When App::bootstrap is invoked
    deadlock_guard("bootstrap with git calls held back", app.bootstrap()).await??;

    // @step Then it returns with the board seeded from list_work_units
    let ids: Vec<&str> = app
        .board_store()
        .work_units()
        .iter()
        .map(|unit| unit.id.as_str())
        .collect();
    assert_eq!(ids, ["BUG-657"]);
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts::default()
    );
    assert!(app.agent_view_store().workspace().is_none());

    // @step When the backend releases checkpoint_counts and get_workspace_info
    held.release.add_permits(2);

    // @step Then CheckpointCountsLoaded and WorkspaceInfoLoaded arrive on the action bus and fill the board header and the workspace footer
    let (mut counts_seen, mut info_seen) = (false, false);
    deadlock_guard("CheckpointCountsLoaded and WorkspaceInfoLoaded", async {
        while !(counts_seen && info_seen) {
            while let Some(action) = app.try_recv_action() {
                counts_seen |= matches!(action, Action::CheckpointCountsLoaded(_));
                info_seen |= matches!(action, Action::WorkspaceInfoLoaded(_));
                app.dispatch(action);
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 2, auto: 3 }
    );
    assert_eq!(app.agent_view_store().workspace(), Some(&info));
    Ok(())
}
