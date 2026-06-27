//! RPC-366 — App::dispatch wiring for the checkpoint DELETE actions.
//!
//! Feature: spec/features/checkpoint-delete-dispatch.feature
//!
//! Drives `App::dispatch(Action::DeleteCheckpoint{..})` and
//! `Action::DeleteAllCheckpoints` against a `MockBackend` that records the
//! `delete_checkpoint` / `delete_all_checkpoints` calls, asserting both
//! that the transport was hit and that a `DeleteCheckpointResult` action
//! was folded back onto the bus. Proves the dispatch without a real git
//! repo.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend};

mod common;
use common::MockBackend;

/// Drain the App's action bus until a matching action arrives or the
/// deadline elapses; dispatch every action seen so spawned follow-ups
/// run, and return Some(action) on first match.
async fn wait_for<F: Fn(&Action) -> bool>(app: &mut App, pred: F) -> Option<Action> {
    let deadline = std::time::Instant::now() + Duration::from_millis(500);
    while std::time::Instant::now() < deadline {
        if let Some(action) = app.try_recv_action() {
            let matched = pred(&action);
            app.dispatch(action.clone());
            if matched {
                return Some(action);
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    None
}

/// Scenario: Dispatching DeleteCheckpoint calls the transport delete_checkpoint
#[tokio::test]
async fn dispatching_delete_checkpoint_calls_the_transport() {
    // @step Given an App whose backend records delete calls
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When Action::DeleteCheckpoint is dispatched for a checkpoint
    app.dispatch(Action::DeleteCheckpoint {
        work_unit_id: "AUTH-001".to_string(),
        name: "baseline".to_string(),
    });

    // @step And the App emits a DeleteCheckpointResult action
    let result = wait_for(&mut app, |a| {
        matches!(a, Action::DeleteCheckpointResult { .. })
    })
    .await;
    assert!(result.is_some(), "expected a DeleteCheckpointResult on the bus");

    // @step Then the backend delete_checkpoint is called for that checkpoint
    assert_eq!(
        mock.delete_checkpoint_calls(),
        1,
        "delete_checkpoint should be called exactly once"
    );
    assert_eq!(
        mock.last_delete_checkpoint(),
        Some(("AUTH-001".to_string(), "baseline".to_string()))
    );
}

/// Scenario: Dispatching DeleteAllCheckpoints calls the transport delete_all_checkpoints
#[tokio::test]
async fn dispatching_delete_all_checkpoints_calls_the_transport() {
    // @step Given an App whose backend records delete calls
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When Action::DeleteAllCheckpoints is dispatched
    app.dispatch(Action::DeleteAllCheckpoints);

    // @step And the App emits a DeleteCheckpointResult action
    let result = wait_for(&mut app, |a| {
        matches!(a, Action::DeleteCheckpointResult { .. })
    })
    .await;
    assert!(result.is_some(), "expected a DeleteCheckpointResult on the bus");

    // @step Then the backend delete_all_checkpoints is called
    assert_eq!(
        mock.delete_all_checkpoints_calls(),
        1,
        "delete_all_checkpoints should be called exactly once"
    );
}
