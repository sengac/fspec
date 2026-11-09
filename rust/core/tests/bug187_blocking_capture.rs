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

/// Wait (bounded) for the watcher's broadcast channel to deliver a frame
/// satisfying `pred`.
async fn wait_for_state<F: Fn(&GitState) -> bool>(watcher: &GitStateWatcher, pred: F) -> GitState {
    let mut rx = watcher.subscribe();
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
// Scenario: A slow GitState capture never blocks the other async work on
// the runtime
// ─────────────────────────────────────────────────────────────────────────

/// A capture gate for the tests below: the FIRST re-capture blocks until the
/// test releases it (TEST-042: an event the test controls, not a sleep);
/// later captures run straight through. Every capture start and finish is
/// counted. Only the deadlock guard ends a first capture nobody releases.
struct SlowFirstCapture {
    starts: AtomicUsize,
    finishes: AtomicUsize,
    entered: AtomicBool,
    release: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
}

impl SlowFirstCapture {
    fn new() -> (Arc<Self>, std::sync::mpsc::Sender<()>) {
        let (release, released) = std::sync::mpsc::channel();
        let gate = Arc::new(Self {
            starts: AtomicUsize::new(0),
            finishes: AtomicUsize::new(0),
            entered: AtomicBool::new(false),
            release: Mutex::new(Some(released)),
        });
        (gate, release)
    }

    fn capture(&self) {
        self.starts.fetch_add(1, Ordering::SeqCst);
        let first = self.release.lock().expect("release lock poisoned").take();
        if let Some(released) = first {
            self.entered.store(true, Ordering::SeqCst);
            let _ =
                codelet_test_helpers::deadlock_guard_blocking("the capture release", move || {
                    released.recv()
                });
        }
        self.finishes.fetch_add(1, Ordering::SeqCst);
    }
}

/// On the paused clock: move time forward in small steps until `ready`
/// holds (TEST-042: virtual time drives the poll ticks; the deadlock guard
/// ends a state that never comes).
async fn advance_until(what: &str, ready: impl Fn() -> bool) {
    codelet_test_helpers::deadlock_guard(what, async {
        while !ready() {
            tokio::time::advance(Duration::from_millis(10)).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect(what);
}

/// Proof of the BUG-187 fix, behaviorally:
///
/// Run on a SINGLE-threaded (current_thread) runtime so the watcher's poll
/// task and the test share ONE async worker. The first capture blocks until
/// the test releases it. If the capture ran inline on the async worker (the
/// pre-fix behavior), the worker would be pinned and no other task could
/// run until the release. With the capture dispatched onto the blocking pool
/// (the fix), another task runs to completion while the capture is held.
#[tokio::test(flavor = "current_thread")]
async fn a_slow_git_state_capture_never_blocks_the_other_async_work_on_the_runtime() {
    // @step Given a GitStateWatcher with a short poll interval watching a temp git repo
    let tmp = setup_test_repo();
    let repo = tmp.path();
    let (gate, release) = SlowFirstCapture::new();
    let hook = Arc::clone(&gate);
    let _watcher =
        GitStateWatcher::with_captures(repo, Duration::from_millis(100), move || hook.capture());

    // @step And a concurrent async timer task on the same runtime firing at a short cadence
    // @step When the watcher keeps taking GitState snapshots for several poll intervals
    let entered = Arc::clone(&gate);
    codelet_test_helpers::wait_until("the first re-capture starts", || {
        std::future::ready(entered.entered.load(Ordering::SeqCst))
    })
    .await
    .expect("the poll re-captures");

    // @step Then the concurrent timer task fires on schedule throughout (its ticks are not delayed by the captures)
    // While the capture is held, another task on the only worker runs to
    // completion: the capture is not pinning the worker.
    let other = codelet_test_helpers::deadlock_guard(
        "another task while the capture is held",
        tokio::spawn(async { 24 }),
    )
    .await
    .expect("the worker is free while the capture is held")
    .expect("the other task completes");
    assert_eq!(other, 24);
    assert_eq!(
        gate.finishes.load(Ordering::SeqCst),
        0,
        "the first capture is still held"
    );

    // @step And no tokio async worker is pinned by the capture work — the capture runs on the blocking pool (R1)
    release.send(()).expect("release the capture");
    let finished = Arc::clone(&gate);
    codelet_test_helpers::wait_until("a second capture completes", || {
        std::future::ready(finished.finishes.load(Ordering::SeqCst) >= 2)
    })
    .await
    .expect("the poll keeps capturing after the release");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A tick that fires while a capture is in flight is dropped,
// not queued
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(start_paused = true)] // TEST-042: virtual time
async fn a_tick_that_fires_while_a_capture_is_in_flight_is_dropped_not_queued() {
    // @step Given a GitStateWatcher whose capture takes longer than the poll interval
    let tmp = setup_test_repo();
    let repo = tmp.path();
    // The FIRST re-capture is held until the test releases it; every later
    // capture runs fast.
    let (gate, release) = SlowFirstCapture::new();
    let hook = Arc::clone(&gate);
    let _watcher =
        GitStateWatcher::with_captures(repo, Duration::from_millis(100), move || hook.capture());
    advance_until("the first re-capture starts", || {
        gate.entered.load(Ordering::SeqCst)
    })
    .await;

    // @step When several poll ticks fire while the first capture is still running
    // Six 100 ms ticks on the paused clock while the capture is held (the
    // clock does not auto-advance while a blocking task is in flight, so
    // it is moved explicitly).
    for _ in 0..6 {
        tokio::time::advance(Duration::from_millis(100)).await;
        tokio::task::yield_now().await;
    }
    for _ in 0..16 {
        tokio::task::yield_now().await;
    }

    // @step Then the in-flight guard skips those ticks — at most one capture ever runs at a time (R2)
    assert_eq!(
        gate.starts.load(Ordering::SeqCst),
        1,
        "a tick landing while the first capture was in flight must be DROPPED, not queued"
    );

    // @step And after the in-flight capture completes, the next tick runs a fresh capture (the guard is cleared, not stuck)
    release.send(()).expect("release the capture");
    // The release is seen on the blocking-pool thread; wait for it without
    // moving the clock.
    codelet_test_helpers::wait_until("the held capture completes", || {
        std::future::ready(gate.finishes.load(Ordering::SeqCst) >= 1)
    })
    .await
    .expect("the held capture completes");
    // No backlog burst: before the next tick nothing else has started.
    for _ in 0..16 {
        tokio::task::yield_now().await;
    }
    assert_eq!(
        gate.starts.load(Ordering::SeqCst),
        1,
        "dropped ticks must not queue a burst of captures after the release"
    );
    advance_until("the next tick captures again", || {
        gate.finishes.load(Ordering::SeqCst) >= 2
    })
    .await;
    assert_eq!(
        gate.starts.load(Ordering::SeqCst),
        gate.finishes.load(Ordering::SeqCst),
        "captures must not be queued behind an in-flight one"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Steady-state unchanged ticks still cost the user nothing
// observable
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(start_paused = true)] // TEST-042: virtual time
async fn steady_state_unchanged_ticks_still_cost_the_user_nothing_observable() {
    // @step Given a GitStateWatcher watching a temp git repo whose snapshot never changes
    let tmp = setup_test_repo();
    let repo = tmp.path();
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(100));
    // BUG-657: the initial frame is broadcast when the off-thread first
    // capture lands.
    let (_initial, mut rx) = first_frame(&watcher).await;

    // @step When many poll ticks elapse
    tokio::time::sleep(Duration::from_millis(700)).await;

    // @step Then exactly one frame (the initial one) was ever broadcast — the off-thread capture still dedups unchanged snapshots and publishes nothing further
    // The initial frame is counted; a silent steady state means no
    // further frame on this receiver over ~7 ticks.
    let mut frames = 1usize;
    while let Ok(frame) = rx.try_recv() {
        frames += 1;
        assert!(
            frame == watcher.snapshot(),
            "no-change ticks must not publish a different frame"
        );
    }
    assert_eq!(
        frames, 1,
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
    let watcher = GitStateWatcher::with_captures(repo, Duration::from_millis(100), {
        let captures = Arc::clone(&captures);
        move || {
            captures.fetch_add(1, Ordering::SeqCst);
        }
    });
    assert_eq!(
        first_frame(&watcher).await.0.checkpoint_counts,
        codelet_rpc_types::CheckpointCounts { manual: 0, auto: 0 },
        "initial snapshot must be zero counts"
    );

    // @step When a manual ghost-checkpoint ref AUTH-001/baseline is written into the repo's .git from outside the process
    make_checkpoint(repo, "AUTH-001", "baseline");

    // @step Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 0 } within the poll/fs-watch window (the off-thread capture must not delay or drop the update)
    let state = wait_for_state(&watcher, |s| {
        s.checkpoint_counts == codelet_rpc_types::CheckpointCounts { manual: 1, auto: 0 }
    })
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
    // TEST-042: the periodic poll is a day apart, so once the first capture
    // (BUG-657: off-thread, always published) has landed only the fs-event
    // path can deliver a change: the path is proven structurally, not by a
    // time bound.
    let watcher = GitStateWatcher::with_interval(&link, Duration::from_secs(24 * 60 * 60));
    let _ = first_frame(&watcher).await;
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

    // @step Then the watcher publishes a GitState frame with git_branch feature-x within well under one poll interval — the fs-event path is NOT silently dropped by a raw-prefix-only match (R4)
    let state = wait_for_state(&watcher, |s| s.git_branch.as_deref() == Some("feature-x")).await;
    assert_eq!(
        state.git_branch.as_deref(),
        Some("feature-x"),
        "the frame must carry the new branch"
    );
}
