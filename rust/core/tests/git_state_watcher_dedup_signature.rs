//! Feature: spec/features/git-state-watcher-dedup-signature.feature
//!
//! Moved out of git_state_watcher.rs (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::git_state::GitStateWatcher;
use codelet_git::ghost_commit::create_ghost_commit;
use codelet_rpc_types::GitState;
use std::fs;
use std::path::Path;
use std::time::Duration;

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
async fn wait_for_state<F: Fn(&GitState) -> bool>(watcher: &GitStateWatcher, pred: F) -> GitState {
    let mut rx = watcher.subscribe();
    // The initial snapshot was broadcast before this receiver existed;
    // backfill so a no-change scenario still resolves.
    let mut got = watcher.snapshot();
    if pred(&got) {
        return got;
    }
    // TEST-042: the deadlock guard ends a watcher that never publishes.
    loop {
        let res = match codelet_test_helpers::deadlock_guard("a matching watcher frame", rx.recv())
            .await
        {
            Ok(r) => r,
            Err(_) => panic!("timeout: watcher did not publish a matching frame (last {got:?})"),
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
// BUG-188 — feature: spec/features/git-state-watcher-dedup-signature.feature
// ─────────────────────────────────────────────────────────────────────────

/// BUG-657: the watcher's first capture lands on the blocking pool, never
/// in its constructor; wait for it (or take it, when it has already
/// landed) and keep the receiver for later frames.
async fn first_frame(
    watcher: &GitStateWatcher,
) -> (GitState, tokio::sync::broadcast::Receiver<GitState>) {
    let (backfill, mut rx) = watcher.subscribe_backfilled();
    let first = match backfill {
        Some(state) => state,
        None => codelet_test_helpers::deadlock_guard("the first capture", rx.recv())
            .await
            .expect("the first capture lands")
            .expect("the watcher publishes it"),
    };
    (first, rx)
}

/// Count frames on `rx` for `window` (yielding to the runtime). Dedup
/// means an unchanged poll tick must NEVER re-broadcast — the caller
/// asserts the count is exactly zero in the silent window.
fn count_frames(rx: &mut tokio::sync::broadcast::Receiver<GitState>) -> usize {
    // TEST-042: the silent window has already passed on the paused clock
    // (the caller slept through several poll ticks), so every frame it
    // produced is queued; count them without waiting.
    let mut frames = 0usize;
    loop {
        match rx.try_recv() {
            // BUG-188: any frame during the silent window is a dedup
            // failure — the caller's `frames == 0` assert catches it.
            Ok(_frame) => frames += 1,
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => frames += n as usize,
            Err(_) => return frames,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Steady-state silence in a clean repo (regression guard)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(start_paused = true)] // TEST-042: virtual time
async fn steady_state_silence_in_a_clean_repo() {
    // @step Given a GitStateWatcher watching a temp git repo with no checkpoint refs and a 100ms poll interval
    let tmp = setup_test_repo();
    let repo = tmp.path();
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(100));
    let (initial, mut rx) = first_frame(&watcher).await;
    assert!(
        initial.checkpoints.is_empty(),
        "clean fixture repo must carry no checkpoints"
    );

    // @step When six poll ticks elapse without any repository change
    tokio::time::sleep(Duration::from_millis(600)).await;

    // @step Then a subscriber that counts frames for 600ms observes exactly 0 frames after the initial broadcast
    // The receiver must observe ZERO frames after the initial one during
    // the silent window.
    let frames = count_frames(&mut rx);
    assert_eq!(
        frames, 0,
        "dedup: unchanged poll ticks must not re-broadcast"
    );
    assert_eq!(
        watcher.snapshot(),
        initial,
        "stored snapshot must be untouched"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Steady-state silence in a checkpointed repo (BUG-188 regression)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(start_paused = true)] // TEST-042: virtual time
async fn steady_state_silence_in_a_checkpointed_repo() {
    // @step Given a GitStateWatcher watching a temp git repo with one ghost-checkpoint ref AUTH-001/baseline and a 100ms poll interval
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(100));
    let (initial, mut rx) = first_frame(&watcher).await;
    assert_eq!(
        initial.checkpoint_counts,
        codelet_rpc_types::CheckpointCounts { manual: 1, auto: 0 },
        "the fixture checkpoint must be visible in the initial snapshot"
    );

    // @step When six poll ticks elapse without any repository change
    tokio::time::sleep(Duration::from_millis(600)).await;

    // @step Then a subscriber that counts frames for 600ms observes exactly 0 frames after the initial broadcast
    // BUG-188: with a checkpoint ref present, the old full-`==` dedup never
    // fired (checkpoints[].timestamp re-stamped SystemTime::now() per
    // capture) — every tick re-broadcast. The stable signature excludes the
    // timestamp leg, so the silent window must stay silent.
    let frames = count_frames(&mut rx);
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
        first_frame(&watcher).await.0.checkpoint_counts,
        codelet_rpc_types::CheckpointCounts { manual: 1, auto: 0 },
    );

    // @step When a second ghost-checkpoint ref AUTH-002/alpha is written into the repo
    make_checkpoint(repo, "AUTH-002", "alpha");

    // @step Then the watcher publishes a frame whose checkpoints list carries both refs and checkpoint_counts is { manual: 2, auto: 0 }
    let state = wait_for_state(&watcher, |s| {
        s.checkpoint_counts == codelet_rpc_types::CheckpointCounts { manual: 2, auto: 0 }
    })
    .await;
    let names: Vec<(String, String)> = state
        .checkpoints
        .iter()
        .map(|c| (c.work_unit_id.clone(), c.name.clone()))
        .collect();
    assert!(
        names
            .iter()
            .any(|(wu, n)| wu == "AUTH-001" && n == "baseline"),
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
