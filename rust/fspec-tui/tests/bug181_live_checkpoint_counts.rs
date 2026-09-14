//! BUG-181 (SUPERSEDED by BUG-182) — live checkpoint counts now ride the
//! ONE centralized `GitStateWatcher` stream.
//!
//! Feature: spec/features/live-checkpoint-counts-in-the-board-header.feature
//!
//! Covers the feature's TUI/RPC/transport scenarios (re-landed on the
//! GitState stream):
//!   - App bootstrap subscriber folds pushed git-state frames into the
//!     BoardStore (MockBackend-driven);
//!   - Embedded transport delivers a live checkpoint-count frame
//!     end-to-end into the BoardStore (real repo, real watcher);
//!   - WebSocket transport degrades gracefully when the git-state push
//!     is not forwarded (closed receiver, no panic, no hang);
//!   - Opening/closing the Checkpoints view re-polls the counts;
//!   - RefreshCheckpointCounts re-reads the counts and updates the
//!     BoardStore (regression guard for the existing RPC-365/366 flow);
//!   - Source-shape: FspecBackend::git_state_changed_rx exists,
//!     SharedFspecService exposes the accessors, and app/bootstrap.rs
//!     subscribes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use codelet_core::git_state::GitStateWatcher;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_git::ghost_commit::{create_ghost_commit, delete_ghost_checkpoint};
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::{CheckpointCounts, GitState};
use tempfile::TempDir;

mod common;
use common::MockBackend;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Create a basic test git repository with an initial commit (mirrors
/// `rust/git/tests/common/mod.rs::setup_test_repo`).
fn setup_test_repo() -> TempDir {
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

/// Build a `SharedFspecService` with a real (empty) watcher + the repo
/// cwd attached, so the GitStateWatcher has a live cwd to read.
fn service_for(repo: &Path) -> Arc<SharedFspecService> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo).expect("watcher on temp repo"));
    Arc::new(SharedFspecService::new(watcher).with_cwd(repo.to_path_buf()))
}

/// Dispatch every action currently on the bus (best-effort, no wait).
fn drain_bus(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

/// Wait for the BoardStore's checkpoint counts to reach `want`,
/// dispatching whatever actions arrive on the bus along the way.
async fn wait_for_board_counts(app: &mut App, want: CheckpointCounts, timeout: Duration) {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        drain_bus(app);
        if app.board_store().checkpoint_counts() == want {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!(
        "timeout: BoardStore counts did not reach {want:?} (last {:?})",
        app.board_store().checkpoint_counts()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: App bootstrap subscriber folds pushed checkpoint counts into
// the BoardStore
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn app_bootstrap_subscriber_folds_pushed_checkpoint_counts_into_the_board_store() {
    // @step Given an App constructed with a backend whose git-state push channel is live, bootstrapped so the BoardStore holds CheckpointCounts { manual: 0, auto: 0 }
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    drain_bus(&mut app);
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 0, auto: 0 },
        "bootstrap leaves the BoardStore at zero counts (MockBackend default)"
    );

    // @step When a GitState frame with checkpoint_counts { manual: 1, auto: 0 } is pushed onto the channel
    mock.push_git_state_changed(GitState {
        checkpoint_counts: CheckpointCounts { manual: 1, auto: 0 },
        git_branch: None,
        changed_files: Vec::new(),
        checkpoints: Vec::new(),
    });

    // @step Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 1, auto: 0 }
    wait_for_board_counts(
        &mut app,
        CheckpointCounts { manual: 1, auto: 0 },
        Duration::from_millis(500),
    )
    .await;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 1, auto: 0 }
    );

    // @step And bootstrap spawns exactly 7 subscriber tasks with the git-state stream replacing the checkpoint-changed stream (no separate checkpoint-changed subscriber remains)
    assert_eq!(
        app.subscriber_task_count(),
        7,
        "bootstrap must spawn the git-state subscriber (replacing BUG-181's checkpoint-changed stream)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Embedded transport delivers a live checkpoint-count frame
// end-to-end into the BoardStore
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn embedded_transport_delivers_a_live_checkpoint_count_frame_end_to_end_into_the_board_store()
{
    // @step Given an App wired to an EmbeddedFspecBackend whose SharedFspecService has with_cwd on a temp git repo with no checkpoints, bootstrapped so the BoardStore holds CheckpointCounts { manual: 0, auto: 0 }
    let tmp = setup_test_repo();
    let repo = tmp.path();
    let service = service_for(repo);
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    drain_bus(&mut app);
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 0, auto: 0 }
    );

    // @step When a manual ghost-checkpoint ref is written into the repo while the App is running
    make_checkpoint(repo, "AUTH-001", "baseline");

    // @step Then the App's git-state subscriber folds the frame and app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 1, auto: 0 } without any TUI restart
    wait_for_board_counts(
        &mut app,
        CheckpointCounts { manual: 1, auto: 0 },
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 1, auto: 0 },
        "the live push must reach the BoardStore without a TUI restart"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: WebSocket transport degrades gracefully when the git-state
// push is not forwarded
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn websocket_transport_degrades_gracefully_when_the_git_state_push_is_not_forwarded() {
    // @step Given an App wired to a WebSocketFspecBackend (remote transport, git-state push not forwarded), bootstrapped so the BoardStore holds the bootstrap CheckpointCounts { manual: 1, auto: 1 }
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-testing");
    let service = service_for(repo);
    let (addr, _join) = common::start_ws_server(service).await;
    let backend: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(common::ws_url(addr))
            .await
            .expect("connect"),
    );
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    drain_bus(&mut app);
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 1, auto: 1 },
        "bootstrap value survives on the degraded transport"
    );

    // @step When a checkpoint is added to the repo on the server side after bootstrap
    make_checkpoint(repo, "AUTH-002", "live-add");

    // @step Then the App's git-state subscriber exits cleanly on a closed receiver without panicking or hanging
    // The WS transport returns the trait's closed-receiver default, so the
    // subscriber task terminates immediately (no panic, no hang). Give it
    // a bounded window; the header must stay at the bootstrap value.
    tokio::time::sleep(Duration::from_millis(500)).await;
    drain_bus(&mut app);
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 1, auto: 1 },
        "degraded transport: no push frame may mutate the BoardStore"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Opening and closing the Checkpoints view re-polls the
// checkpoint counts
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn opening_and_closing_the_checkpoints_view_re_polls_the_checkpoint_counts() {
    // @step Given an App with a backend whose checkpoint_counts() returns CheckpointCounts { manual: 2, auto: 1 } and a BoardStore still holding the stale { manual: 0, auto: 0 }
    let mock = Arc::new(MockBackend::new());
    mock.set_checkpoint_counts(CheckpointCounts { manual: 2, auto: 1 });
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    // Bootstrap re-reads the same (2,1) value, so re-seed the stale value
    // directly to isolate the Open/Close re-poll behaviour.
    drain_bus(&mut app);
    app.board_store_mut()
        .set_checkpoint_counts(CheckpointCounts { manual: 0, auto: 0 });

    // @step When Action::OpenCheckpointsView is dispatched and then Action::CloseCheckpointsView is dispatched
    app.dispatch(Action::OpenCheckpointsView);
    wait_for_board_counts(
        &mut app,
        CheckpointCounts { manual: 2, auto: 1 },
        Duration::from_millis(1000),
    )
    .await;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 2, auto: 1 },
        "opening the view must re-poll the counts"
    );
    // Re-stale the store, then close the view.
    app.board_store_mut()
        .set_checkpoint_counts(CheckpointCounts { manual: 0, auto: 0 });
    app.dispatch(Action::CloseCheckpointsView);
    wait_for_board_counts(
        &mut app,
        CheckpointCounts { manual: 2, auto: 1 },
        Duration::from_millis(1000),
    )
    .await;

    // @step Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 2, auto: 1 } so the header repaints with the live counts
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 2, auto: 1 },
        "closing the view must re-poll the counts (degraded-transport mitigation)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: RefreshCheckpointCounts re-reads the counts and updates the
// BoardStore
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn refresh_checkpoint_counts_re_reads_the_counts_and_updates_the_board_store() {
    // @step Given an App with a backend whose checkpoint_counts() returns CheckpointCounts { manual: 3, auto: 1 } and a BoardStore still holding the stale { manual: 0, auto: 0 }
    let mock = Arc::new(MockBackend::new());
    mock.set_checkpoint_counts(CheckpointCounts { manual: 3, auto: 1 });
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    drain_bus(&mut app);
    app.board_store_mut()
        .set_checkpoint_counts(CheckpointCounts { manual: 0, auto: 0 });

    // @step When Action::RefreshCheckpointCounts is dispatched (the follow-up the Checkpoints view emits after a successful in-view restore or delete)
    app.dispatch(Action::RefreshCheckpointCounts);
    wait_for_board_counts(
        &mut app,
        CheckpointCounts { manual: 3, auto: 1 },
        Duration::from_millis(1000),
    )
    .await;

    // @step Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 3, auto: 1 }
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 3, auto: 1 },
        "the existing RefreshCheckpointCounts flow must keep landing fresh counts in the BoardStore"
    );
    assert!(mock.checkpoint_counts_calls() >= 1);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: FspecBackend exposes a transport-agnostic git-state push
// surface (source-shape)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn fspec_backend_exposes_a_transport_agnostic_git_state_push_surface() {
    // @step Given the rust/fspec-tui, rust/rpc, and rust/core source trees after BUG-182 lands
    let root = common::workspace_root();
    let transport_mod = common::read_to_string_or_panic(
        &root
            .join("fspec-tui")
            .join("src")
            .join("transport")
            .join("mod.rs"),
    );
    let rpc_lib = common::read_to_string_or_panic(&root.join("rpc").join("src").join("lib.rs"));
    let core_watcher =
        common::read_to_string_or_panic(&root.join("core").join("src").join("git_state.rs"));
    let bootstrap = common::read_to_string_or_panic(
        &root
            .join("fspec-tui")
            .join("src")
            .join("app")
            .join("bootstrap.rs"),
    );

    // @step When a developer scans the source trees
    // @step Then the FspecBackend trait in fspec-tui/src/transport/mod.rs contains the method git_state_changed_rx returning broadcast::Receiver<GitState>
    assert!(
        common::strip_rust_comments(&transport_mod)
            .contains("fn git_state_changed_rx(&self) -> broadcast::Receiver<GitState>"),
        "FspecBackend must declare the git-state push receiver"
    );

    // @step And SharedFspecService in rpc/src/lib.rs exposes git_state_changed_rx and git_state_snapshot
    assert!(
        rpc_lib.contains("pub fn git_state_changed_rx"),
        "SharedFspecService must expose git_state_changed_rx()"
    );
    assert!(
        rpc_lib.contains("pub fn git_state_snapshot"),
        "SharedFspecService must expose git_state_snapshot()"
    );

    // @step And the GitStateWatcher type exists in codelet-core and app/bootstrap.rs subscribes to git_state_changed_rx
    assert!(
        core_watcher.contains("pub struct GitStateWatcher"),
        "codelet-core must define the GitStateWatcher type"
    );
    assert!(
        bootstrap.contains("git_state_changed_rx"),
        "app/bootstrap.rs must subscribe to git_state_changed_rx"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario (extra guard): delete path end-to-end through the embedded
// transport (BUG-181 example 3 — in-TUI delete is a regression guard).
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn embedded_transport_delivers_a_decremented_frame_after_a_checkpoint_delete() {
    // @step Given an App wired to an EmbeddedFspecBackend on a temp git repo with 1 manual + 1 auto checkpoint, bootstrapped to { manual: 1, auto: 1 }
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-testing");
    let service = service_for(repo);
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    wait_for_board_counts(
        &mut app,
        CheckpointCounts { manual: 1, auto: 1 },
        Duration::from_secs(5),
    )
    .await;

    // @step When the manual checkpoint ref is deleted while the App is running
    delete_ghost_checkpoint(repo, "AUTH-001", "baseline").expect("delete");

    // @step Then the BoardStore reflects the decremented counts
    wait_for_board_counts(
        &mut app,
        CheckpointCounts { manual: 0, auto: 1 },
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 0, auto: 1 }
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Watcher-level scenarios (re-landed on the GitState stream; previously in
// rust/core/tests/checkpoints_watcher.rs, removed for the 1:1 mapping)
// ─────────────────────────────────────────────────────────────────────────

/// Wait (bounded) for the watcher's broadcast channel to deliver a frame
/// whose `checkpoint_counts` equals `want`. Frames are full snapshots, so
/// the first match is the authoritative post-change state.
async fn wait_for_counts(watcher: &GitStateWatcher, want: CheckpointCounts) -> CheckpointCounts {
    let mut rx = watcher.subscribe();
    // The initial snapshot was broadcast before this receiver existed;
    // backfill so a no-change scenario still resolves.
    let mut got = watcher.snapshot().checkpoint_counts;
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
            Ok(GitState {
                checkpoint_counts, ..
            }) => {
                got = checkpoint_counts;
                if got == want {
                    return got;
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
    // @step Given a GitStateWatcher watching a temp git repo that has no checkpoint refs
    let tmp = setup_test_repo();
    let repo = tmp.path();
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(250));
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        CheckpointCounts { manual: 0, auto: 0 },
        "initial snapshot must be zero counts"
    );

    // @step When a manual ghost-checkpoint ref AUTH-001/baseline is written into the repo (as fspec checkpoint AUTH-001 baseline does)
    make_checkpoint(repo, "AUTH-001", "baseline");

    // @step Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 0 } on its broadcast channel
    let counts = wait_for_counts(&watcher, CheckpointCounts { manual: 1, auto: 0 }).await;
    assert_eq!(counts, CheckpointCounts { manual: 1, auto: 0 });
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Auto-checkpoint creation publishes incremented counts on the
// checkpoint-changed push
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn auto_checkpoint_creation_publishes_incremented_counts_on_the_checkpoint_changed_push() {
    // @step Given a GitStateWatcher watching a temp git repo that has one manual checkpoint (AUTH-001/baseline)
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(250));
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        CheckpointCounts { manual: 1, auto: 0 },
        "initial snapshot must reflect the existing manual checkpoint"
    );

    // @step When an automatic checkpoint ref AUTH-001/AUTH-001-auto-testing is written into the repo (as update-work-unit-status does before the transition)
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-testing");

    // @step Then the watcher publishes a GitState frame with checkpoint_counts { manual: 1, auto: 1 } on its broadcast channel
    wait_for_counts(&watcher, CheckpointCounts { manual: 1, auto: 1 }).await;
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        CheckpointCounts { manual: 1, auto: 1 }
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Checkpoint delete publishes decremented counts on the
// checkpoint-changed push
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn checkpoint_delete_publishes_decremented_counts_on_the_checkpoint_changed_push() {
    // @step Given a GitStateWatcher watching a temp git repo that has one manual + one auto checkpoint (AUTH-001/baseline, AUTH-001/AUTH-001-auto-testing)
    let tmp = setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-testing");
    let watcher = GitStateWatcher::with_interval(repo, Duration::from_millis(250));
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        CheckpointCounts { manual: 1, auto: 1 }
    );

    // @step When the manual checkpoint ref is deleted from the repo (as fspec cleanup-checkpoints / TUI delete do)
    delete_ghost_checkpoint(repo, "AUTH-001", "baseline").expect("delete");

    // @step Then the watcher publishes a GitState frame with checkpoint_counts { manual: 0, auto: 1 } on its broadcast channel
    wait_for_counts(&watcher, CheckpointCounts { manual: 0, auto: 1 }).await;
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        CheckpointCounts { manual: 0, auto: 1 }
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Watcher degrades to an empty GitState when the watched
// directory is not a git repository
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn watcher_degrades_to_an_empty_git_state_when_the_watched_directory_is_not_a_git_repository()
{
    // @step Given a GitStateWatcher watching a plain temp directory that has no .git
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let dir = tmp.path();
    let watcher = GitStateWatcher::with_interval(dir, Duration::from_millis(250));

    // @step When the watcher's initial snapshot is read and a file is later created inside that directory
    let initial = watcher.snapshot();
    fs::write(dir.join("plain.txt"), "not a repo\n").expect("write file");

    // @step Then the initial snapshot is an empty GitState (zero checkpoint counts, empty changed_files and checkpoints lists)
    assert_eq!(
        initial.checkpoint_counts,
        CheckpointCounts { manual: 0, auto: 0 },
        "initial snapshot of a non-repo cwd must be zero counts with no error"
    );
    assert!(initial.changed_files.is_empty());
    assert!(initial.checkpoints.is_empty());
    assert_eq!(
        watcher.snapshot().checkpoint_counts,
        CheckpointCounts { manual: 0, auto: 0 }
    );
}
