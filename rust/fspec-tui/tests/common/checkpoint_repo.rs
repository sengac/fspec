//! BUG-355: temp git repo + checkpoint fixtures shared by the BUG-181 and
//! TUI-109 test files (one copy instead of one per test file).

use codelet_test_helpers::OrFail;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_git::ghost_commit::create_ghost_commit;
use codelet_rpc::SharedFspecService;
use tempfile::TempDir;

/// Create a basic test git repository with an initial commit (mirrors
/// `rust/git/tests/common/mod.rs::setup_test_repo`).
pub fn setup_test_repo() -> Result<TempDir, codelet_test_helpers::TestError> {
    let tmp_dir = TempDir::new().or_fail("tempdir")?;
    let repo_path = tmp_dir.path();
    for args in [
        vec!["init"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test User"],
    ] {
        Command::new("git")
            .args(args)
            .current_dir(repo_path)
            .output()
            .or_fail("git")?;
    }
    fs::write(repo_path.join("README.md"), "# Test Repository\n").or_fail("write README")?;
    for args in [vec!["add", "."], vec!["commit", "-m", "initial"]] {
        Command::new("git")
            .args(args)
            .current_dir(repo_path)
            .output()
            .or_fail("git add/commit")?;
    }
    Ok(tmp_dir)
}

/// Create a checkpoint by writing a unique file then snapshotting the
/// worktree.
pub fn make_checkpoint(
    repo: &Path,
    work_unit_id: &str,
    name: &str,
) -> Result<(), codelet_test_helpers::TestError> {
    let marker = repo.join(format!("touch-{work_unit_id}-{name}.txt"));
    fs::write(&marker, format!("{work_unit_id}/{name}")).or_fail("write marker")?;
    create_ghost_commit(repo, work_unit_id, name).or_fail("create_ghost_commit")?;
    Ok(())
}

/// Build a `SharedFspecService` with a real (empty) work-units watcher and
/// the repo cwd attached, so the git-state watcher and `list_checkpoints`
/// read the real repo.
pub fn service_for(
    repo: &Path,
) -> Result<Arc<SharedFspecService>, codelet_test_helpers::TestError> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo).or_fail("watcher on temp repo")?);
    Ok(Arc::new(
        SharedFspecService::new(watcher).with_cwd(repo.to_path_buf()),
    ))
}
