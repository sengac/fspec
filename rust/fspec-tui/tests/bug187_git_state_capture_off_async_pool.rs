//! BUG-187 — production service wiring: `SharedFspecService::with_cwd`
//! counts every GitState capture that runs (on the blocking pool).
//!
//! Feature: spec/features/git-state-watcher-blocking-pool.feature
//!
//! Integration scenario (service wiring of the SAME scenario the core
//! watcher-level test covers): a checkpoint ref written from another
//! terminal produces a counted capture AND a fresh GitState frame with
//! the updated counts on the production wiring. The proof that those
//! captures run OFF the async pool lives at the watcher level in
//! `rust/core/tests/bug187_blocking_capture.rs` (the hook fires on the
//! blocking-pool thread; `Handle::try_current()` fails there).

use codelet_test_helpers::{OrFail, TestResult};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_git::ghost_commit::create_ghost_commit;
use codelet_rpc::SharedFspecService;
use tempfile::TempDir;

/// Create a basic test git repository with an initial commit.
fn setup_test_repo() -> Result<TempDir, codelet_test_helpers::TestError> {
    let tmp_dir = tempfile::TempDir::new().or_fail("tempdir")?;
    let repo_path = tmp_dir.path();
    for args in [
        vec!["init"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test User"],
    ] {
        std::process::Command::new("git")
            .args(args)
            .current_dir(repo_path)
            .output()
            .or_fail("git")?;
    }
    fs::write(repo_path.join("README.md"), "# Test Repository\n").or_fail("write README")?;
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .or_fail("git add")?;
    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo_path)
        .output()
        .or_fail("git commit")?;
    Ok(tmp_dir)
}

/// Create a checkpoint by writing a unique file then snapshotting the
/// worktree (triggers `.git` fs-events + a new checkpoint ref).
fn make_checkpoint(
    repo: &Path,
    work_unit_id: &str,
    name: &str,
) -> Result<(), codelet_test_helpers::TestError> {
    let marker = repo.join(format!("touch-{work_unit_id}-{name}.txt"));
    fs::write(&marker, format!("{work_unit_id}/{name}")).or_fail("write marker")?;
    create_ghost_commit(repo, work_unit_id, name).or_fail("create_ghost_commit")?;
    Ok(())
}

/// Build a `SharedFspecService` with a real (empty) watcher + the repo
/// cwd attached — the SAME production wiring as
/// `rust/fspec/src/common.rs::build_service` (minus the session
/// manager), so the GitStateWatcher runs on the attached cwd.
fn service_for(repo: &Path) -> Result<Arc<SharedFspecService>, codelet_test_helpers::TestError> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo).or_fail("watcher on temp repo")?);
    Ok(Arc::new(
        SharedFspecService::new(watcher).with_cwd(repo.to_path_buf()),
    ))
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A checkpoint written from another terminal produces a
// counted capture on the production wiring
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_checkpoint_written_from_another_terminal_produces_a_counted_capture_on_the_production_wiring(
) -> TestResult {
    // @step Given a GitStateWatcher watching a temp git repo with no checkpoint refs
    let tmp = setup_test_repo()?;
    let repo = tmp.path();
    // Production wiring: SharedFspecService::with_cwd builds the ONE
    // centralized GitStateWatcher (BUG-182) on this repo, with the
    // capture counter hook wired in.
    let service = service_for(repo)?;
    // BUG-657: the first capture runs on the blocking pool too, through
    // the same counted hook — never in the constructor.
    codelet_test_helpers::wait_until("the first capture through the hook", || {
        std::future::ready(service.git_state_captures() >= 1)
    })
    .await?;
    assert_eq!(
        service.git_state_snapshot().checkpoint_counts,
        codelet_rpc_types::CheckpointCounts { manual: 0, auto: 0 },
        "initial snapshot must be zero counts"
    );

    // @step When a manual ghost-checkpoint ref AUTH-001/baseline is written into the repo's .git from outside the process
    make_checkpoint(repo, "AUTH-001", "baseline")?;

    // @step Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 0 } within the poll/fs-watch window (the off-thread capture must not delay or drop the update)
    // TEST-042: wait for the re-captured snapshot under the deadlock guard
    // (a lost update fails loudly there), not within a fixed window.
    codelet_test_helpers::wait_until("the fs-event re-capture", || {
        std::future::ready(
            service.git_state_snapshot().checkpoint_counts
                == (codelet_rpc_types::CheckpointCounts { manual: 1, auto: 0 }),
        )
    })
    .await?;
    assert_eq!(
        service.git_state_snapshot().checkpoint_counts,
        codelet_rpc_types::CheckpointCounts { manual: 1, auto: 0 }
    );
    // The update arrived via a capture that ran through the production
    // wiring's hook (the off-thread capture path is the ONLY path that
    // re-captures after construction).
    assert!(
        service.git_state_captures() >= 1,
        "the re-capture must run through the production wiring's capture hook"
    );
    Ok(())
}
