//! BUG-181 — Live checkpoint counts in the board header (TUI + RPC +
//! transport layers).
//!
//! Feature: spec/features/live-checkpoint-counts-in-the-board-header.feature
//!
//! Covers the feature's TUI/RPC/transport scenarios:
//!   - App bootstrap subscriber folds pushed checkpoint counts into the
//!     BoardStore (7th subscriber task, MockBackend-driven);
//!   - Embedded transport delivers a live checkpoint-count frame
//!     end-to-end into the BoardStore (real repo, real watcher);
//!   - WebSocket transport degrades gracefully when the
//!     checkpoint-changed push is not forwarded (closed receiver, no
//!     panic, no hang — documented like TUI-109);
//!   - Opening/closing the Checkpoints view re-polls the counts;
//!   - RefreshCheckpointCounts re-reads the counts and updates the
//!     BoardStore (regression guard for the existing RPC-365/366 flow);
//!   - Source-shape: FspecBackend::checkpoint_counts_changed_rx exists,
//!     SharedFspecService exposes the accessors, and app/bootstrap.rs
//!     subscribes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_git::ghost_commit::{create_ghost_commit, delete_ghost_checkpoint};
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::CheckpointCounts;
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
/// cwd attached, so checkpoint counts have a live cwd to read.
fn service_for(repo: &Path) -> Arc<SharedFspecService> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo).expect("watcher on temp repo"));
    Arc::new(SharedFspecService::new(watcher).with_cwd(repo.to_path_buf()))
}

/// Drain the App's action bus until a matching action arrives or
/// `timeout` elapses (mirrors `app_bootstrap_rpc009.rs::wait_for_action`).
async fn wait_for_action<F: Fn(&Action) -> bool>(
    app: &mut App,
    pred: F,
    timeout: Duration,
) -> Option<Action> {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if let Some(action) = app.try_recv_action() {
            if pred(&action) {
                return Some(action);
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    None
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
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: App bootstrap subscriber folds pushed checkpoint counts into
// the BoardStore
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn app_bootstrap_subscriber_folds_pushed_checkpoint_counts_into_the_board_store() {
    // @step Given an App constructed with a backend whose checkpoint-changed push channel is live, bootstrapped so the BoardStore holds CheckpointCounts { manual: 0, auto: 0 }
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

    // @step When a frame CheckpointCounts { manual: 1, auto: 0 } is pushed onto the channel
    mock.push_checkpoint_counts_changed(CheckpointCounts { manual: 1, auto: 0 });
    let action = wait_for_action(
        &mut app,
        |a| matches!(a, Action::CheckpointCountsLoaded(_)),
        Duration::from_millis(500),
    )
    .await
    .expect("CheckpointCountsLoaded forwarded by the 7th subscriber");
    app.dispatch(action);

    // @step Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 1, auto: 0 }
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 1, auto: 0 }
    );

    // @step And bootstrap spawns a 7th subscriber task on the checkpoint-changed channel alongside the existing six
    assert_eq!(
        app.subscriber_task_count(),
        7,
        "bootstrap must spawn the checkpoint-changed subscriber task (TUI-109's 6 + BUG-181's 1)"
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

    // @step Then the App's checkpoint-changed subscriber forwards the frame and app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 1, auto: 0 } without any TUI restart
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
// Scenario: WebSocket transport degrades gracefully when the
// checkpoint-changed push is not forwarded
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn websocket_transport_degrades_gracefully_when_the_checkpoint_changed_push_is_not_forwarded()
{
    // @step Given an App wired to a WebSocketFspecBackend (remote transport, checkpoint-changed push not forwarded), bootstrapped so the BoardStore holds the bootstrap CheckpointCounts { manual: 1, auto: 1 }
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

    // @step Then the App's checkpoint-changed subscriber exits cleanly on a closed receiver without panicking or hanging
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
// Scenario: FspecBackend exposes a transport-agnostic checkpoint-changed
// push surface (source-shape)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn fspec_backend_exposes_a_transport_agnostic_checkpoint_changed_push_surface() {
    // @step Given the rust/fspec-tui, rust/rpc, and rust/core source trees after BUG-181 lands
    let root = common::workspace_root();
    let transport_mod = common::read_to_string_or_panic(
        &root
            .join("fspec-tui")
            .join("src")
            .join("transport")
            .join("mod.rs"),
    );
    let rpc_lib = common::read_to_string_or_panic(&root.join("rpc").join("src").join("lib.rs"));
    let core_watcher = common::read_to_string_or_panic(
        &root.join("core").join("src").join("checkpoints_watcher.rs"),
    );
    let bootstrap = common::read_to_string_or_panic(
        &root
            .join("fspec-tui")
            .join("src")
            .join("app")
            .join("bootstrap.rs"),
    );

    // @step When a developer scans the source trees
    // @step Then the FspecBackend trait in fspec-tui/src/transport/mod.rs contains the method checkpoint_counts_changed_rx returning broadcast::Receiver<CheckpointCounts>
    assert!(
        common::strip_rust_comments(&transport_mod).contains(
            "fn checkpoint_counts_changed_rx(&self) -> broadcast::Receiver<CheckpointCounts>"
        ),
        "FspecBackend must declare the checkpoint-changed push receiver"
    );

    // @step And SharedFspecService in rpc/src/lib.rs exposes checkpoint_counts_changed_rx and checkpoint_counts_snapshot
    assert!(
        rpc_lib.contains("pub fn checkpoint_counts_changed_rx"),
        "SharedFspecService must expose checkpoint_counts_changed_rx()"
    );
    assert!(
        rpc_lib.contains("pub fn checkpoint_counts_snapshot"),
        "SharedFspecService must expose checkpoint_counts_snapshot()"
    );

    // @step And the CheckpointsWatcher type exists in codelet-core and app/bootstrap.rs subscribes to checkpoint_counts_changed_rx
    assert!(
        core_watcher.contains("pub struct CheckpointsWatcher"),
        "codelet-core must define the CheckpointsWatcher type"
    );
    assert!(
        bootstrap.contains("checkpoint_counts_changed_rx"),
        "app/bootstrap.rs must subscribe to checkpoint_counts_changed_rx"
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
