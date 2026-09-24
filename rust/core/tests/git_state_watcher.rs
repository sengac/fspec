//! BUG-182 — `GitStateWatcher` (codelet-core): the ONE centralized git
//! polling mechanism.
//!
//! Feature: spec/features/git-state-watcher.feature
//! BUG-188 feature: spec/features/git-state-watcher-dedup-signature.feature
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

// ─────────────────────────────────────────────────────────────────────────
// BUG-188 — feature: spec/features/git-state-watcher-dedup-signature.feature
// ─────────────────────────────────────────────────────────────────────────

/// Count frames on `rx` for `window` (yielding to the runtime). Dedup
/// means an unchanged poll tick must NEVER re-broadcast — the caller
/// asserts the count is exactly zero in the silent window.
async fn count_frames(
    rx: &mut tokio::sync::broadcast::Receiver<GitState>,
    window: Duration,
) -> usize {
    let mut frames = 0usize;
    let deadline = tokio::time::Instant::now() + window;
    while tokio::time::Instant::now() < deadline {
        match rx.try_recv() {
            Ok(_frame) => {
                // BUG-188: any frame during the silent window is a
                // dedup failure — the caller's `frames == 0` assert
                // catches it. (The frame's raw timestamps legitimately
                // differ from the initial snapshot, so we do NOT compare
                // frames here — the count is the invariant.)
                frames += 1;
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                // Yield so the watcher's poll task can run (the dedup test
                // is only meaningful if the timer actually fires).
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                frames += n as usize;
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Closed) => break,
        }
    }
    frames
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Steady-state silence in a clean repo (regression guard)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn steady_state_silence_in_a_clean_repo() {
    // @step Given a GitStateWatcher watching a temp git repo with no checkpoint refs and a 100ms poll interval
    let tmp = setup_test_repo();
    let repo = tmp.path();
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(100));
    let initial = watcher.snapshot();
    assert!(
        watcher.snapshot().checkpoints.is_empty(),
        "clean fixture repo must carry no checkpoints"
    );

    // @step When six poll ticks elapse without any repository change
    let mut rx = watcher.subscribe();
    tokio::time::sleep(Duration::from_millis(600)).await;

    // @step Then a subscriber that counts frames for 600ms observes exactly 0 frames after the initial broadcast
    // The initial broadcast landed before `rx` existed — the receiver must
    // observe ZERO frames during the silent window.
    let frames = count_frames(&mut rx, Duration::from_millis(500)).await;
    assert_eq!(frames, 0, "dedup: unchanged poll ticks must not re-broadcast");
    assert_eq!(watcher.snapshot(), initial, "stored snapshot must be untouched");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Steady-state silence in a checkpointed repo (BUG-188 regression)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn steady_state_silence_in_a_checkpointed_repo() {
    // @step Given a GitStateWatcher watching a temp git repo with one ghost-checkpoint ref AUTH-001/baseline and a 100ms poll interval
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(100));
    let initial = watcher.snapshot();
    assert_eq!(
        initial.checkpoint_counts,
        codelet_rpc_types::CheckpointCounts { manual: 1, auto: 0 },
        "the fixture checkpoint must be visible in the initial snapshot"
    );

    // @step When six poll ticks elapse without any repository change
    let mut rx = watcher.subscribe();
    tokio::time::sleep(Duration::from_millis(600)).await;

    // @step Then a subscriber that counts frames for 600ms observes exactly 0 frames after the initial broadcast
    // BUG-188: with a checkpoint ref present, the old full-`==` dedup never
    // fired (checkpoints[].timestamp re-stamped SystemTime::now() per
    // capture) — every tick re-broadcast. The stable signature excludes the
    // timestamp leg, so the silent window must stay silent.
    let frames = count_frames(&mut rx, Duration::from_millis(500)).await;
    assert_eq!(
        frames, 0,
        "dedup must hold WITH checkpoint refs present (BUG-188 regression)"
    );

    // @step And the stored snapshot is untouched by no-change ticks
    assert_eq!(
        watcher.snapshot(),
        initial,
        "the stored snapshot must be untouched by no-change ticks"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A new checkpoint ref produces a frame on the checkpointed repo
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_new_checkpoint_ref_produces_a_frame_on_the_checkpointed_repo() {
    // @step Given a GitStateWatcher watching a temp git repo with one ghost-checkpoint ref AUTH-001/baseline and a 250ms poll interval
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(250));
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        codelet_rpc_types::CheckpointCounts { manual: 1, auto: 0 },
    );

    // @step When a second ghost-checkpoint ref AUTH-002/alpha is written into the repo
    make_checkpoint(repo, "AUTH-002", "alpha");

    // @step Then the watcher publishes a frame whose checkpoints list carries both refs and checkpoint_counts is { manual: 2, auto: 0 }
    let state = wait_for_state(
        &watcher,
        |s| s.checkpoint_counts == codelet_rpc_types::CheckpointCounts { manual: 2, auto: 0 },
        Duration::from_secs(5),
    )
    .await;
    let names: Vec<(String, String)> = state
        .checkpoints
        .iter()
        .map(|c| (c.work_unit_id.clone(), c.name.clone()))
        .collect();
    assert!(
        names.iter().any(|(wu, n)| wu == "AUTH-001" && n == "baseline"),
        "frame must carry the pre-existing checkpoint: {names:?}"
    );
    assert!(
        names.iter().any(|(wu, n)| wu == "AUTH-002" && n == "alpha"),
        "frame must carry the new checkpoint: {names:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Two captures of an unchanged checkpointed repo produce equal
// signatures but unequal raw states
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn two_captures_of_an_unchanged_checkpointed_repo_differ_raw_but_match_in_signature() {
    // @step Given a temp git repo with one ghost-checkpoint ref
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");

    // @step When the GitState snapshot is captured twice in a row
    // (Two captures of the SAME repo at ~millisecond spacing: the fallback
    // "now" timestamp on every checkpoint row is guaranteed to differ.)
    let a = codelet_core::git_state::capture(repo);
    let b = codelet_core::git_state::capture(repo);

    // @step Then the two raw GitState values are not ==-equal (checkpoints[].timestamp differs)
    assert!(
        !a.checkpoints.is_empty(),
        "fixture repo must carry the checkpoint leg"
    );
    assert!(
        a.checkpoints
            .iter()
            .zip(b.checkpoints.iter())
            .any(|(x, y)| x.timestamp != y.timestamp),
        "root cause: the fallback timestamp re-stamps SystemTime::now() per capture, so full-struct == never holds (old dedup was dead code)"
    );
    assert_ne!(a, b, "raw GitState values must NOT be ==-equal");

    // @step And the two stable signatures ARE equal (timestamps excluded from the signature)
    assert_eq!(
        codelet_core::git_state::git_state_signature(&a),
        codelet_core::git_state::git_state_signature(&b),
        "the stable signature must exclude the volatile timestamp leg so unchanged captures dedup"
    );
}
