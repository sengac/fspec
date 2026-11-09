//! BUG-181 (SUPERSEDED by BUG-182) — live checkpoint counts ride the ONE
//! centralized `GitStateWatcher` stream into the BoardStore.
//!
//! Feature: spec/features/live-checkpoint-counts-in-the-board-header.feature
//!
//! The App and transport scenarios: the bootstrap subscriber folds pushed
//! git-state frames (MockBackend), the embedded transport delivers live
//! frames end to end (real repo, real watcher), the WebSocket transport
//! degrades on a closed receiver, and the Checkpoints view open/close and
//! RefreshCheckpointCounts re-poll the counts. The watcher-level scenarios
//! live in bug181_checkpoint_watcher_push.rs (BUG-355 split).

use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_git::ghost_commit::delete_ghost_checkpoint;
use codelet_rpc_types::{CheckpointCounts, GitState};

mod common;
use common::checkpoint_repo::{make_checkpoint, service_for, setup_test_repo};
use common::MockBackend;
use common::TaskOutcome;

mod bug181_support;
use bug181_support::{drain_bus, wait_for_board_counts};

/// BUG-657: bootstrap loads the checkpoint counts and the workspace info in
/// a tracked background task; await the bootstrap's tasks, then fold what
/// they sent.
async fn settle_bootstrap(app: &mut App) -> TestResult {
    while let Some(task) = app.next_pending_task() {
        codelet_test_helpers::deadlock_guard("a bootstrap task", task)
            .await?
            .completed_or_cancelled()?;
    }
    drain_bus(app);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: App bootstrap subscriber folds pushed checkpoint counts into
// the BoardStore
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn app_bootstrap_subscriber_folds_pushed_checkpoint_counts_into_the_board_store() -> TestResult
{
    // @step Given an App constructed with a backend whose git-state push channel is live, bootstrapped so the BoardStore holds CheckpointCounts { manual: 0, auto: 0 }
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    settle_bootstrap(&mut app).await?;
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
    wait_for_board_counts(&mut app, CheckpointCounts { manual: 1, auto: 0 }).await?;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 1, auto: 0 }
    );

    // @step And bootstrap spawns the full bootstrap subscriber set (BOOTSTRAP_SUBSCRIBER_TASKS) with the git-state stream replacing the checkpoint-changed stream (no separate checkpoint-changed subscriber remains)
    assert_eq!(
        app.subscriber_task_count(),
        common::BOOTSTRAP_SUBSCRIBER_TASKS,
        "bootstrap must spawn the git-state subscriber (replacing BUG-181's checkpoint-changed stream)"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Embedded transport delivers a live checkpoint-count frame
// end-to-end into the BoardStore
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn embedded_transport_delivers_a_live_checkpoint_count_frame_end_to_end_into_the_board_store(
) -> TestResult {
    // @step Given an App wired to an EmbeddedFspecBackend whose SharedFspecService has with_cwd on a temp git repo with <initial_manual> manual and <initial_auto> auto checkpoints, bootstrapped so the BoardStore holds CheckpointCounts { manual: <initial_manual>, auto: <initial_auto> }
    let tmp = setup_test_repo()?;
    let repo = tmp.path();
    let service = service_for(repo)?;
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    settle_bootstrap(&mut app).await?;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 0, auto: 0 }
    );

    // @step When a manual ghost-checkpoint ref is <mutation> the repo while the App is running
    make_checkpoint(repo, "AUTH-001", "baseline")?;

    // @step Then the App's git-state subscriber folds the frame and app.board_store().checkpoint_counts() returns CheckpointCounts { manual: <final_manual>, auto: <final_auto> } without any TUI restart
    wait_for_board_counts(&mut app, CheckpointCounts { manual: 1, auto: 0 }).await?;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 1, auto: 0 },
        "the live push must reach the BoardStore without a TUI restart"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: WebSocket transport degrades gracefully when the git-state
// push is not forwarded
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn websocket_transport_degrades_gracefully_when_the_git_state_push_is_not_forwarded(
) -> TestResult {
    // @step Given an App wired to a WebSocketFspecBackend (remote transport, git-state push not forwarded), bootstrapped so the BoardStore holds the bootstrap CheckpointCounts { manual: 1, auto: 1 }
    let tmp = setup_test_repo()?;
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline")?;
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-testing")?;
    let service = service_for(repo)?;
    let (addr, _join) = common::start_ws_server(service).await?;
    let backend: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(common::ws_url(addr)?)
            .await
            .or_fail("connect")?,
    );
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    settle_bootstrap(&mut app).await?;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 1, auto: 1 },
        "bootstrap value survives on the degraded transport"
    );

    // @step When a checkpoint is added to the repo on the server side after bootstrap
    make_checkpoint(repo, "AUTH-002", "live-add")?;

    // @step Then the App's git-state subscriber exits cleanly on a closed receiver without panicking or hanging
    // The WS transport returns the trait's closed-receiver default, so the
    // subscriber task terminates immediately (no panic, no hang). TEST-042:
    // wait for a closed-receiver subscriber to have ended, then read the
    // header, which must stay at the bootstrap value.
    codelet_test_helpers::wait_until("a closed-receiver subscriber to end", || {
        std::future::ready(app.finished_subscriber_task_count() > 0)
    })
    .await?;
    drain_bus(&mut app);
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 1, auto: 1 },
        "degraded transport: no push frame may mutate the BoardStore"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Opening and closing the Checkpoints view re-polls the
// checkpoint counts
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn opening_and_closing_the_checkpoints_view_re_polls_the_checkpoint_counts() -> TestResult {
    // @step Given an App with a backend whose checkpoint_counts() returns CheckpointCounts { manual: 2, auto: 1 } and a BoardStore still holding the stale { manual: 0, auto: 0 }
    let mock = Arc::new(MockBackend::new());
    mock.set_checkpoint_counts(CheckpointCounts { manual: 2, auto: 1 });
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    // Bootstrap re-reads the same (2,1) value, so re-seed the stale value
    // directly to isolate the Open/Close re-poll behaviour.
    drain_bus(&mut app);
    app.board_store_mut()
        .set_checkpoint_counts(CheckpointCounts { manual: 0, auto: 0 });

    // @step When Action::OpenCheckpointsView is dispatched and then Action::CloseCheckpointsView is dispatched
    app.dispatch(Action::OpenCheckpointsView);
    wait_for_board_counts(&mut app, CheckpointCounts { manual: 2, auto: 1 }).await?;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 2, auto: 1 },
        "opening the view must re-poll the counts"
    );
    // Re-stale the store, then close the view.
    app.board_store_mut()
        .set_checkpoint_counts(CheckpointCounts { manual: 0, auto: 0 });
    app.dispatch(Action::CloseCheckpointsView);
    wait_for_board_counts(&mut app, CheckpointCounts { manual: 2, auto: 1 }).await?;

    // @step Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 2, auto: 1 } so the header repaints with the live counts
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 2, auto: 1 },
        "closing the view must re-poll the counts (degraded-transport mitigation)"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: RefreshCheckpointCounts re-reads the counts and updates the
// BoardStore
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn refresh_checkpoint_counts_re_reads_the_counts_and_updates_the_board_store() -> TestResult {
    // @step Given an App with a backend whose checkpoint_counts() returns CheckpointCounts { manual: 3, auto: 1 } and a BoardStore still holding the stale { manual: 0, auto: 0 }
    let mock = Arc::new(MockBackend::new());
    mock.set_checkpoint_counts(CheckpointCounts { manual: 3, auto: 1 });
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    settle_bootstrap(&mut app).await?;
    app.board_store_mut()
        .set_checkpoint_counts(CheckpointCounts { manual: 0, auto: 0 });

    // @step When Action::RefreshCheckpointCounts is dispatched (the follow-up the Checkpoints view emits after a successful in-view restore or delete)
    app.dispatch(Action::RefreshCheckpointCounts);
    wait_for_board_counts(&mut app, CheckpointCounts { manual: 3, auto: 1 }).await?;

    // @step Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 3, auto: 1 }
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 3, auto: 1 },
        "the existing RefreshCheckpointCounts flow must keep landing fresh counts in the BoardStore"
    );
    assert!(mock.checkpoint_counts_calls() >= 1);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario (extra guard): delete path end-to-end through the embedded
// transport (BUG-181 example 3 — in-TUI delete is a regression guard).
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn embedded_transport_delivers_a_decremented_frame_after_a_checkpoint_delete() -> TestResult {
    // @step Given an App wired to an EmbeddedFspecBackend whose SharedFspecService has with_cwd on a temp git repo with <initial_manual> manual and <initial_auto> auto checkpoints, bootstrapped so the BoardStore holds CheckpointCounts { manual: <initial_manual>, auto: <initial_auto> }
    let tmp = setup_test_repo()?;
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline")?;
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-testing")?;
    let service = service_for(repo)?;
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    wait_for_board_counts(&mut app, CheckpointCounts { manual: 1, auto: 1 }).await?;

    // @step When a manual ghost-checkpoint ref is <mutation> the repo while the App is running
    delete_ghost_checkpoint(repo, "AUTH-001", "baseline").or_fail("delete")?;

    // @step Then the App's git-state subscriber folds the frame and app.board_store().checkpoint_counts() returns CheckpointCounts { manual: <final_manual>, auto: <final_auto> } without any TUI restart
    wait_for_board_counts(&mut app, CheckpointCounts { manual: 0, auto: 1 }).await?;
    assert_eq!(
        app.board_store().checkpoint_counts(),
        CheckpointCounts { manual: 0, auto: 1 }
    );
    Ok(())
}
