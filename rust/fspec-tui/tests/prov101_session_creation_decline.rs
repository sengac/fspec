//! PROV-101 FIX 1: the TUI surfaces a declined session creation instead of
//! swallowing it.
//!
//! Feature: spec/features/session-creation-decline-surfaced.feature
//!
//! `create_session` returns an empty `SessionId` when no default model is set
//! (decline). The TUI must NOT append an empty-id session. A shared helper maps
//! the result to `Action::SessionCreated` (real id) or
//! `Action::SessionCreationDeclined` (empty id), and the decline action pushes a
//! Priority::Critical ErrorDialog. Fully offline against the MockBackend
//! fixture — no network, no env mutation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::app::session_creation::post_create_session_action;
use codelet_fspec_tui::components::error_dialog::ERROR_DIALOG_ID;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;

mod common;
use common::MockBackend;

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

/// Drain spawned backend tasks then pump any follow-up actions back through
/// dispatch until quiescent.
async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

// =============================================================================
// Scenario: A declined session creation maps to an explicit decline action
// =============================================================================
#[test]
fn declined_result_maps_to_decline_action() {
    // @step Given a create_session result whose session id value is empty
    let result = SessionId::new("");

    // @step When the TUI builds the follow-up action for the result
    let action = post_create_session_action(result);

    // @step Then the follow-up action is the session-creation-declined action
    assert!(
        matches!(action, Action::SessionCreationDeclined),
        "empty id must map to SessionCreationDeclined, got {action:?}"
    );
}

// =============================================================================
// Scenario: A successful session creation maps to a session-created action
// =============================================================================
#[test]
fn real_result_maps_to_session_created_action() {
    // @step Given a create_session result whose session id value is not empty
    let result = SessionId::new("s-1");

    // @step When the TUI builds the follow-up action for the result
    let action = post_create_session_action(result);

    // @step Then the follow-up action is the session-created action
    assert!(
        matches!(action, Action::SessionCreated(ref s) if s.value == "s-1"),
        "non-empty id must map to SessionCreated, got {action:?}"
    );
}

// =============================================================================
// Scenario: The TUI surfaces a declined session creation as an explicit error
// =============================================================================
#[tokio::test]
async fn declined_session_creation_surfaces_error_dialog() {
    // @step Given an App whose backend declines create_session with an empty session id
    let (mut app, mock) = fresh_app();
    mock.script_create_session(SessionId::new(""));

    // @step When the user confirms creating a non-isolated session
    app.dispatch(Action::CreateSessionSubmitted { isolated: false });
    drain_pending(&mut app).await;

    // @step Then an error dialog is shown to the user
    assert!(
        app.compositor().contains(ERROR_DIALOG_ID),
        "a declined creation must surface a Priority::Critical ErrorDialog"
    );

    // @step And no session becomes the active session
    assert!(
        app.agent_view_store().current_session().is_none(),
        "no empty-id session may become the active session"
    );
}
