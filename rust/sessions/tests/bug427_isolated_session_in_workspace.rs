//! BUG-427 — isolated sessions are created in the requested workspace.
//!
//! Feature: spec/features/isolated-session-in-workspace.feature

// The shared WT-004 fixtures; the WT-004 suite that owns them allows these
// lints for the same helpers.
#[path = "wt004_session_tool_callbacks_support/mod.rs"]
#[allow(dead_code, clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod support;

use codelet_core::SessionManagerHandle;
use support::{fresh_git_repo, manager_with_seeded_cache, ENV_GUARD};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_isolated_session_is_cut_from_the_requested_workspace() -> TestResult {
    let _guard = ENV_GUARD.lock().await;
    // @step Given a session manager whose start workspace is one repository
    let manager = manager_with_seeded_cache();
    let start = fresh_git_repo();
    let other = fresh_git_repo();
    manager.set_default_project(start.path().to_path_buf())?;
    manager.set_default_model("anthropic/claude-opus-4-5")?;
    // @step When an isolated session is created in a second repository
    let info = SessionManagerHandle::create_isolated_session_in_workspace(
        manager.as_ref(),
        other.path(),
        None,
    )?;
    // @step Then the session's project is the second repository and its worktree belongs to it
    let other_root = std::fs::canonicalize(other.path())?;
    let session = manager.get_session(&info.session_id.value)?;
    assert_eq!(std::path::PathBuf::from(&session.project), other_root);
    let listing = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(&other_root)
        .output()?;
    let worktrees = String::from_utf8_lossy(&listing.stdout);
    assert!(worktrees.contains(&info.worktree_path), "{worktrees}");
    Ok(())
}
