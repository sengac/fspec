//! MODEL-006 — Selecting a model in `/model` with NO active session must
//! re-trigger session creation (not silently set the default and stop).
//!
//! Feature: spec/features/no-session-model-selection-creates-session.feature
//!
//! `handle_model_selected` in the `session_id == None` branch previously
//! `return`ed after `set_default_model` succeeded, never re-attempting
//! `create_session`. The PROV-101 deadlock guard refuses a silent anthropic
//! fallback, so the model choice that was meant to UNBLOCK session creation
//! must trigger a retry once the default is committed:
//!   * real id  -> Action::SessionCreated (usable active session)
//!   * empty id -> Action::SessionCreationDeclined (error dialog, no empty seed)
//!
//! The session-present path stays unchanged (set_session_model only).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

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
// Scenario: Selecting a model with no active session retries session creation
// =============================================================================
#[tokio::test]
async fn no_session_model_selection_retries_create_session() {
    // @step Given no session is active and no default model is set
    let (mut app, mock) = fresh_app();
    assert!(
        app.agent_view_store().current_session().is_none(),
        "precondition: no active session"
    );
    // The retried create_session returns a real id once the default is set.
    mock.script_create_session(SessionId::new("s-after-default"));
    let prior_create = mock.create_session_calls();

    // @step And the model selector is open with session_id None
    // @step When I select the model "anthropic/claude-opus-4-8" with Enter
    app.dispatch(Action::ModelSelected(
        None,
        "anthropic".to_string(),
        "claude-opus-4-8".to_string(),
    ));
    drain_pending(&mut app).await;

    // @step Then the backend set_default_model is called with "anthropic/claude-opus-4-8"
    assert_eq!(
        mock.last_set_default_model(),
        Some("anthropic/claude-opus-4-8".to_string()),
        "set_default_model must receive the provider/model string"
    );

    // @step And after the default is committed create_session is retried
    assert_eq!(
        mock.create_session_calls(),
        prior_create + 1,
        "create_session must be retried exactly once after the default is set"
    );

    // @step And an Action::SessionCreated is dispatched
    // @step And a usable active session exists
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&SessionId::new("s-after-default")),
        "a usable active session must exist after the retry"
    );
}

// =============================================================================
// Scenario: Retried session creation is declined with an empty id
// =============================================================================
#[tokio::test]
async fn no_session_model_selection_declined_when_empty_id() {
    // @step Given no session is active and no default model is set
    let (mut app, mock) = fresh_app();
    assert!(app.agent_view_store().current_session().is_none());

    // @step And the next create_session returns an empty session id
    mock.script_create_session(SessionId::new(""));
    let prior_create = mock.create_session_calls();

    // @step And the model selector is open with session_id None
    // @step When I select the model "anthropic/claude-opus-4-8" with Enter
    app.dispatch(Action::ModelSelected(
        None,
        "anthropic".to_string(),
        "claude-opus-4-8".to_string(),
    ));
    drain_pending(&mut app).await;

    // @step Then the backend set_default_model is called with "anthropic/claude-opus-4-8"
    assert_eq!(
        mock.last_set_default_model(),
        Some("anthropic/claude-opus-4-8".to_string())
    );

    // @step And create_session is retried after the default is committed
    assert_eq!(
        mock.create_session_calls(),
        prior_create + 1,
        "create_session must be retried even when it will be declined"
    );

    // @step And an Action::SessionCreationDeclined is dispatched
    assert!(
        app.compositor().contains(ERROR_DIALOG_ID),
        "an empty id must surface the SessionCreationDeclined error dialog"
    );

    // @step And no empty active session is seeded
    assert!(
        app.agent_view_store().current_session().is_none(),
        "an empty SessionId must never be seeded as the active session"
    );
}

// =============================================================================
// Scenario: Selecting a model with an active session does not retry creation
// =============================================================================
#[tokio::test]
async fn active_session_model_selection_does_not_retry_create() {
    // @step Given an active session exists
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    let prior_session = mock.set_session_model_calls();
    let prior_default = mock.set_default_model_calls();
    let prior_create = mock.create_session_calls();

    // @step And the model selector is open with that session id
    // @step When I select a different model with Enter
    app.dispatch(Action::ModelSelected(
        Some(SessionId::new("s-1")),
        "openai".to_string(),
        "gpt-5.1-codex".to_string(),
    ));
    drain_pending(&mut app).await;

    // @step Then the live session model is updated via set_session_model
    assert_eq!(
        mock.set_session_model_calls(),
        prior_session + 1,
        "the active-session branch must call set_session_model"
    );

    // @step And the default-model path is not taken
    assert_eq!(
        mock.set_default_model_calls(),
        prior_default,
        "the active-session branch must NOT call set_default_model"
    );

    // @step And create_session is not retried
    assert_eq!(
        mock.create_session_calls(),
        prior_create,
        "the active-session branch must NOT retry create_session"
    );
}
