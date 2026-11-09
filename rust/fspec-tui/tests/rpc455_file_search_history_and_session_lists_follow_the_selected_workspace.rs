//! RPC-455 — @file search, input history and the session lists follow the
//! selected workspace; model sync looks the session up in its own project.
//!
//! Feature: spec/features/file-search-history-and-session-lists-follow-the-selected-workspace.feature

mod common;

use crate::common::TaskOutcome;
use codelet_test_helpers::TestResult;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{Action, App};
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
use codelet_rpc_types::{SessionId, WorkspaceEntry};
use common::workspace_mock::session_in;
use common::MockBackend;
use serial_test::serial;

fn git(cwd: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let out = Command::new("git").args(args).current_dir(cwd).output()?;
    if !out.status.success() {
        return Err(format!("git {args:?}: {}", String::from_utf8_lossy(&out.stderr)).into());
    }
    Ok(())
}

/// A committed fspec workspace `<root>/<name>` holding `files`.
fn workspace(
    root: &Path,
    name: &str,
    files: &[&str],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = root.join(name);
    std::fs::create_dir_all(path.join("spec"))?;
    std::fs::write(path.join("spec/work-units.json"), r#"{"workUnits":{}}"#)?;
    for file in files {
        let target = path.join(file);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(target, "// fixture\n")?;
    }
    git(&path, &["init", "-q", "-b", "main"])?;
    git(&path, &["config", "user.email", "test@example.com"])?;
    git(&path, &["config", "user.name", "Test"])?;
    git(&path, &["add", "."])?;
    git(&path, &["commit", "-q", "-m", "initial"])?;
    Ok(std::fs::canonicalize(path)?)
}

fn daemon(start: &Path) -> Result<FspecServiceImpl, Box<dyn std::error::Error>> {
    let shared = Arc::new(
        SharedFspecService::new(Arc::new(WorkUnitsWatcher::new(start)?))
            .with_cwd(start.to_path_buf()),
    );
    Ok(FspecServiceImpl::new(shared))
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn file_search_over_rpc_searches_the_named_workspace() -> TestResult {
    // @step Given a daemon started in workspace "alpha" and a second workspace "beta" holding "src/beta_only.rs"
    let data = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let root = tempfile::tempdir()?;
    let alpha = workspace(root.path(), "alpha", &["src/alpha_only.rs"])?;
    let beta = workspace(root.path(), "beta", &["src/beta_only.rs"])?;
    let service = daemon(&alpha)?;
    // @step When files starting with "src/beta" are searched in "beta" over RPC
    let found = service
        .search_files_in_workspace(
            tarpc::context::current(),
            text(&beta),
            "src/beta".into(),
            20,
        )
        .await?;
    // @step Then "src/beta_only.rs" is found
    assert!(
        found.iter().any(|path| path.ends_with("beta_only.rs")),
        "{found:?}"
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
#[serial]
async fn input_history_over_rpc_is_kept_per_named_workspace() -> TestResult {
    // @step Given a daemon started in workspace "alpha" and a second workspace "beta"
    let data = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let root = tempfile::tempdir()?;
    let alpha = workspace(root.path(), "alpha", &[])?;
    let beta = workspace(root.path(), "beta", &[])?;
    let service = daemon(&alpha)?;
    let session = SessionId::new("s-beta");
    // @step When the input "beta command" is added to the history of "beta" over RPC
    service
        .clone()
        .persistence_add_history_in_workspace(
            tarpc::context::current(),
            text(&beta),
            session.clone(),
            "beta command".into(),
        )
        .await?;
    // @step Then the history of "beta" contains "beta command"
    let beta_history = service
        .clone()
        .persistence_get_history_in_workspace(
            tarpc::context::current(),
            text(&beta),
            session.clone(),
            10,
        )
        .await?;
    assert_eq!(beta_history, vec!["beta command".to_string()]);
    let searched = service
        .clone()
        .persistence_search_history_in_workspace(
            tarpc::context::current(),
            text(&beta),
            "beta".into(),
        )
        .await?;
    assert_eq!(searched.len(), 1, "{searched:?}");
    // @step And the start workspace's history does not
    let start_history = service
        .persistence_get_history(tarpc::context::current(), session, 10)
        .await?;
    assert!(start_history.is_empty(), "{start_history:?}");
    Ok(())
}

/// The App with workspaces alpha (start) and beta listed and beta chosen.
fn app_showing_beta(mock: &Arc<MockBackend>) -> App {
    let mut app = App::new(mock.clone());
    app.board_store_mut().set_workspaces(vec![
        WorkspaceEntry {
            path: "/work/alpha".into(),
            name: "alpha".into(),
            start: true,
        },
        WorkspaceEntry {
            path: "/work/beta".into(),
            name: "beta".into(),
            start: false,
        },
    ]);
    app.dispatch(Action::OpenWorkspaceDropdown);
    app.dispatch(Action::ChooseWorkspace(Some(1)));
    app
}

async fn settle(app: &mut App) -> TestResult {
    for _ in 0..8 {
        while let Some(task) = app.next_pending_task() {
            task.await.completed_or_cancelled()?;
        }
        // TEST-042: scheduler rounds, not a time window: on the
        // current-thread test runtime every ready task (a spawned backend
        // call, a subscriber forwarding a push) runs before this resumes.
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
        }
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn the_tui_sends_input_history_and_session_lists_to_the_selected_workspace() -> TestResult {
    // @step Given the board shows workspace "beta"
    let mock = Arc::new(MockBackend::new());
    let mut app = app_showing_beta(&mock);
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    settle(&mut app).await?;
    // @step When the user recalls history and opens the session list
    app.dispatch(Action::HistoryPrev);
    app.dispatch(Action::SearchHistory("cmd".into()));
    settle(&mut app).await?;
    app.dispatch(Action::SlashCommandSelected(
        app.current_session(),
        codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction::Resume,
    ));
    settle(&mut app).await?;
    // @step Then each request names workspace "beta"
    let requests = mock.workspace_scope_requests();
    for expected in ["get_history:/work/beta", "search_history:/work/beta"] {
        assert!(
            requests.contains(&expected.to_string()),
            "{expected} missing from {requests:?}"
        );
    }
    assert_eq!(mock.list_sessions_project().as_deref(), Some("/work/beta"));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn model_sync_looks_the_session_up_in_its_own_project() -> TestResult {
    // @step Given a session recorded in project "/work/beta"
    let mock = Arc::new(MockBackend::new());
    let mut app = App::new(mock.clone());
    app.dispatch(Action::SessionObserved(session_in("s-beta", "/work/beta")));
    // @step When the session's model selection is synchronised
    app.dispatch(Action::SessionCreated(SessionId::new("s-beta")));
    settle(&mut app).await?;
    // @step Then the sessions of "/work/beta" are listed
    assert_eq!(mock.list_sessions_project().as_deref(), Some("/work/beta"));
    Ok(())
}
