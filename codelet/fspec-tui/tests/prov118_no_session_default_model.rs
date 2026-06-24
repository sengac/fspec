//! PROV-118 — App::dispatch routing for `Action::ModelSelected` with NO
//! active session.
//!
//! Feature: spec/features/no-session-model-selection-sets-default-model.feature
//!
//! `handle_model_selected` previously returned early when `session_id` was
//! `None`, persisting nothing — the chicken-and-egg deadlock. It must now call
//! `backend.set_default_model("<provider>/<model>")` so the next
//! `create_session` succeeds. The session-present path stays unchanged (it
//! still calls `set_session_model` and never touches the default-model path).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
// Scenario: Selecting a model with no active session sets the default model
// =============================================================================
#[tokio::test]
async fn no_session_model_selection_calls_set_default_model() {
    // @step Given no session exists and no default model is set
    let (mut app, mock) = fresh_app();
    assert!(
        app.agent_view_store().current_session().is_none(),
        "precondition: no active session"
    );
    let prior_default = mock.set_default_model_calls();
    let prior_session = mock.set_session_model_calls();

    // @step And the model selector is open with session_id None
    // @step When I select the model "anthropic/claude-opus-4-8" with Enter
    app.dispatch(Action::ModelSelected(
        None,
        "anthropic".to_string(),
        "claude-opus-4-8".to_string(),
    ));

    // @step Then handle_model_selected does not return early
    // @step And the backend set_default_model is called with "anthropic/claude-opus-4-8"
    drain_pending(&mut app).await;
    assert_eq!(
        mock.set_default_model_calls(),
        prior_default + 1,
        "no-session selection must call set_default_model exactly once"
    );
    assert_eq!(
        mock.last_set_default_model(),
        Some("anthropic/claude-opus-4-8".to_string()),
        "set_default_model must receive the provider/model string"
    );
    assert_eq!(
        mock.set_session_model_calls(),
        prior_session,
        "the no-session branch must NOT call set_session_model"
    );
}

// =============================================================================
// Scenario: Selecting a model with an active session updates the live session
//           unchanged (no regression — default-model path not taken)
// =============================================================================
#[tokio::test]
async fn active_session_model_selection_updates_live_session_only() {
    // @step Given an active session exists
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    let prior_session = mock.set_session_model_calls();
    let prior_default = mock.set_default_model_calls();

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
    let last = mock
        .last_set_session_model()
        .expect("last set_session_model");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, "openai");
    assert_eq!(last.2, "gpt-5.1-codex");

    // @step And the default-model path is not taken
    assert_eq!(
        mock.set_default_model_calls(),
        prior_default,
        "the active-session branch must NOT call set_default_model"
    );
}
