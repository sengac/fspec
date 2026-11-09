//! RPC-437 — TUI consumes the SessionEvent::Cleared lifecycle event.
//!
//! Feature: spec/features/session-events-tui-cleared-consumption.feature
//!
//! The five TUI-facing scenarios, driven through the MockBackend
//! (embedded-transport equivalent). The WS live-wire counterpart is covered
//! at the wire level by rpc-server/tests/rpc436_session_cleared_fan_out.rs;
//! the App-level behaviour is transport-agnostic. Helpers live in
//! tests/session_events_support/ and tests/rpc437_support/ (BUG-355: one
//! test file per feature).

use codelet_test_helpers::{OrFail, TestResult};

use codelet_fspec_tui::Action;

mod common;

mod session_events_support;
use session_events_support::{
    actions_after_subscribers_ran, app_with_mock, booted_app, sid, tabs_for, wait_for_action,
};

mod rpc437_support;
use rpc437_support::{
    input_draft_is_empty, is_status_cleared, seed_chunks, session_chunk_count,
    session_scrollback_text,
};

// =============================================================================
// Scenario: A session cleared from another client resets its local tab live
//           (no poll)
// =============================================================================

#[tokio::test]
async fn a_session_cleared_from_another_client_resets_its_local_tab_live_no_poll() -> TestResult {
    // @step Given a TUI connected to a daemon over WebSocket with one session S whose tab is focused and has scrollback content
    // Bootstrap has run, so the session_cleared_rx subscriber folds the
    // mock's cleared broadcast onto the (transport-agnostic) action bus.
    let (mock, mut app) = booted_app().await?;
    let s = sid("S-clear");
    app.dispatch(Action::SessionCreated(s.clone()));
    seed_chunks(&mut app, &s, 4)?;
    app.navigator_mut()
        .agent
        .input
        .set_value("typed before the remote clear");
    assert_eq!(app.agent_view_store().current_session(), Some(&s));
    assert_eq!(session_chunk_count(&app, &s)?, 4, "precondition: seeded");

    // @step When session S is cleared from another machine connected to the same daemon
    mock.push_session_cleared(s.clone());
    let action = wait_for_action(
        &mut app,
        |a| matches!(a, Action::SessionCleared(id) if id.value == "S-clear"),
    )
    .await
    .or_fail("the session-cleared event must reach the action bus")?;
    app.dispatch(action);

    // @step Then S's local scrollback and input draft are reset without any poll or user action
    assert_eq!(session_chunk_count(&app, &s)?, 0, "S's scrollback resets");
    assert!(input_draft_is_empty(&app, &s)?, "S's input draft blanks");
    assert!(
        app.navigator().agent.input.is_empty(),
        "the remote clear must blank the focused session's input buffer"
    );

    // @step And S's per-session SessionHeader chrome (status badge, token counters, compaction reduction) returns to the cleared state
    assert!(
        is_status_cleared(&app, &s),
        "RPC-100 chrome marks S Cleared"
    );
    assert!(
        app.agent_view_store()
            .compaction_reduction_for(&s)
            .is_none(),
        "the RPC-100 chrome fold must clear S's compaction reduction"
    );
    Ok(())
}

// =============================================================================
// Scenario: A clear event for a background session resets only that session
// =============================================================================

#[tokio::test]
async fn a_clear_event_for_a_background_session_resets_only_that_session() -> TestResult {
    // @step Given a TUI with two open sessions S-1 (background) and S-2 (focused), each with scrollback, and S-2's input buffer holding a typed draft
    let (_mock, mut app) = booted_app().await?;
    let s1 = sid("S-1");
    let s2 = sid("S-2");
    app.dispatch(Action::SessionCreated(s1.clone()));
    app.dispatch(Action::SessionCreated(s2.clone()));
    // S-2 was created second → it is focused; S-1 is the background tab.
    assert_eq!(app.agent_view_store().current_session(), Some(&s2));
    seed_chunks(&mut app, &s1, 3)?;
    seed_chunks(&mut app, &s2, 3)?;
    app.agent_view_store_mut()
        .session_context_mut_for(&s1)
        .or_fail("ctx s-1")?
        .input_draft
        .push_str("draft on background session");
    app.navigator_mut()
        .agent
        .input
        .set_value("draft on focused session");

    // @step When a SessionCleared event for S-1 is processed
    // Direct dispatch (the handler is transport independent); the live
    // bus path is pinned by the first scenario.
    app.dispatch(Action::SessionCleared(s1.clone()));

    // @step Then S-1's local scrollback and input draft are reset
    assert_eq!(
        session_chunk_count(&app, &s1)?,
        0,
        "S-1's scrollback resets"
    );
    assert!(input_draft_is_empty(&app, &s1)?, "S-1's input draft blanks");
    assert!(
        is_status_cleared(&app, &s1),
        "S-1's status folds to Cleared"
    );

    // @step And S-2's scrollback, input buffer and focus are untouched
    assert_eq!(
        session_chunk_count(&app, &s2)?,
        3,
        "S-2's scrollback survives"
    );
    assert!(
        !app.navigator().agent.input.is_empty(),
        "S-2's (focused) input buffer must survive"
    );
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&s2),
        "focus must stay on S-2"
    );

    // @step And S-1's per-session token and compaction badge state is reset
    assert!(
        app.agent_view_store()
            .compaction_reduction_for(&s1)
            .is_none(),
        "S-1's compaction reduction must be cleared (RPC-100 chrome)"
    );
    Ok(())
}

// =============================================================================
// Scenario: Clear events for a session without an open tab are a no-op
// =============================================================================

#[tokio::test]
async fn clear_events_for_a_session_without_an_open_tab_are_a_no_op() -> TestResult {
    // @step Given a TUI where session S was already closed locally (its tab already removed) or was never opened in this client
    let (_mock, mut app) = app_with_mock();
    let closed = sid("S-closed");
    let keep = sid("S-keep");
    app.dispatch(Action::SessionCreated(keep.clone()));
    // The local close path removes the tab BEFORE the daemon's event.
    app.agent_view_store_mut().remove_session_if_open(&closed);
    assert_eq!(tabs_for(&app, &closed), 0, "precondition: tab is closed");

    // @step When the TUI processes a SessionCleared event for S (fired twice to pin the double-fire no-op)
    app.dispatch(Action::SessionCleared(closed.clone()));
    app.dispatch(Action::SessionCleared(closed.clone()));

    // @step Then the TUI ignores the events as no-ops — no tab is created, no panic, and the surviving session's focus and state are untouched
    assert_eq!(tabs_for(&app, &closed), 0, "no tab from a stale clear");
    assert_eq!(tabs_for(&app, &keep), 1, "the surviving tab is untouched");
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&keep),
        "focus must be unaffected by a clear of a non-open session"
    );
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    Ok(())
}

// =============================================================================
// Scenario: Consecutive clear events for the same live session are
//           idempotent
// =============================================================================

#[tokio::test]
async fn consecutive_clear_events_for_the_same_live_session_are_idempotent() -> TestResult {
    // @step Given a TUI with one open session S whose tab was already reset by a first SessionCleared event
    let (_mock, mut app) = app_with_mock();
    let s = sid("S-repeat");
    app.dispatch(Action::SessionCreated(s.clone()));
    seed_chunks(&mut app, &s, 3)?;
    app.dispatch(Action::SessionCleared(s.clone()));
    assert_eq!(session_chunk_count(&app, &s)?, 0, "the first clear resets");

    // @step When a second SessionCleared event for S arrives (clear is repeatable — the session stays live)
    app.dispatch(Action::SessionCleared(s.clone()));

    // @step Then the TUI holds exactly one tab for S with no duplicate cleanup, no scrollback notice lines, and no state drift
    assert_eq!(tabs_for(&app, &s), 1, "exactly one tab survives");
    assert_eq!(
        session_chunk_count(&app, &s)?,
        0,
        "the second clear is a no-op"
    );
    let text = session_scrollback_text(&app, &s)?;
    assert!(
        text.lines().all(|l| !l.contains("[notice]")),
        "no scrollback notice line may be pushed by the clear fold (RPC-074 TS parity), got {text:?}"
    );
    assert!(is_status_cleared(&app, &s), "the status stays Cleared");
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    Ok(())
}

// =============================================================================
// Scenario: Bootstrap spawns the session-cleared subscriber and degenerate
//           defaults leak nothing
// =============================================================================

#[tokio::test]
async fn bootstrap_spawns_the_session_cleared_subscriber_with_degenerate_defaults() -> TestResult {
    // @step Given an App bootstrapped against a backend whose session-cleared receiver is the degenerate default (closed)
    let (mock, mut app) = app_with_mock();

    // @step When the App's bootstrap runs
    app.bootstrap().await.or_fail("bootstrap")?;

    // @step Then bootstrap spawns the full bootstrap subscriber set (BOOTSTRAP_SUBSCRIBER_TASKS), which includes the session_cleared_rx subscriber
    assert_eq!(
        app.subscriber_task_count(),
        common::BOOTSTRAP_SUBSCRIBER_TASKS,
        "bootstrap must spawn the session-cleared subscriber task"
    );

    // @step And the degenerate (closed) session-cleared receiver must not leak SessionCleared events onto the action bus
    let stale: Vec<Action> = actions_after_subscribers_ran(&mut app)
        .await
        .into_iter()
        .filter(|a| matches!(a, Action::SessionCleared(_)))
        .collect();
    assert!(
        stale.is_empty(),
        "a closed session-cleared receiver must never emit SessionCleared; got {stale:?}"
    );
    assert!(
        app.agent_view_store().open_sessions().is_empty(),
        "a degenerate default must seed no tabs"
    );
    drop(mock); // the backend stays alive for the subscriber's lifetime
    Ok(())
}
