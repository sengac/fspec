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

    // @step And bootstrap spawns exactly 9 subscriber tasks with the git-state stream replacing the checkpoint-changed stream (no separate checkpoint-changed subscriber remains; RPC-434 adds session_removed; RPC-437 adds session_cleared)
    assert_eq!(
        app.subscriber_task_count(),
        9,
        "bootstrap must spawn the git-state subscriber (replacing BUG-181's checkpoint-changed stream) + the RPC-434 session-removed subscriber + the RPC-437 session-cleared subscriber"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Embedded transport delivers a live checkpoint-count frame
// end-to-end into the BoardStore
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn embedded_transport_delivers_a_live_checkpoint_count_frame_end_to_end_into_the_board_store()
{
    // @step Given an App wired to an EmbeddedFspecBackend whose SharedFspecService has with_cwd on a temp git repo with <initial_manual> manual and <initial_auto> auto checkpoints, bootstrapped so the BoardStore holds CheckpointCounts { manual: <initial_manual>, auto: <initial_auto> }
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

    // @step When a manual ghost-checkpoint ref is <mutation> the repo while the App is running
    make_checkpoint(repo, "AUTH-001", "baseline");

    // @step Then the App's git-state subscriber folds the frame and app.board_store().checkpoint_counts() returns CheckpointCounts { manual: <final_manual>, auto: <final_auto> } without any TUI restart
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

include!("bug181_live_checkpoint_counts/polling_and_surface.rs");

// ─────────────────────────────────────────────────────────────────────────
// Watcher-level scenarios (re-landed on the GitState stream; previously in
// rust/core/tests/checkpoints_watcher.rs, removed for the 1:1 mapping)
// ─────────────────────────────────────────────────────────────────────────

include!("bug181_live_checkpoint_counts/watcher_updates.rs");
