//! RPC-453 — Checkpoints view and checkpoint counts follow the selected
//! workspace.
//!
//! Feature: spec/features/checkpoints-follow-the-selected-workspace.feature

mod common;

use crate::common::TaskOutcome;
use codelet_test_helpers::TestResult;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{Action, App};
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
use codelet_rpc_types::WorkspaceEntry;
use common::checkpoint_repo::{make_checkpoint, setup_test_repo};
use common::MockBackend;
use serial_test::serial;

/// A git repository with spec/work-units.json, canonicalized.
fn repo() -> Result<(tempfile::TempDir, PathBuf), Box<dyn std::error::Error>> {
    let dir = setup_test_repo()?;
    std::fs::create_dir_all(dir.path().join("spec"))?;
    std::fs::write(
        dir.path().join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )?;
    let path = std::fs::canonicalize(dir.path())?;
    Ok((dir, path))
}

fn daemon(start: &Path) -> Result<FspecServiceImpl, Box<dyn std::error::Error>> {
    Ok(FspecServiceImpl::new(Arc::new(
        SharedFspecService::new(Arc::new(WorkUnitsWatcher::new(start)?))
            .with_cwd(start.to_path_buf()),
    )))
}

fn ctx() -> tarpc::context::Context {
    tarpc::context::current()
}

#[tokio::test]
#[serial]
async fn checkpoints_over_rpc_come_from_the_named_workspace() -> TestResult {
    // @step Given a daemon started in git repository "alpha" and a second git repository "beta" with checkpoint "cp-beta"
    let data = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let (_alpha_dir, alpha) = repo()?;
    let (_beta_dir, beta) = repo()?;
    make_checkpoint(&beta, "BETA-1", "cp-beta")?;
    let service = daemon(&alpha)?;
    let workspace = beta.to_string_lossy().into_owned();
    // @step When the checkpoints of workspace "beta" are listed over RPC
    let listed = service
        .clone()
        .list_checkpoints_in_workspace(ctx(), workspace.clone())
        .await?;
    // @step Then the listing contains "cp-beta"
    assert!(
        listed.iter().any(|checkpoint| checkpoint.name == "cp-beta"),
        "{listed:?}"
    );
    // @step And the checkpoint counts of workspace "beta" report one manual checkpoint
    let counts = service
        .checkpoint_counts_in_workspace(ctx(), workspace)
        .await?;
    assert_eq!(counts.manual, 1, "{counts:?}");
    Ok(())
}

#[tokio::test]
#[serial]
async fn restoring_a_checkpoint_over_rpc_acts_on_the_named_workspace() -> TestResult {
    // @step Given a daemon started in git repository "alpha" and a second git repository "beta" with checkpoint "cp-beta" whose marker file was deleted
    let data = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let (_alpha_dir, alpha) = repo()?;
    let (_beta_dir, beta) = repo()?;
    make_checkpoint(&beta, "BETA-1", "cp-beta")?;
    let marker = beta.join("touch-BETA-1-cp-beta.txt");
    std::fs::remove_file(&marker)?;
    let service = daemon(&alpha)?;
    // @step When checkpoint "cp-beta" of workspace "beta" is restored over RPC
    service
        .restore_checkpoint_all_in_workspace(
            ctx(),
            beta.to_string_lossy().into_owned(),
            "BETA-1".to_string(),
            "cp-beta".to_string(),
        )
        .await?;
    // @step Then the marker file exists again in "beta"
    assert!(marker.exists(), "{} was not restored", marker.display());
    assert!(!alpha.join("touch-BETA-1-cp-beta.txt").exists());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_checkpoints_view_lists_the_selected_workspaces_checkpoints() -> TestResult {
    // @step Given the board has workspace "beta" selected
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
    assert!(app.board_store_mut().select_workspace("/work/beta"));
    // @step When the Checkpoints view opens
    app.dispatch(Action::OpenCheckpointsView);
    loop {
        while let Some(task) = app.next_pending_task() {
            task.await.completed_or_cancelled()?;
        }
        let Some(action) = app.try_recv_action() else {
            break;
        };
        app.dispatch(action);
    }
    // @step Then the backend is asked for the checkpoints of workspace "beta"
    assert!(
        mock.checkpoint_requests()
            .contains(&"list_checkpoints:/work/beta".to_string()),
        "{:?}",
        mock.checkpoint_requests()
    );
    Ok(())
}
