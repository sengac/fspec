//! RPC-365 — App::dispatch wiring for `Action::RestoreCheckpointFile`.
//!
//! Feature: spec/features/checkpoint-restore-dispatch.feature
//!
//! Drives `App::dispatch(Action::RestoreCheckpointFile{..})` against a
//! `MockBackend` that records `restore_checkpoint_file` calls, and
//! asserts both that the transport was hit with the right path and that
//! a `RestoreCheckpointResult` action was folded back onto the bus.
//! This proves the dispatch without a real git repo.

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

/// Scenario: Dispatching RestoreCheckpointFile calls the transport restore_checkpoint_file
#[tokio::test]
async fn dispatching_restore_checkpoint_file_calls_the_transport() {
    // @step Given an App whose backend records restore calls
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When Action::RestoreCheckpointFile is dispatched for a.txt
    app.dispatch(Action::RestoreCheckpointFile {
        work_unit_id: "AUTH-001".to_string(),
        name: "baseline".to_string(),
        path: "a.txt".to_string(),
    });

    // @step Then the backend restore_checkpoint_file is called for a.txt
    let result = wait_for(&mut app, |a| {
        matches!(a, Action::RestoreCheckpointResult { .. })
    })
    .await;
    assert_eq!(
        mock.restore_checkpoint_file_calls(),
        1,
        "restore_checkpoint_file should be called exactly once"
    );

    // @step And the App emits a RestoreCheckpointResult action
    assert!(
        result.is_some(),
        "expected a RestoreCheckpointResult on the bus"
    );
    assert_eq!(
        mock.last_restore_file(),
        Some((
            "AUTH-001".to_string(),
            "baseline".to_string(),
            "a.txt".to_string()
        ))
    );
}

/// Scenario: Dispatching RestoreCheckpointAll calls the transport restore_checkpoint_all
#[tokio::test]
async fn dispatching_restore_checkpoint_all_calls_the_transport() {
    // @step Given an App whose backend records restore calls
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When Action::RestoreCheckpointAll is dispatched
    app.dispatch(Action::RestoreCheckpointAll {
        work_unit_id: "AUTH-001".to_string(),
        name: "baseline".to_string(),
    });

    // @step Then the backend restore_checkpoint_all is called once
    let result = wait_for(&mut app, |a| {
        matches!(a, Action::RestoreCheckpointResult { .. })
    })
    .await;
    assert_eq!(
        mock.restore_checkpoint_all_calls(),
        1,
        "restore_checkpoint_all should be called exactly once"
    );

    // @step And the App emits a RestoreCheckpointResult action
    assert!(
        result.is_some(),
        "expected a RestoreCheckpointResult on the bus"
    );
}
