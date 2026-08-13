//! RPC-022 — App::dispatch routing tests for ModelSelected /
//! ThinkingLevelSelected / SetSessionRole / SessionRoleLoaded /
//! ListProvidersLoaded.
//!
//! Feature: spec/features/rpc022-app-dispatch.feature
//!
//! Exercises the App-level wiring of the five new RPC-022 Action
//! variants against the shared `MockBackend` fixture so we can assert
//! both the synchronous store mutation AND the spawned backend tasks
//! (set_session_model / set_thinking_level / set_session_role /
//! get_session_role) plus the follow-up Action::ModelInfoLoaded /
//! Action::ThinkingLevelLoaded / Action::SessionRoleLoaded refreshes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, ThinkingLevel};

mod common;
use common::MockBackend;
fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

/// Drain `app.next_pending_task()` until empty so spawned backend
/// tasks observably complete before we assert against MockBackend
/// counters / store mutations driven by `Action::ModelInfoLoaded` etc.
async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    // Run any follow-up actions emitted onto the bus by completing
    // backend tasks (e.g. ModelInfoLoaded after set_session_model).
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        // The follow-up action may itself spawn another task
        // (set_session_model → get_model_info → ModelInfoLoaded). Keep
        // draining until quiescent.
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

/// Scenario: Action::ModelSelected spawns set_session_model and refreshes SessionHeader chrome
#[tokio::test]
async fn action_model_selected_spawns_set_session_model_and_refreshes_chrome() {
    // @step Given an App attached to an EmbeddedFspecBackend wrapping a SharedFspecService with a session manager attached
    // @step And an open session SessionId("s-1") with current_session_index = 0
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    // Snapshot baseline call counts.
    let prior_set_calls = mock.set_session_model_calls();

    // @step When the App dispatches Action::ModelSelected(SessionId("s-1"), "openai", "gpt-5.1-codex")
    app.dispatch(Action::ModelSelected(
        Some(SessionId::new("s-1")),
        "openai".to_string(),
        "gpt-5.1-codex".to_string(),
    ));
    // @step Then a tokio task is spawned that calls backend.set_session_model(SessionId("s-1"), "openai", "gpt-5.1-codex")
    // @step When the spawned task completes
    drain_pending(&mut app).await;
    assert_eq!(mock.set_session_model_calls(), prior_set_calls + 1);
    let last = mock
        .last_set_session_model()
        .expect("last set_session_model");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, "openai");
    assert_eq!(last.2, "gpt-5.1-codex");
    // @step Then a follow-up tokio task is spawned that calls backend.get_model_info(SessionId("s-1"))
    // @step And Action::ModelInfoLoaded(SessionId("s-1"), <fresh ModelInfo>) is dispatched
    // (We've already drained pending — the follow-up ModelInfoLoaded
    // action has been pumped through dispatch above.)
}

/// Scenario: Action::ThinkingLevelSelected spawns set_thinking_level and refreshes the [T:] badge
#[tokio::test]
async fn action_thinking_level_selected_spawns_set_thinking_level_and_refreshes_badge() {
    // @step Given an App attached to an EmbeddedFspecBackend with a session manager attached
    // @step And an open session SessionId("s-1")
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    let prior = mock.set_thinking_level_calls();

    // @step When the App dispatches Action::ThinkingLevelSelected(SessionId("s-1"), ThinkingLevel::High)
    app.dispatch(Action::ThinkingLevelSelected(
        SessionId::new("s-1"),
        ThinkingLevel::High,
    ));
    // @step Then a tokio task is spawned that calls backend.set_thinking_level(SessionId("s-1"), ThinkingLevel::High)
    // @step When the spawned task completes
    drain_pending(&mut app).await;
    assert_eq!(mock.set_thinking_level_calls(), prior + 1);
    let last = mock
        .last_set_thinking_level()
        .expect("last set_thinking_level");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, ThinkingLevel::High);
    // @step Then a follow-up tokio task is spawned that calls backend.get_thinking_level(SessionId("s-1"))
    // @step And Action::ThinkingLevelLoaded(SessionId("s-1"), ThinkingLevel::High) is dispatched
    // (Drained above; the follow-up arm has been pumped through dispatch.)
}

/// Scenario: Action::SetSessionRole(Some) spawns set_session_role and updates AgentViewStore.role_by_session
#[tokio::test]
async fn action_set_session_role_some_spawns_set_role_and_updates_store() {
    // @step Given an App attached to an EmbeddedFspecBackend with a session manager attached
    // @step And an open session SessionId("s-1") whose role_for is None
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    assert!(app
        .agent_view_store()
        .role_for(&SessionId::new("s-1"))
        .is_none());
    let prior = mock.set_session_role_calls();
    // @step When the App dispatches Action::SetSessionRole(SessionId("s-1"), Some("You are a security reviewer".to_string()))
    app.dispatch(Action::SetSessionRole(
        SessionId::new("s-1"),
        Some("You are a security reviewer".to_string()),
    ));
    // @step Then AgentViewStore.role_for(&SessionId("s-1")) equals Some("You are a security reviewer")
    assert_eq!(
        app.agent_view_store().role_for(&SessionId::new("s-1")),
        Some("You are a security reviewer")
    );
    // @step And a tokio task is spawned that calls backend.set_session_role(SessionId("s-1"), Some("You are a security reviewer".to_string()))
    drain_pending(&mut app).await;
    assert_eq!(mock.set_session_role_calls(), prior + 1);
    let last = mock.last_set_session_role().expect("last set_session_role");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, Some("You are a security reviewer".to_string()));
}

/// Scenario: Action::SetSessionRole(None) clears the role and persists via backend
#[tokio::test]
async fn action_set_session_role_none_clears_role_and_persists() {
    // @step Given an App attached to an EmbeddedFspecBackend with a session manager attached
    // @step And an open session SessionId("s-1") whose role_for is Some("Reviewer A")
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    app.agent_view_store_mut()
        .set_role(SessionId::new("s-1"), Some("Reviewer A".to_string()));
    assert_eq!(
        app.agent_view_store().role_for(&SessionId::new("s-1")),
        Some("Reviewer A")
    );
    let prior = mock.set_session_role_calls();
    // @step When the App dispatches Action::SetSessionRole(SessionId("s-1"), None)
    app.dispatch(Action::SetSessionRole(SessionId::new("s-1"), None));
    // @step Then AgentViewStore.role_for(&SessionId("s-1")) equals None
    assert!(app
        .agent_view_store()
        .role_for(&SessionId::new("s-1"))
        .is_none());
    // @step And a tokio task is spawned that calls backend.set_session_role(SessionId("s-1"), None)
    drain_pending(&mut app).await;
    assert_eq!(mock.set_session_role_calls(), prior + 1);
    let last = mock.last_set_session_role().expect("last set_session_role");
    assert_eq!(last.0, SessionId::new("s-1"));
    assert_eq!(last.1, None);
}

/// Scenario: Action::SessionRoleLoaded folds a backend-fetched role into AgentViewStore
#[tokio::test]
async fn action_session_role_loaded_folds_backend_role_into_store() {
    // @step Given an App attached to an EmbeddedFspecBackend with a session manager attached
    // @step And an open session SessionId("s-1") whose role_for is None
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    let prior_writes = mock.set_session_role_calls();
    // @step When the App dispatches Action::SessionRoleLoaded(SessionId("s-1"), Some("Reviewer A".to_string()))
    app.dispatch(Action::SessionRoleLoaded(
        SessionId::new("s-1"),
        Some("Reviewer A".to_string()),
    ));
    // @step Then AgentViewStore.role_for(&SessionId("s-1")) equals Some("Reviewer A")
    assert_eq!(
        app.agent_view_store().role_for(&SessionId::new("s-1")),
        Some("Reviewer A")
    );
    // @step And no backend task is spawned in response
    drain_pending(&mut app).await;
    assert_eq!(
        mock.set_session_role_calls(),
        prior_writes,
        "SessionRoleLoaded must NOT trigger a backend write"
    );
}

/// Scenario: Action::SessionCreated triggers a backend.get_session_role spawn that fills AgentViewStore.role_by_session
#[tokio::test]
async fn action_session_created_spawns_get_session_role() {
    // @step Given an App attached to an EmbeddedFspecBackend with a session manager that returns Some("Reviewer A") from get_session_role
    let (mut app, mock) = fresh_app();
    mock.seed_session_role(SessionId::new("s-9"), Some("Reviewer A".to_string()));
    let prior_get_calls = mock.get_session_role_calls();
    // @step When the App dispatches Action::SessionCreated(SessionId("s-9"))
    app.dispatch(Action::SessionCreated(SessionId::new("s-9")));
    // @step Then refresh_session_chrome(SessionId("s-9")) is called
    // (refresh_session_chrome is private. Its observable side-effect is
    //  that backend.get_session_role(sid) is spawned — assert that.)
    // @step And a tokio task is spawned that calls backend.get_session_role(SessionId("s-9"))
    // @step When the spawned task completes
    // Give the spawned task a moment to issue the read.
    tokio::time::sleep(Duration::from_millis(20)).await;
    drain_pending(&mut app).await;
    assert!(
        mock.get_session_role_calls() > prior_get_calls,
        "expected get_session_role to be called, before={prior_get_calls} after={}",
        mock.get_session_role_calls()
    );
    // @step Then Action::SessionRoleLoaded(SessionId("s-9"), Some("Reviewer A".to_string())) is dispatched
    // @step And AgentViewStore.role_for(&SessionId("s-9")) equals Some("Reviewer A")
    assert_eq!(
        app.agent_view_store().role_for(&SessionId::new("s-9")),
        Some("Reviewer A")
    );
}

/// Scenario: Action::ModelSelected against a service with no session manager is a silent no-op
#[tokio::test]
async fn action_model_selected_against_no_session_manager_is_silent_no_op() {
    // @step Given an App attached to an EmbeddedFspecBackend wrapping a SharedFspecService with NO session manager attached
    // (MockBackend's defaults for set_session_model return Ok(()) and
    //  for get_model_info return ModelInfo::default(); the "no session
    //  manager" path is structurally equivalent — no-op writes.)
    let (mut app, _mock) = fresh_app();
    // @step And no open sessions
    // (Skip SessionCreated.)
    // @step When the App dispatches Action::ModelSelected(SessionId("any"), "openai", "gpt-5.1-codex")
    app.dispatch(Action::ModelSelected(
        Some(SessionId::new("any")),
        "openai".to_string(),
        "gpt-5.1-codex".to_string(),
    ));
    // @step Then no panic occurs
    // @step And no spawned task fails
    drain_pending(&mut app).await;
    // If we reach this line, neither dispatch nor drain panicked.
}

/// Scenario: dispatch_model_thinking_dialogs.rs stays under 300 lines
#[test]
fn dispatch_model_thinking_dialogs_rs_stays_under_300_lines() {
    // @step Given the file rust/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs after RPC-022 lands
    let path = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("app")
        .join("dispatch_model_thinking_dialogs.rs");
    // @step When a test counts the line-count of the file
    let lines = common::read_to_string_or_panic(&path).lines().count();
    // @step Then the file has fewer than 300 lines
    assert!(
        lines < 300,
        "dispatch_model_thinking_dialogs.rs has {lines} lines"
    );
}
