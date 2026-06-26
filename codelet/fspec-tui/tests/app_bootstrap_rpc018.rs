//! RPC-018 — App bootstrap + dispatch wiring tests for AgentView chrome state.
//!
//! Feature: spec/features/rpc018-app-bootstrap.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{
    ContextFillInfo, ModelInfo, SessionId, StreamChunk, ThinkingLevel, TokenTracker, WorkspaceInfo,
};

use common::MockBackend;

fn make_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

/// Scenario: Bootstrap fetches workspace info and stores it in AgentViewStore
#[tokio::test]
async fn bootstrap_fetches_workspace_info_and_stores_it_in_agent_view_store() {
    // @step Given an App constructed against a SharedFspecService bound to a temp git repo on branch "main"
    let (mut app, mock) = make_app();
    mock.set_workspace_info(WorkspaceInfo {
        cwd: "/tmp/x".to_string(),
        git_branch: Some("main".to_string()),
    });
    // @step When App::bootstrap is invoked
    app.bootstrap().await.expect("bootstrap");
    // @step Then App::dispatch has matched an Action::WorkspaceInfoLoaded(info)
    // @step And app.agent_view_store().workspace() returns Some(info) with cwd = <tmp_path> and git_branch = Some("main")
    let ws = app
        .agent_view_store()
        .workspace()
        .expect("workspace populated");
    assert_eq!(ws.cwd, "/tmp/x");
    assert_eq!(ws.git_branch.as_deref(), Some("main"));
}

/// Scenario: Action::WorkspaceInfoLoaded updates the AgentViewStore.workspace slot
#[test]
fn action_workspace_info_loaded_updates_workspace_slot() {
    // @step Given an App with agent_view_store.workspace() returning None
    let (mut app, _mock) = make_app();
    assert!(app.agent_view_store().workspace().is_none());
    // @step When App::dispatch receives Action::WorkspaceInfoLoaded(WorkspaceInfo { cwd: "/x", git_branch: Some("dev") })
    app.dispatch(Action::WorkspaceInfoLoaded(WorkspaceInfo {
        cwd: "/x".to_string(),
        git_branch: Some("dev".to_string()),
    }));
    // @step Then app.agent_view_store().workspace() returns Some(WorkspaceInfo { cwd: "/x", git_branch: Some("dev") })
    let ws = app.agent_view_store().workspace().expect("workspace");
    assert_eq!(ws.cwd, "/x");
    assert_eq!(ws.git_branch.as_deref(), Some("dev"));
}

/// Scenario: Bootstrap is best-effort — get_workspace_info failures do not abort
#[tokio::test]
async fn bootstrap_is_best_effort_workspace_info_failure_does_not_abort() {
    // @step Given an App whose backend.get_workspace_info() returns an error
    let (mut app, mock) = make_app();
    mock.set_workspace_info_error("simulated workspace fetch failure".to_string());
    // @step When App::bootstrap is invoked
    let result = app.bootstrap().await;
    // @step Then App::bootstrap returns Ok(()) (failure is non-fatal)
    assert!(
        result.is_ok(),
        "bootstrap should be non-fatal on workspace fetch error"
    );
    // @step And app.agent_view_store().workspace() returns None
    assert!(app.agent_view_store().workspace().is_none());
}

/// Scenario: Action::SessionCreated spawns get_model_info + get_thinking_level fetches
#[tokio::test]
async fn action_session_created_spawns_model_info_and_thinking_level_fetches() {
    // @step Given an App with no current_session yet
    let (mut app, mock) = make_app();
    mock.set_model_info(ModelInfo {
        display_name: "demo".to_string(),
        supports_reasoning: true,
        supports_vision: false,
        context_window: 100_000,
        compaction_threshold: 0,
    });
    mock.set_thinking_level(ThinkingLevel::Medium);
    assert!(app.agent_view_store().current_session().is_none());
    // @step When App::dispatch receives Action::SessionCreated(SessionId::new("s-1"))
    let sid = SessionId::new("s-1");
    app.dispatch(Action::SessionCreated(sid.clone()));
    // @step Then App::dispatch sets agent_view_store.current_session = Some("s-1")
    assert_eq!(app.agent_view_store().current_session(), Some(&sid));
    // @step And two new tasks are pending — one for backend.get_model_info("s-1") and one for backend.get_thinking_level("s-1")
    // Drain the spawned tasks deterministically.
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    // @step When both tasks complete and emit Action::ModelInfoLoaded("s-1", info) and Action::ThinkingLevelLoaded("s-1", level)
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then agent_view_store.model_info_for(SessionId("s-1")) returns Some(info)
    assert!(
        app.agent_view_store().model_info_for(&sid).is_some(),
        "model_info should be populated"
    );
    // @step And app.agent_view_store().thinking_level_for(SessionId("s-1")) returns Some(level)
    assert_eq!(
        app.agent_view_store().thinking_level_for(&sid).copied(),
        Some(ThinkingLevel::Medium)
    );
}

/// Scenario: Action::ModelInfoLoaded for a session NOT current_session still updates the by-session map
#[test]
fn action_model_info_loaded_for_non_current_session_still_updates_map() {
    // @step Given an App with current_session = Some(SessionId::new("s-1"))
    let (mut app, _mock) = make_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    // @step When App::dispatch receives Action::ModelInfoLoaded(SessionId::new("s-2"), ModelInfo { display_name: "Other", supports_reasoning: false, supports_vision: false, context_window: 100000 })
    app.dispatch(Action::ModelInfoLoaded(
        SessionId::new("s-2"),
        ModelInfo {
            display_name: "Other".to_string(),
            supports_reasoning: false,
            supports_vision: false,
            context_window: 100_000,
            compaction_threshold: 0,
        },
    ));
    // @step Then agent_view_store.model_info_for(SessionId("s-2")) returns Some(info)
    let info_s2 = app
        .agent_view_store()
        .model_info_for(&SessionId::new("s-2"))
        .cloned();
    assert!(info_s2.is_some(), "model_info for s-2 should be populated");
    // @step And agent_view_store.model_info_for(SessionId("s-1")) returns None
    let info_s1 = app
        .agent_view_store()
        .model_info_for(&SessionId::new("s-1"))
        .cloned();
    assert!(info_s1.is_none(), "model_info for s-1 should still be None");
}

/// Scenario: Action::ChunkReceived TokenUpdate updates token_state for the current session only
#[test]
fn action_chunk_received_token_update_updates_token_state() {
    // @step Given an App with current_session = Some(SessionId::new("s-1"))
    let (mut app, _mock) = make_app();
    let sid = SessionId::new("s-1");
    app.dispatch(Action::SessionCreated(sid.clone()));
    // @step And token_state_by_session["s-1"] starts at TokenState::default()
    // @step When App::dispatch receives Action::ChunkReceived("s-1", StreamChunk::TokenUpdate { tokens: TokenTracker with input_tokens = 1234 and output_tokens = 567 })
    let chunk = StreamChunk::TokenUpdate {
        tokens: TokenTracker {
            input_tokens: 1234,
            output_tokens: 567,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            tokens_per_second: None,
            cumulative_billed_input: None,
            cumulative_billed_output: None,
            reasoning_tokens: None,
        },
    };
    app.dispatch(Action::ChunkReceived(sid.clone(), chunk));
    // @step Then agent_view_store.token_state_for(SessionId("s-1")) returns Some(TokenState with input_tokens = 1234 and output_tokens = 567)
    let ts = app
        .agent_view_store()
        .token_state_for(&sid)
        .copied()
        .expect("token state");
    assert_eq!(ts.input_tokens, 1234);
    assert_eq!(ts.output_tokens, 567);
}

/// Scenario: Action::ChunkReceived ContextFillUpdate updates only context_fill_pct
#[test]
fn action_chunk_received_context_fill_update_updates_only_fill_pct() {
    // @step Given an App with current_session = Some(SessionId::new("s-1"))
    let (mut app, _mock) = make_app();
    let sid = SessionId::new("s-1");
    app.dispatch(Action::SessionCreated(sid.clone()));
    // @step And token_state_by_session["s-1"] is TokenState { input_tokens: 100, output_tokens: 50, context_fill_pct: 0 }
    app.agent_view_store_mut().set_token_state(
        sid.clone(),
        codelet_fspec_tui::store::TokenState {
            input_tokens: 100,
            output_tokens: 50,
            context_fill_pct: 0,
            ..Default::default()
        },
    );
    // @step When App::dispatch receives Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate { context_fill: ContextFillInfo with fill_percentage = 45 })
    let chunk = StreamChunk::ContextFillUpdate {
        context_fill: ContextFillInfo {
            fill_percentage: 45,
            effective_tokens: 0.0,
            threshold: 0.0,
            context_window: 0.0,
        },
    };
    app.dispatch(Action::ChunkReceived(sid.clone(), chunk));
    // @step Then agent_view_store.token_state_for(SessionId("s-1")) has context_fill_pct = 45
    let ts = app
        .agent_view_store()
        .token_state_for(&sid)
        .copied()
        .expect("token state");
    assert_eq!(ts.context_fill_pct, 45);
    // @step And input_tokens still equals 100
    assert_eq!(ts.input_tokens, 100);
    // @step And output_tokens still equals 50
    assert_eq!(ts.output_tokens, 50);
}
