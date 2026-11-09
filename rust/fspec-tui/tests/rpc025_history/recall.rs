//! RPC-025: Shift+Up/Down history recall through App::dispatch.

use super::*;
use codelet_test_helpers::{OrFail, TestResult};

/// Scenario: First Shift+↑ caches the live draft, loads a fresh snapshot, and replaces input with history[0]
#[tokio::test(flavor = "current_thread")]
async fn first_shift_up_caches_draft_loads_snapshot_replaces_input() -> TestResult {
    // @step Given an AgentViewStore with open_sessions[0].id == SessionId("s-1") and current_session_index == 0
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // @step And history_state_by_session is empty
    // @step And cached_history_snapshot is empty
    // @step And the backend's persistence_get_history(SessionId("s-1"), 100) is stubbed to return ["third", "second", "first"]
    mock.script_history(
        sid("s-1"),
        vec![
            "third".to_string(),
            "second".to_string(),
            "first".to_string(),
        ],
    )?;
    // @step And the MultiLineInput buffer is "live draft"
    app.navigator_mut().agent.input.set_value("live draft");
    // @step When App::dispatch handles Action::HistoryPrev
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index equals Some(0)
    let state = app
        .agent_view_store()
        .history_state_for(&sid("s-1"))
        .or_fail("history state must exist after first HistoryPrev")?;
    assert_eq!(state.recall_index, Some(0));
    // @step And history_state_by_session[SessionId("s-1")].cached_draft equals Some("live draft")
    assert_eq!(state.cached_draft.as_deref(), Some("live draft"));
    // @step And cached_history_snapshot[SessionId("s-1")] equals ["third", "second", "first"]
    let snap = app
        .agent_view_store()
        .cached_history_snapshot(&sid("s-1"))
        .or_fail("snapshot must be cached after first HistoryPrev")?;
    assert_eq!(
        snap,
        &vec![
            "third".to_string(),
            "second".to_string(),
            "first".to_string()
        ]
    );
    // @step And the MultiLineInput buffer equals "third"
    assert_eq!(app.navigator().agent.input.value(), "third");
    Ok(())
}

/// Scenario: Subsequent Shift+↑ walks backwards through the cached snapshot and clamps at len-1
#[tokio::test(flavor = "current_thread")]
async fn subsequent_shift_up_walks_back_and_clamps() -> TestResult {
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    mock.script_history(
        sid("s-1"),
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
    )?;
    app.navigator_mut().agent.input.set_value("live");
    // @step Given an AgentViewStore where history_state_by_session[SessionId("s-1")] is HistoryNavState { recall_index: Some(0), cached_draft: Some("live") }
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    let calls_after_first = mock.get_history_call_count()?;
    assert_eq!(calls_after_first, 1);
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        Some(0)
    );
    // @step And cached_history_snapshot[SessionId("s-1")] equals ["a", "b", "c"]
    // @step When App::dispatch handles Action::HistoryPrev
    app.dispatch(Action::HistoryPrev);
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index equals Some(1)
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        Some(1)
    );
    // @step And the MultiLineInput buffer equals "b"
    assert_eq!(app.navigator().agent.input.value(), "b");
    // @step When App::dispatch handles Action::HistoryPrev
    app.dispatch(Action::HistoryPrev);
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index equals Some(2)
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        Some(2)
    );
    // @step And the MultiLineInput buffer equals "c"
    assert_eq!(app.navigator().agent.input.value(), "c");
    // @step When App::dispatch handles Action::HistoryPrev
    app.dispatch(Action::HistoryPrev);
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index stays at Some(2)
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        Some(2)
    );
    // @step And the MultiLineInput buffer is still "c"
    assert_eq!(app.navigator().agent.input.value(), "c");
    // @step And the backend's persistence_get_history was called exactly 0 additional times (snapshot is cached)
    assert_eq!(mock.get_history_call_count()?, calls_after_first);
    Ok(())
}

/// Scenario: Shift+↓ walks forwards and exits recall on the final step restoring the cached_draft
#[tokio::test(flavor = "current_thread")]
async fn shift_down_walks_forward_and_exits_recall() -> TestResult {
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    mock.script_history(
        sid("s-1"),
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
    )?;
    app.navigator_mut().agent.input.set_value("live");
    // @step Given an AgentViewStore where history_state_by_session[SessionId("s-1")] is HistoryNavState { recall_index: Some(2), cached_draft: Some("live") }
    // @step And cached_history_snapshot[SessionId("s-1")] equals ["a", "b", "c"]
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    app.dispatch(Action::HistoryPrev);
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    app.dispatch(Action::HistoryPrev);
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        Some(2)
    );
    // @step When App::dispatch handles Action::HistoryNext
    app.dispatch(Action::HistoryNext);
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index equals Some(1)
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        Some(1)
    );
    // @step And the MultiLineInput buffer equals "b"
    assert_eq!(app.navigator().agent.input.value(), "b");
    // @step When App::dispatch handles Action::HistoryNext
    app.dispatch(Action::HistoryNext);
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index equals Some(0)
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        Some(0)
    );
    // @step And the MultiLineInput buffer equals "a"
    assert_eq!(app.navigator().agent.input.value(), "a");
    // @step When App::dispatch handles Action::HistoryNext
    app.dispatch(Action::HistoryNext);
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index equals None
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        None
    );
    // @step And the MultiLineInput buffer equals "live"
    assert_eq!(app.navigator().agent.input.value(), "live");
    Ok(())
}

/// Scenario: Shift+↓ from live-draft mode is a no-op
#[tokio::test(flavor = "current_thread")]
async fn shift_down_from_live_draft_mode_is_a_noop() -> TestResult {
    // @step Given an AgentViewStore where history_state_by_session[SessionId("s-1")].recall_index is None
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // @step And the MultiLineInput buffer is "current draft"
    app.navigator_mut().agent.input.set_value("current draft");
    // @step When App::dispatch handles Action::HistoryNext
    app.dispatch(Action::HistoryNext);
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index stays None
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        None
    );
    // @step And the MultiLineInput buffer is unchanged at "current draft"
    assert_eq!(app.navigator().agent.input.value(), "current draft");
    Ok(())
}

/// Scenario: Shift+↑ when history is empty is a no-op
#[tokio::test(flavor = "current_thread")]
async fn shift_up_with_empty_history_is_a_noop() -> TestResult {
    // @step Given an AgentViewStore with open_sessions[0].id == SessionId("s-1") and current_session_index == 0
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // @step And the backend's persistence_get_history(SessionId("s-1"), 100) is stubbed to return an empty Vec
    mock.script_history(sid("s-1"), Vec::new())?;
    // @step And the MultiLineInput buffer is "untouched"
    app.navigator_mut().agent.input.set_value("untouched");
    // @step When App::dispatch handles Action::HistoryPrev
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index stays at None
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        None
    );
    // @step And cached_history_snapshot[SessionId("s-1")] is empty
    let snap = app.agent_view_store().cached_history_snapshot(&sid("s-1"));
    assert!(snap.map(std::vec::Vec::is_empty).unwrap_or(true));
    // @step And the MultiLineInput buffer is unchanged at "untouched"
    assert_eq!(app.navigator().agent.input.value(), "untouched");
    Ok(())
}
