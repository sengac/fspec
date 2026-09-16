//! BUG-182 — `GitStateWatcher` (codelet-core): the ONE centralized git
//! polling mechanism.
//!
//! Feature: spec/features/git-state-watcher.feature
//!
//! Watcher-level scenarios:
//!   - Manual checkpoint creation publishes a GitState frame with
//!     incremented counts on the fs-watch;
//!   - The periodic poll catches a working-tree change that emits no
//!     .git event;
//!   - Dedup: unchanged snapshots across poll ticks broadcast nothing.
//!
//! The App/transport scenarios live in
//! `rust/fspec-tui/tests/bug182_git_state_watcher.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::time::Duration;

use codelet_core::git_state::GitStateWatcher;
use codelet_git::ghost_commit::create_ghost_commit;
use codelet_rpc_types::GitState;

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
/// satisfying `pred`. Frames are full snapshots, so the first match is
/// the authoritative post-change state.
async fn wait_for_state<F: Fn(&GitState) -> bool>(
    watcher: &GitStateWatcher,
    pred: F,
    timeout: Duration,
) -> GitState {
    let mut rx = watcher.subscribe();
    // The initial snapshot was broadcast before this receiver existed;
    // backfill so a no-change scenario still resolves.
    let mut got = watcher.snapshot();
    if pred(&got) {
        return got;
    }
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);
    loop {
        let res = tokio::select! {
            _ = &mut deadline => panic!("timeout: watcher did not publish a matching frame (last {got:?})"),
            r = rx.recv() => r,
        };
        match res {
            Ok(state) => {
                got = state;
                if pred(&got) {
                    return got;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                panic!("watcher channel closed while waiting (last {got:?})")
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Manual checkpoint creation publishes a GitState frame with
// incremented counts on the fs-watch
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn manual_checkpoint_creation_publishes_a_git_state_frame_with_incremented_counts_on_the_fs_watch(
) {
    // @step Given a GitStateWatcher watching a temp git repo that has no checkpoint refs
    let tmp = setup_test_repo();
    let repo = tmp.path();
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(250));
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        codelet_rpc_types::CheckpointCounts { manual: 0, auto: 0 },
        "initial snapshot must be zero counts"
    );

    // @step When a manual ghost-checkpoint ref AUTH-001/baseline is written into the repo
    make_checkpoint(repo, "AUTH-001", "baseline");

    // @step Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 0 } on its broadcast channel
    let state = wait_for_state(
        &watcher,
        |s| s.checkpoint_counts == codelet_rpc_types::CheckpointCounts { manual: 1, auto: 0 },
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(
        state.checkpoint_counts,
        codelet_rpc_types::CheckpointCounts { manual: 1, auto: 0 }
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The periodic poll catches a working-tree change that emits no
// .git event
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn the_periodic_poll_catches_a_working_tree_change_that_emits_no_git_event() {
    // @step Given a GitStateWatcher watching a temp git repo with a clean working tree
    let tmp = setup_test_repo();
    let repo = tmp.path();
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(250));
    assert!(
        watcher.snapshot().changed_files.is_empty(),
        "initial snapshot of a clean tree must have no changed files"
    );

    // @step When the modified file's content is rewritten WITHOUT any .git event and the watcher's poll interval elapses
    fs::write(repo.join("README.md"), "# Test Repository\nChanged\n").expect("rewrite README");

    // @step Then the watcher publishes a GitState frame whose changed_files list reflects the new state
    let state = wait_for_state(
        &watcher,
        |s| {
            s.changed_files
                .iter()
                .any(|f| f.path == "README.md" && !f.staged)
        },
        Duration::from_secs(5),
    )
    .await;
    assert!(
        state
            .changed_files
            .iter()
            .any(|f| f.path == "README.md" && !f.staged),
        "the frame must carry the working-tree change: {state:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: GitStateWatcher dedups unchanged snapshots across poll ticks
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn git_state_watcher_dedups_unchanged_snapshots_across_poll_ticks() {
    // @step Given a GitStateWatcher watching a temp git repo with a fixed snapshot and a 100ms poll interval
    let tmp = setup_test_repo();
    let repo = tmp.path();
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(100));
    let initial = watcher.snapshot();

    // @step When four poll ticks elapse without any repository change
    let mut rx = watcher.subscribe();
    tokio::time::sleep(Duration::from_millis(600)).await;

    // @step Then only the initial snapshot was broadcast — a subscriber that counts frames for 600ms observes exactly 1 frame
    // NOTE: the single (initial) broadcast landed before `rx` existed, so
    // this receiver must observe ZERO frames during the silent window —
    // dedup means an unchanged poll tick is NEVER re-broadcast.
    let mut frames = 0usize;
    let deadline = std::time::Instant::now() + Duration::from_millis(500);
    while std::time::Instant::now() < deadline {
        match rx.try_recv() {
            Ok(frame) => {
                frames += 1;
                assert_eq!(
                    frame, initial,
                    "an unchanged poll tick must not re-broadcast a different frame"
                );
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                // Yield to the runtime so the watcher's poll task can run
                // (the dedup test is only meaningful if the timer fires).
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                frames += n as usize;
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Closed) => break,
        }
    }
    assert_eq!(
        frames, 0,
        "dedup: unchanged poll ticks must not re-broadcast (only the initial frame was ever sent)"
    );
    assert_eq!(
        watcher.snapshot(),
        initial,
        "the stored snapshot must be untouched by no-change ticks"
    );
}
