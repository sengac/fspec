//! RPC-454 — git state, branch and footer directory follow the selected
//! workspace.
//!
//! Feature: spec/features/git-state-follows-the-selected-workspace.feature

mod common;

use crate::common::TaskOutcome;
use codelet_test_helpers::TestResult;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{Action, App};
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
use codelet_rpc_types::WorkspaceEntry;
use common::MockBackend;
use serial_test::serial;

fn git(cwd: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let out = Command::new("git").args(args).current_dir(cwd).output()?;
    if !out.status.success() {
        return Err(format!("git {args:?}: {}", String::from_utf8_lossy(&out.stderr)).into());
    }
    Ok(())
}

/// A committed git repository `<root>/<name>` checked out on `branch`.
fn repo(root: &Path, name: &str, branch: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = root.join(name);
    std::fs::create_dir_all(path.join("spec"))?;
    std::fs::write(path.join("spec/work-units.json"), r#"{"workUnits":{}}"#)?;
    git(&path, &["init", "-q", "-b", branch])?;
    git(&path, &["config", "user.email", "test@example.com"])?;
    git(&path, &["config", "user.name", "Test"])?;
    git(&path, &["add", "."])?;
    git(&path, &["commit", "-q", "-m", "initial"])?;
    Ok(std::fs::canonicalize(path)?)
}

fn daemon(
    start: &Path,
) -> Result<(Arc<SharedFspecService>, FspecServiceImpl), Box<dyn std::error::Error>> {
    let shared = Arc::new(
        SharedFspecService::new(Arc::new(WorkUnitsWatcher::new(start)?))
            .with_cwd(start.to_path_buf()),
    );
    Ok((shared.clone(), FspecServiceImpl::new(shared)))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn workspace_info_over_rpc_comes_from_the_named_workspace() -> TestResult {
    // @step Given a daemon started in git repository "alpha" and a second git repository "beta" on branch "feature-beta"
    let data = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let root = tempfile::tempdir()?;
    let alpha = repo(root.path(), "alpha", "main")?;
    let beta = repo(root.path(), "beta", "feature-beta")?;
    let (_shared, service) = daemon(&alpha)?;
    // @step When the workspace info of "beta" is requested over RPC
    let info = service
        .get_workspace_info_in_workspace(
            tarpc::context::current(),
            beta.to_string_lossy().into_owned(),
        )
        .await?;
    // @step Then it names beta's directory and the branch "feature-beta"
    assert_eq!(info.cwd, beta.to_string_lossy());
    assert_eq!(info.git_branch.as_deref(), Some("feature-beta"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn an_opened_workspaces_git_state_is_published() -> TestResult {
    // @step Given a daemon started in git repository "alpha" with workspace "beta" opened
    let data = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let root = tempfile::tempdir()?;
    let alpha = repo(root.path(), "alpha", "main")?;
    let beta = repo(root.path(), "beta", "main")?;
    let (shared, service) = daemon(&alpha)?;
    let mut pushes = shared.workspace_git_state_rx();
    service
        .git_state_in_workspace(
            tarpc::context::current(),
            beta.to_string_lossy().into_owned(),
        )
        .await?;
    // @step When a file in "beta" changes
    git(&beta, &["checkout", "-q", "-b", "feature-pushed"])?;
    // @step Then a git-state push tagged with workspace "beta" arrives
    // TEST-042: wait for the push under the deadlock guard.
    codelet_test_helpers::deadlock_guard("beta's git-state push", async {
        loop {
            let push = pushes.recv().await?;
            if push.workspace == beta.to_string_lossy()
                && push.state.git_branch.as_deref() == Some("feature-pushed")
            {
                return Ok::<(), tokio::sync::broadcast::error::RecvError>(());
            }
        }
    })
    .await??;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn choosing_a_workspace_loads_its_directory_branch_and_git_state() -> TestResult {
    // @step Given the board lists workspaces "alpha" and "beta"
    let mock = Arc::new(MockBackend::new());
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
    // @step When workspace "beta" is chosen
    app.dispatch(Action::OpenWorkspaceDropdown);
    app.dispatch(Action::ChooseWorkspace(Some(1)));
    loop {
        while let Some(task) = app.next_pending_task() {
            task.await.completed_or_cancelled()?;
        }
        let Some(action) = app.try_recv_action() else {
            break;
        };
        app.dispatch(action);
    }
    // @step Then the backend is asked for the workspace info and git state of "beta"
    let requests = mock.workspace_git_requests();
    assert!(
        requests.contains(&"info:/work/beta".to_string()),
        "{requests:?}"
    );
    assert!(
        requests.contains(&"git_state:/work/beta".to_string()),
        "{requests:?}"
    );
    Ok(())
}
