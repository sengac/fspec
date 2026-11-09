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
//!   * creation error (BUG-410) -> session failure dialog, nothing seeded
//!
//! The session-present path stays unchanged (set_session_model only).

use crate::common::TaskOutcome;
use codelet_test_helpers::TestResult;
use std::sync::Arc;

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
async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

// =============================================================================
// Scenario: Selecting a model with no active session retries session creation
// =============================================================================
#[tokio::test]
async fn no_session_model_selection_retries_create_session() -> TestResult {
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
    drain_pending(&mut app).await?;

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
    Ok(())
}

// =============================================================================
// Scenario: Retried session creation fails
// =============================================================================
#[tokio::test]
async fn no_session_model_selection_retry_failure_is_shown() -> TestResult {
    // @step Given no session is active and no default model is set
    let (mut app, mock) = fresh_app();
    assert!(app.agent_view_store().current_session().is_none());

    // @step And the next create_session fails because no default model is set
    mock.script_create_session_error("no default model is set");
    let prior_create = mock.create_session_calls();

    // @step And the model selector is open with session_id None
    // @step When I select the model "anthropic/claude-opus-4-8" with Enter
    app.dispatch(Action::ModelSelected(
        None,
        "anthropic".to_string(),
        "claude-opus-4-8".to_string(),
    ));
    drain_pending(&mut app).await?;

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

    // @step And the session failure dialog is shown
    assert!(
        app.compositor().contains("session-storage-dialog"),
        "a failed retry must open the session failure dialog"
    );

    // @step And no active session is seeded
    assert!(
        app.agent_view_store().current_session().is_none(),
        "a failed creation must never seed an active session"
    );
    Ok(())
}

// =============================================================================
// Scenario: Selecting a model with an active session does not retry creation
// =============================================================================
#[tokio::test]
async fn active_session_model_selection_does_not_retry_create() -> TestResult {
    // @step Given an active session exists
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await?;
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
    drain_pending(&mut app).await?;

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
    Ok(())
}
