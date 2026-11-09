//! RPC-452 — Changed Files view and file diffs follow the selected workspace.
//!
//! Feature: spec/features/changed-files-follow-the-selected-workspace.feature

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
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serial_test::serial;

fn git(cwd: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let out = Command::new("git").args(args).current_dir(cwd).output()?;
    if !out.status.success() {
        return Err(format!("git {args:?}: {}", String::from_utf8_lossy(&out.stderr)).into());
    }
    Ok(())
}

/// A git repository `<root>/<name>` with `<name>.txt` committed then edited.
fn repo(root: &Path, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = root.join(name);
    std::fs::create_dir_all(path.join("spec"))?;
    std::fs::write(path.join("spec/work-units.json"), r#"{"workUnits":{}}"#)?;
    git(&path, &["init", "-q"])?;
    git(&path, &["config", "user.email", "test@example.com"])?;
    git(&path, &["config", "user.name", "Test"])?;
    std::fs::write(path.join(format!("{name}.txt")), "before\n")?;
    git(&path, &["add", "."])?;
    git(&path, &["commit", "-q", "-m", "initial"])?;
    std::fs::write(
        path.join(format!("{name}.txt")),
        format!("after in {name}\n"),
    )?;
    Ok(std::fs::canonicalize(path)?)
}

fn daemon(start: &Path) -> Result<FspecServiceImpl, Box<dyn std::error::Error>> {
    Ok(FspecServiceImpl::new(Arc::new(
        SharedFspecService::new(Arc::new(WorkUnitsWatcher::new(start)?))
            .with_cwd(start.to_path_buf()),
    )))
}

fn select_beta(app: &mut App) {
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
}

async fn drain(app: &mut App) -> TestResult {
    loop {
        while let Some(task) = app.next_pending_task() {
            task.await.completed_or_cancelled()?;
        }
        let Some(action) = app.try_recv_action() else {
            return Ok(());
        };
        app.dispatch(action);
    }
}

fn screen(app: &mut App) -> String {
    let area = Rect::new(0, 0, 160, 40);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
#[serial]
async fn changed_files_over_rpc_come_from_the_named_workspace() -> TestResult {
    // @step Given a daemon started in git repository "alpha" and a second git repository "beta" with a modified file "beta.txt"
    let data = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let root = tempfile::tempdir()?;
    let alpha = repo(root.path(), "alpha")?;
    let beta = repo(root.path(), "beta")?;
    let service = daemon(&alpha)?;
    // @step When the changed files of workspace "beta" are requested over RPC
    let files = service
        .changed_files_in_workspace(
            tarpc::context::current(),
            beta.to_string_lossy().into_owned(),
        )
        .await?;
    // @step Then the listing contains "beta.txt" and no file of "alpha"
    let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    assert!(paths.contains(&"beta.txt"), "{paths:?}");
    assert!(!paths.contains(&"alpha.txt"), "{paths:?}");
    Ok(())
}

#[tokio::test]
#[serial]
async fn a_file_diff_over_rpc_comes_from_the_named_workspace() -> TestResult {
    // @step Given a daemon started in git repository "alpha" and a second git repository "beta" with a modified file "beta.txt"
    let data = tempfile::tempdir()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    let root = tempfile::tempdir()?;
    let alpha = repo(root.path(), "alpha")?;
    let beta = repo(root.path(), "beta")?;
    let service = daemon(&alpha)?;
    // @step When the diff of "beta.txt" in workspace "beta" is requested over RPC
    let diff = service
        .file_diff_in_workspace(
            tarpc::context::current(),
            beta.to_string_lossy().into_owned(),
            "beta.txt".to_string(),
        )
        .await?
        .ok_or("beta.txt has no diff")?;
    // @step Then the diff shows the change made in "beta"
    assert!(diff.contains("after in beta"), "{diff}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_changed_files_view_requests_the_selected_workspace() -> TestResult {
    // @step Given the board has workspace "beta" selected
    let mock = Arc::new(MockBackend::new());
    let mut app = App::new(mock.clone());
    select_beta(&mut app);
    // @step When the Changed Files view opens
    app.dispatch(Action::OpenChangedFilesView);
    drain(&mut app).await?;
    // @step Then the backend is asked for the changed files of workspace "beta"
    assert_eq!(
        mock.changed_files_requests(),
        vec!["/work/beta".to_string()]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_changed_files_listing_is_shown() -> TestResult {
    // @step Given a backend whose changed-files listing fails with "not a git repository"
    let mock = Arc::new(MockBackend::new());
    mock.fail_changed_files("not a git repository");
    let mut app = App::new(mock.clone());
    select_beta(&mut app);
    // @step When the Changed Files view opens
    app.dispatch(Action::OpenChangedFilesView);
    drain(&mut app).await?;
    // @step Then an error dialog shows "not a git repository"
    let screen = screen(&mut app);
    assert!(screen.contains("not a git repository"), "{screen}");
    Ok(())
}
