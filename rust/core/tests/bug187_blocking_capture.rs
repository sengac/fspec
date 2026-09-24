//! BUG-187 — `GitStateWatcher` capture runs on the blocking pool, never on
//! an async worker (no pinned `tokio-rt-worker`).
//!
//! Feature: spec/features/git-state-watcher-blocking-pool.feature
//!
//! Watcher-level scenarios:
//!   - A slow GitState capture never blocks the other async work on the
//!     runtime (behavioral proof: on a single-threaded runtime, a 400ms
//!     capture dispatched to the blocking pool keeps a 50ms-cadence
//!     concurrent timer on schedule, while an inline capture on the
//!     worker would stall it by 400ms);
//!   - A tick that fires while a capture is in flight is dropped, not
//!     queued (the in-flight guard serializes captures and the second
//!     capture starts only after the first finishes);
//!   - Steady-state unchanged ticks still cost the user nothing
//!     observable (the off-thread capture still dedups);
//!   - A checkpoint written from another terminal still lands within
//!     the watch window (the off-thread capture neither delays nor
//!     drops the update).
//!
//! The TUI/transport scenarios live in
//! `rust/fspec-tui/tests/bug182_git_state_watcher.rs` (unchanged by this
//! fix). The blocking-pool proof at the service wiring lives in
//! `rust/fspec-tui/tests/bug187_git_state_capture_off_async_pool.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use codelet_core::git_state::GitStateWatcher;
use codelet_git::ghost_commit::create_ghost_commit;
use codelet_rpc_types::GitState;

/// Create a basic test git repository with an initial commit (mirrors
/// `rust/core/tests/git_state_watcher.rs::setup_test_repo`).
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
/// worktree (mirrors `rust/core/tests/git_state_watcher.rs`).
fn make_checkpoint(repo: &Path, work_unit_id: &str, name: &str) {
    let marker = repo.join(format!("touch-{work_unit_id}-{name}.txt"));
    fs::write(&marker, format!("{work_unit_id}/{name}")).expect("write marker");
    create_ghost_commit(repo, work_unit_id, name).expect("create_ghost_commit");
}

/// Wait (bounded) for the watcher's broadcast channel to deliver a frame
/// satisfying `pred`.
async fn wait_for_state<F: Fn(&GitState) -> bool>(
    watcher: &GitStateWatcher,
    pred: F,
    timeout: Duration,
) -> GitState {
    let mut rx = watcher.subscribe();
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
// Scenario: A slow GitState capture never blocks the other async work on
// the runtime
// ─────────────────────────────────────────────────────────────────────────

/// Proof of the BUG-187 fix, behaviorally:
///
/// Run on a SINGLE-threaded (current_thread) runtime so the watcher's poll
/// task and a concurrent timer task share ONE async worker. The first
/// capture is deliberately slow (400ms via the capture hook). If the
/// capture ran inline on the async worker (the pre-fix behavior), the
/// worker would be pinned for 400ms and the 50ms-cadence timer would show
/// a gap >= 400ms. With the capture dispatched onto the blocking pool
/// (the fix), the poll task returns immediately after `spawn_blocking` and
/// the timer keeps firing on schedule.
#[tokio::test(flavor = "current_thread")]
async fn a_slow_git_state_capture_never_blocks_the_other_async_work_on_the_runtime() {
    // @step Given a GitStateWatcher with a short poll interval watching a temp git repo
    let tmp = setup_test_repo();
    let repo = tmp.path();
    let first_capture_slow = Arc::new(AtomicBool::new(true));
    let captures = Arc::new(AtomicUsize::new(0));
    let captures_hook = Arc::clone(&captures);
    let slow_flag = Arc::clone(&first_capture_slow);
    let _watcher = GitStateWatcher::with_captures(
        repo,
        Duration::from_millis(100),
        move || {
            if slow_flag.swap(false, Ordering::SeqCst) {
                // Simulate the profiled slow capture (gix pack decode):
                // 400ms of pure sync work — far longer than the 100ms
                // poll interval.
                std::thread::sleep(Duration::from_millis(400));
            }
            captures_hook.fetch_add(1, Ordering::SeqCst);
        },
    );

    // @step And a concurrent async timer task on the same runtime firing at a short cadence
    let timer_started = std::time::Instant::now();
    let last_fire = Arc::new(std::sync::atomic::AtomicU64::new(
        timer_started.elapsed().as_millis() as u64,
    ));
    let max_gap = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let ticks = Arc::new(AtomicUsize::new(0));
    let timer = tokio::spawn({
        let last_fire = Arc::clone(&last_fire);
        let max_gap = Arc::clone(&max_gap);
        let ticks = Arc::clone(&ticks);
        let started = timer_started;
        async move {
            let mut interval = tokio::time::interval(Duration::from_millis(50));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            for _ in 0..24 {
                interval.tick().await;
                let now = started.elapsed().as_millis() as u64;
                let prev = last_fire.swap(now, Ordering::SeqCst);
                let gap = now.saturating_sub(prev);
                max_gap.fetch_max(gap, Ordering::SeqCst);
                ticks.fetch_add(1, Ordering::SeqCst);
            }
        }
    });

    // @step When the watcher keeps taking GitState snapshots for several poll intervals
    timer.await.expect("timer task joined");
    let total_elapsed = timer_started.elapsed();
    assert!(
        captures.load(Ordering::SeqCst) >= 2,
        "the poll must have captured at least twice in {total_elapsed:?} at a 100ms interval (saw {})",
        captures.load(Ordering::SeqCst)
    );

    // @step Then the concurrent timer task fires on schedule throughout (its ticks are not delayed by the captures)
    assert_eq!(
        ticks.load(Ordering::SeqCst), 24,
        "all 24 timer ticks must fire"
    );
    // 24 ticks at 50ms = 1.2s of work. A 400ms inline capture on this
    // single worker would push completion to >= ~1.6s (and produce a
    // 400ms gap). The off-pool capture keeps the worker free.
    assert!(
        total_elapsed <= Duration::from_millis(1_700),
        "the worker must not be pinned by the 400ms capture: the 1.2s timer finished in {total_elapsed:?}"
    );
    assert!(
        max_gap.load(Ordering::SeqCst) < 300,
        "the slow capture must not stall the timer: max inter-tick gap was {}ms (a 400ms inline capture on the worker would show >= 400ms)",
        max_gap.load(Ordering::SeqCst)
    );

    // @step And no tokio async worker is pinned by the capture work — the capture runs on the blocking pool (R1)
    // (The no-pin guarantee is what the gap assertions above prove
    // directly: an inline capture on this single worker WOULD produce a
    // >= 400ms timer gap and a >= 1.6s total.)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A tick that fires while a capture is in flight is dropped,
// not queued
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_tick_that_fires_while_a_capture_is_in_flight_is_dropped_not_queued() {
    // @step Given a GitStateWatcher whose capture takes longer than the poll interval
    let tmp = setup_test_repo();
    let repo = tmp.path();
    // The FIRST capture holds the lock for 600ms (>> the 100ms poll
    // interval) — it simulates a slow capture. Every later capture finds
    // the lock released and runs fast.
    let slow_guard = Arc::new(Mutex::new(()));
    let first_capture_slow = Arc::new(AtomicBool::new(true));
    let capture_starts = Arc::new(Mutex::new(Vec::<std::time::Instant>::new()));
    let captures = Arc::new(AtomicUsize::new(0));
    let watcher = GitStateWatcher::with_captures(
        repo,
        Duration::from_millis(100),
        {
            let slow_guard = Arc::clone(&slow_guard);
            let first_capture_slow = Arc::clone(&first_capture_slow);
            let capture_starts = Arc::clone(&capture_starts);
            let captures = Arc::clone(&captures);
            move || {
                capture_starts
                    .lock()
                    .expect("capture_starts poisoned")
                    .push(std::time::Instant::now());
                if first_capture_slow.swap(false, Ordering::SeqCst) {
                    // Hold the guard for 600ms — any capture that
                    // started during this window would be overlapping.
                    let _guard = slow_guard.lock().expect("slow_guard poisoned");
                    std::thread::sleep(Duration::from_millis(600));
                }
                captures.fetch_add(1, Ordering::SeqCst);
            }
        },
    );

    // @step When several poll ticks fire while the first capture is still running
    let window = Duration::from_millis(2000);
    let mut rx = watcher.subscribe();
    let deadline = tokio::time::sleep(window);
    tokio::pin!(deadline);
    loop {
        let res = tokio::select! {
            _ = &mut deadline => break,
            r = rx.recv() => r,
        };
        if matches!(
            res,
            Err(tokio::sync::broadcast::error::RecvError::Closed)
        ) {
            break;
        }
        let _ = res;
    }

    // @step Then the in-flight guard skips those ticks — at most one capture ever runs at a time (R2)
    let starts = capture_starts
        .lock()
        .expect("capture_starts poisoned")
        .clone();
    assert!(
        starts.len() >= 2,
        "at least 2 captures ran in the 2s window (saw {})",
        starts.len()
    );
    // While the first capture held the guard for 600ms, the 100ms ticks
    // (≈6 of them) could NOT start a second capture. The earliest the
    // second capture may start is ~600ms after the first.
    let gap = starts[1].saturating_duration_since(starts[0]);
    assert!(
        gap >= Duration::from_millis(550),
        "a tick landing while the first capture was in flight must be DROPPED, not queued — the second capture started {gap:?} after the first (must be >= the 600ms hold)"
    );
    // And the dropped ticks did not queue up: the third capture starts
    // a normal interval after the second (bounded slack), not in a
    // burst of the backlog.
    // @step And after the in-flight capture completes, the next tick runs a fresh capture (the guard is cleared, not stuck)
    if starts.len() >= 3 {
        let second_gap = starts[2].saturating_duration_since(starts[1]);
        assert!(
            second_gap <= Duration::from_millis(500),
            "no backlog burst after the in-flight window: capture #3 started {second_gap:?} after capture #2"
        );
    }
    // No backlog burst: as many captures ran as started (each capture
    // runs to completion — none was queued while in flight).
    let total = captures.load(Ordering::SeqCst);
    assert!(
        total >= starts.len().saturating_sub(1) && total <= starts.len(),
        "captures must not be queued behind an in-flight one: {total} completed, {} started",
        starts.len()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Steady-state unchanged ticks still cost the user nothing
// observable
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn steady_state_unchanged_ticks_still_cost_the_user_nothing_observable() {
    // @step Given a GitStateWatcher watching a temp git repo whose snapshot never changes
    let tmp = setup_test_repo();
    let repo = tmp.path();
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(100));

    // @step When many poll ticks elapse
    let mut rx = watcher.subscribe();
    tokio::time::sleep(Duration::from_millis(700)).await;

    // @step Then exactly one frame (the initial one) was ever broadcast — the off-thread capture still dedups unchanged snapshots and publishes nothing further
    // The initial broadcast landed before `rx` existed, so a silent
    // steady state means ZERO frames on this receiver over ~7 ticks.
    let mut frames = 0usize;
    while let Ok(frame) = rx.try_recv() {
        frames += 1;
        assert!(
            frame == watcher.snapshot(),
            "no-change ticks must not publish a different frame"
        );
    }
    assert_eq!(
        frames, 0,
        "unchanged off-thread poll ticks must not re-broadcast"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A checkpoint written from another terminal still lands
// within the watch window
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_checkpoint_written_from_another_terminal_still_lands_within_the_watch_window() {
    // @step Given a GitStateWatcher watching a temp git repo with no checkpoint refs
    let tmp = setup_test_repo();
    let repo = tmp.path();
    let captures = Arc::new(AtomicUsize::new(0));
    let watcher = GitStateWatcher::with_captures(
        repo,
        Duration::from_millis(100),
        {
            let captures = Arc::clone(&captures);
            move || {
                captures.fetch_add(1, Ordering::SeqCst);
            }
        },
    );
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        codelet_rpc_types::CheckpointCounts { manual: 0, auto: 0 },
        "initial snapshot must be zero counts"
    );

    // @step When a manual ghost-checkpoint ref AUTH-001/baseline is written into the repo's .git from outside the process
    make_checkpoint(repo, "AUTH-001", "baseline");

    // @step Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 0 } within the poll/fs-watch window (the off-thread capture must not delay or drop the update)
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
    // The update arrived via a capture that ran (the off-thread path is
    // the ONLY path that re-captures after construction).
    assert!(
        captures.load(Ordering::SeqCst) >= 1,
        "the post-change frame must come from a re-capture on the (blocking-pool) capture path"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A branch change in a symlinked-cwd workspace still lands via
// the fs-event path
// ─────────────────────────────────────────────────────────────────────────

/// Create a basic test git repository (mirrors `setup_test_repo`) and
/// return BOTH the real path and a symlink pointing at it.
fn setup_symlinked_repo() -> (tempfile::TempDir, tempfile::TempDir, std::path::PathBuf) {
    let real = setup_test_repo();
    // A sibling temp dir hosting the symlink; kept alive for the whole
    // test so the link target cannot disappear out from under us.
    let link_host = tempfile::TempDir::new().expect("link host tempdir");
    let link = link_host.path().join("workspace");
    std::os::unix::fs::symlink(real.path(), &link).expect("symlink");
    (real, link_host, link)
}

#[tokio::test]
async fn a_branch_change_in_a_symlinked_cwd_workspace_still_lands_via_the_fs_event_path() {
    // @step Given a GitStateWatcher whose workspace cwd sits behind a filesystem symlink (raw form differs from its canonicalized form)
    let (_real, _link_host, link) = setup_symlinked_repo();
    let raw = link.clone();
    let canon = std::fs::canonicalize(&raw).expect("canonicalize link");
    assert_ne!(
        raw, canon,
        "the test requires a cwd whose raw form differs from its canonicalized form"
    );
    let watcher = GitStateWatcher::with_interval(&link, Duration::from_millis(10_000));
    let initial_branch = {
        let out = std::process::Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&link)
            .output()
            .expect("git branch --show-current");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    assert_eq!(
        watcher.snapshot().git_branch.as_deref(),
        Some(initial_branch.as_str()),
        "initial snapshot via the symlinked cwd must read the branch"
    );

    // @step When the branch is switched to feature-x from another terminal
    std::process::Command::new("git")
        .args(["checkout", "-b", "feature-x"])
        .current_dir(&link)
        .output()
        .expect("git checkout -b feature-x");
    let start = std::time::Instant::now();

    // @step Then the watcher publishes a GitState frame with git_branch feature-x within well under one poll interval — the fs-event path is NOT silently dropped by a raw-prefix-only match (R4)
    let state = wait_for_state(
        &watcher,
        |s| s.git_branch.as_deref() == Some("feature-x"),
        // 5s << the 10s poll interval: landing this fast PROVES the
        // fs-event path fired (a raw-prefix-only match would only surface
        // the change on the first poll tick, at ~10s).
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(
        state.git_branch.as_deref(),
        Some("feature-x"),
        "the frame must carry the new branch"
    );
    assert!(
        start.elapsed() < Duration::from_secs(9),
        "the branch change must land via the fs-event path (well under one 10s poll interval), not the periodic tick: took {:?}",
        start.elapsed()
    );
}
