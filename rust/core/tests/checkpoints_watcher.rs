//! BUG-181 — `CheckpointsWatcher` (codelet-core) publishes fresh
//! `CheckpointCounts` on a debounced fs-watch of the checkpoint locations.
//!
//! Feature: spec/features/live-checkpoint-counts-in-the-board-header.feature
//!
//! Watcher-level scenarios:
//!   - Manual checkpoint creation publishes incremented counts;
//!   - Auto-checkpoint creation publishes incremented counts;
//!   - Checkpoint delete publishes decremented counts;
//!   - Watcher degrades to zero counts when the cwd is not a git repo.
//!
//! The App/transport/WS scenarios live in
//! `rust/fspec-tui/tests/bug181_live_checkpoint_counts.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::time::Duration;

use codelet_core::checkpoints_watcher::CheckpointsWatcher;
use codelet_git::ghost_commit::{create_ghost_commit, delete_ghost_checkpoint};
use codelet_rpc_types::CheckpointCounts;

/// Create a basic test git repository with an initial commit (mirrors
/// `rust/git/tests/common/mod.rs::setup_test_repo`).
fn setup_test_repo() -> tempfile::TempDir {
    let tmp_dir = tempfile::TempDir::new().expect("tempdir");
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
            .expect("git");
    }
    fs::write(repo_path.join("README.md"), "# Test Repository\n").expect("write README");
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("git add");
    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo_path)
        .output()
        .expect("git commit");
    tmp_dir
}

/// Create a checkpoint by writing a unique file then snapshotting the
/// worktree.
fn make_checkpoint(repo: &Path, work_unit_id: &str, name: &str) {
    let marker = repo.join(format!("touch-{work_unit_id}-{name}.txt"));
    fs::write(&marker, format!("{work_unit_id}/{name}")).expect("write marker");
    create_ghost_commit(repo, work_unit_id, name).expect("create_ghost_commit");
}

/// Wait (bounded) for the watcher's broadcast channel to deliver a frame
/// equal to `want`. Frames are full snapshots, so the first match is the
/// authoritative post-change state.
async fn wait_for_counts(watcher: &CheckpointsWatcher, want: CheckpointCounts) -> CheckpointCounts {
    let mut rx = watcher.subscribe();
    // The initial snapshot was broadcast before this receiver existed;
    // backfill so a no-change scenario still resolves.
    let mut got = watcher.snapshot();
    if got == want {
        return got;
    }
    let deadline = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(deadline);
    loop {
        let res = tokio::select! {
            _ = &mut deadline => panic!("timeout: watcher did not publish {want:?} (last {got:?})"),
            r = rx.recv() => r,
        };
        match res {
            Ok(counts) => {
                got = counts;
                if counts == want {
                    return counts;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                panic!("watcher channel closed while waiting for {want:?} (last {got:?})")
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Manual checkpoint creation publishes incremented counts on
// the checkpoint-changed push
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn manual_checkpoint_creation_publishes_incremented_counts_on_the_checkpoint_changed_push() {
    // @step Given a CheckpointsWatcher watching a temp git repo that has no checkpoint refs
    let tmp = setup_test_repo();
    let repo = tmp.path();
    let watcher = CheckpointsWatcher::new(repo);
    assert_eq!(
        watcher.snapshot(),
        CheckpointCounts { manual: 0, auto: 0 },
        "initial snapshot must be zero counts"
    );

    // @step When a manual ghost-checkpoint ref AUTH-001/baseline is written into the repo (as fspec checkpoint AUTH-001 baseline does)
    make_checkpoint(repo, "AUTH-001", "baseline");

    // @step Then the watcher publishes CheckpointCounts { manual: 1, auto: 0 } on its broadcast channel
    let counts = wait_for_counts(&watcher, CheckpointCounts { manual: 1, auto: 0 }).await;
    assert_eq!(counts, CheckpointCounts { manual: 1, auto: 0 });
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Auto-checkpoint creation publishes incremented counts on the
// checkpoint-changed push
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn auto_checkpoint_creation_publishes_incremented_counts_on_the_checkpoint_changed_push() {
    // @step Given a CheckpointsWatcher watching a temp git repo that has one manual checkpoint (AUTH-001/baseline)
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    let watcher = CheckpointsWatcher::new(repo);
    assert_eq!(
        watcher.snapshot(),
        CheckpointCounts { manual: 1, auto: 0 },
        "initial snapshot must reflect the existing manual checkpoint"
    );

    // @step When an automatic checkpoint ref AUTH-001/AUTH-001-auto-testing is written into the repo (as update-work-unit-status does before the transition)
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-testing");

    // @step Then the watcher publishes CheckpointCounts { manual: 1, auto: 1 } on its broadcast channel
    wait_for_counts(&watcher, CheckpointCounts { manual: 1, auto: 1 }).await;
    assert_eq!(watcher.snapshot(), CheckpointCounts { manual: 1, auto: 1 });
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Checkpoint delete publishes decremented counts on the
// checkpoint-changed push
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn checkpoint_delete_publishes_decremented_counts_on_the_checkpoint_changed_push() {
    // @step Given a CheckpointsWatcher watching a temp git repo that has one manual + one auto checkpoint (AUTH-001/baseline, AUTH-001/AUTH-001-auto-testing)
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-testing");
    let watcher = CheckpointsWatcher::new(repo);
    assert_eq!(watcher.snapshot(), CheckpointCounts { manual: 1, auto: 1 });

    // @step When the manual checkpoint ref is deleted from the repo (as fspec cleanup-checkpoints / TUI delete do)
    delete_ghost_checkpoint(repo, "AUTH-001", "baseline").expect("delete");

    // @step Then the watcher publishes CheckpointCounts { manual: 0, auto: 1 } on its broadcast channel
    wait_for_counts(&watcher, CheckpointCounts { manual: 0, auto: 1 }).await;
    assert_eq!(watcher.snapshot(), CheckpointCounts { manual: 0, auto: 1 });
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Watcher degrades to zero counts when the watched directory is
// not a git repository
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn watcher_degrades_to_zero_counts_when_the_watched_directory_is_not_a_git_repository() {
    // @step Given a CheckpointsWatcher watching a plain temp directory that has no .git
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let dir = tmp.path();
    let watcher = CheckpointsWatcher::new(dir);

    // @step When the watcher's initial snapshot is read and a file is later created inside that directory
    assert_eq!(
        watcher.snapshot(),
        CheckpointCounts { manual: 0, auto: 0 },
        "initial snapshot of a non-repo cwd must be zero counts with no error"
    );
    fs::write(dir.join("plain.txt"), "not a repo\n").expect("write file");

    // @step Then the initial snapshot is CheckpointCounts { manual: 0, auto: 0 }
    // (re-assert after the fs-write: a non-repo write must never surface an
    // error or change the snapshot away from zero counts)
    assert_eq!(watcher.snapshot(), CheckpointCounts { manual: 0, auto: 0 });
}
