//! RPC-025: per-session recall cycling and fire-and-forget submission persistence.

use super::*;
use codelet_test_helpers::{OrFail, TestResult};

/// Scenario: Each open session keeps an independent recall position across Shift+←/→ cycling
#[tokio::test(flavor = "current_thread")]
async fn each_session_keeps_independent_recall_across_cycling() -> TestResult {
    // @step Given an AgentViewStore with open_sessions [SessionId("s-1"), SessionId("s-2")] and current_session_index == 0
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    app.dispatch(Action::SessionPrev); // focus s-1
                                       // @step And the backend's persistence_get_history(SessionId("s-1"), 100) is stubbed to return ["a", "b"]
    mock.script_history(sid("s-1"), vec!["a".to_string(), "b".to_string()])?;
    // @step And the backend's persistence_get_history(SessionId("s-2"), 100) is stubbed to return ["x", "y", "z"]
    mock.script_history(
        sid("s-2"),
        vec!["x".to_string(), "y".to_string(), "z".to_string()],
    )?;
    // @step When App::dispatch handles Action::HistoryPrev
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
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
    // @step When App::dispatch handles Action::SessionNext
    app.dispatch(Action::SessionNext);
    // @step Then current_session_index equals 1
    assert_eq!(app.agent_view_store().current_session_index(), 1);
    // @step And the MultiLineInput buffer is restored from open_sessions[1].input_draft (NOT from history)
    let s2_draft = app.agent_view_store().open_sessions()[1]
        .input_draft
        .clone();
    assert_eq!(app.navigator().agent.input.value(), s2_draft);
    // @step When App::dispatch handles Action::HistoryPrev
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then history_state_by_session[SessionId("s-2")].recall_index equals Some(0)
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-2"))
            .and_then(|s| s.recall_index),
        Some(0)
    );
    // @step And the MultiLineInput buffer equals "x"
    assert_eq!(app.navigator().agent.input.value(), "x");
    // @step When App::dispatch handles Action::SessionPrev
    app.dispatch(Action::SessionPrev);
    // @step Then history_state_by_session[SessionId("s-1")].recall_index is preserved at Some(0)
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        Some(0)
    );
    // @step And history_state_by_session[SessionId("s-2")].recall_index is preserved at Some(0)
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-2"))
            .and_then(|s| s.recall_index),
        Some(0)
    );
    Ok(())
}

/// Scenario: Submitting an input fires persistence_add_history via tokio::spawn and resets the session's history state
#[tokio::test(flavor = "current_thread")]
async fn submitting_input_fires_add_history_and_resets_state() -> TestResult {
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    mock.script_history(
        sid("s-1"),
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
    )?;
    app.navigator_mut().agent.input.set_value("live");
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    app.dispatch(Action::HistoryPrev);
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Given an AgentViewStore with open_sessions[0].id == SessionId("s-1") and current_session_index == 0
    // @step And history_state_by_session[SessionId("s-1")] is HistoryNavState { recall_index: Some(1), cached_draft: Some("live") }
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        Some(1)
    );
    // @step And cached_history_snapshot[SessionId("s-1")] equals ["a", "b", "c"]
    assert_eq!(
        app.agent_view_store()
            .cached_history_snapshot(&sid("s-1"))
            .or_fail("snap")?,
        &vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
    // @step When App::dispatch handles Action::InputSubmitted("hello") for SessionId("s-1")
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, "hello".to_string()));
    }
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then backend.send_input(SessionId("s-1"), "hello") is invoked
    let sent = mock.send_input_calls.lock().or_fail("send_input")?.clone();
    assert!(sent.iter().any(|(s, t)| s == &sid("s-1") && t == "hello"));
    // BUG-331: an idle session's plain submission is not a mode delivery.
    assert!(mock
        .submitted_inputs
        .lock()
        .or_fail("submitted")?
        .is_empty());
    // @step And backend.persistence_add_history(SessionId("s-1"), "hello") is invoked via tokio::spawn
    let added = mock
        .persistence_add_history_calls
        .lock()
        .or_fail("add_history")?
        .clone();
    assert!(added.iter().any(|(s, t)| s == &sid("s-1") && t == "hello"));
    // @step And history_state_by_session[SessionId("s-1")].recall_index is reset to None
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.recall_index),
        None
    );
    // @step And history_state_by_session[SessionId("s-1")].cached_draft is reset to None
    assert_eq!(
        app.agent_view_store()
            .history_state_for(&sid("s-1"))
            .and_then(|s| s.cached_draft.clone()),
        None
    );
    // @step And cached_history_snapshot[SessionId("s-1")] is cleared
    let snap = app.agent_view_store().cached_history_snapshot(&sid("s-1"));
    assert!(snap.map(std::vec::Vec::is_empty).unwrap_or(true));
    // @step And the next Action::HistoryPrev triggers a fresh persistence_get_history round-trip
    let before = mock.get_history_call_count()?;
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    assert!(mock.get_history_call_count()? > before);
    Ok(())
}

/// Scenario: persistence_add_history is fire-and-forget — a slow disk write never blocks input submission
#[tokio::test(flavor = "current_thread")]
async fn persistence_add_history_is_fire_and_forget() -> TestResult {
    // @step Given a fake FspecBackend whose persistence_add_history blocks indefinitely on an internal latch
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    let latch = Arc::new(Notify::new());
    mock.install_add_history_latch(latch.clone())?;
    // @step When App::dispatch handles Action::InputSubmitted("hello") for SessionId("s-1")
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, "hello".to_string()));
    }
    // @step Then the dispatch call returns within 50 milliseconds without awaiting persistence_add_history
    // TEST-042: no wall-clock bound. The latch is never released, so a
    // dispatch that awaited persistence_add_history could not return at
    // all; reaching this line with the latch still closed is the proof.
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    // @step And backend.send_input(SessionId("s-1"), "hello") is invoked exactly once
    let sent = mock.send_input_calls.lock().or_fail("send_input")?.clone();
    assert_eq!(
        sent.iter()
            .filter(|(s, t)| s == &sid("s-1") && t == "hello")
            .count(),
        1
    );
    // @step And the spawned persistence_add_history task is still pending (the latch has not been released)
    let added = mock
        .persistence_add_history_calls
        .lock()
        .or_fail("add_history")?
        .clone();
    assert!(added.is_empty(), "add_history must still be latched");
    latch.notify_one();
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    Ok(())
}
