//! RPC-015 — App bootstrap + dispatch wiring for `Action::CheckpointCountsLoaded`.
//!
//! Feature: spec/features/rpc015-app-bootstrap.feature
//!
//! Drives the MockBackend's `checkpoint_counts()` return value through
//! `App::bootstrap` and asserts the BoardStore's `checkpoint_counts`
//! field is populated by `App::dispatch` on the resulting Action.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::CheckpointCounts;

mod common;
use common::MockBackend;

/// Drain the App's action bus until a matching action arrives or 200ms
/// elapses; dispatch it and return Some(action) on match, None on timeout.
async fn wait_and_dispatch<F: Fn(&Action) -> bool>(app: &mut App, pred: F) -> Option<Action> {
    let deadline = std::time::Instant::now() + Duration::from_millis(500);
    while std::time::Instant::now() < deadline {
        if let Some(action) = app.try_recv_action() {
            let matches = pred(&action);
            app.dispatch(action.clone());
            if matches {
                return Some(action);
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    None
}

/// Scenario: BoardStore.checkpoint_counts is updated by Action::CheckpointCountsLoaded
#[tokio::test]
async fn board_store_checkpoint_counts_is_updated_by_action_checkpointcountsloaded() {
    // @step Given an App constructed with a backend that returns CheckpointCounts { manual: 2, auto: 3 } from checkpoint_counts()
    let mock = Arc::new(MockBackend::new());
    mock.set_checkpoint_counts(CheckpointCounts { manual: 2, auto: 3 });
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    // @step When App::bootstrap is awaited and the bootstrap task's spawned future delivers Action::CheckpointCountsLoaded
    app.bootstrap().await.expect("bootstrap");
    // The bootstrap may fire the checkpoint_counts() call inline or onto
    // the bus — drain the bus and dispatch each Action until either we
    // see CheckpointCountsLoaded or the timeout elapses.
    let _ = wait_and_dispatch(&mut app, |a| matches!(a, Action::CheckpointCountsLoaded(_))).await;
    // @step And App::dispatch processes the action
    // (dispatched inside wait_and_dispatch)
    // @step Then app.board_store().checkpoint_counts() returns CheckpointCounts { manual: 2, auto: 3 }
    let counts = app.board_store().checkpoint_counts();
    assert_eq!(
        counts,
        CheckpointCounts { manual: 2, auto: 3 },
        "BoardStore.checkpoint_counts must reflect the value returned by the backend after bootstrap"
    );
    // Also verify the MockBackend's checkpoint_counts() was hit at least once.
    assert!(
        mock.checkpoint_counts_calls() >= 1,
        "Expected at least 1 checkpoint_counts() call, got {}",
        mock.checkpoint_counts_calls()
    );
}
