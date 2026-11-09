//! RPC-434 — TUI consumes session lifecycle events (created/removed) +
//! connect-time snapshot replay.
//!
//! Feature: spec/features/session-events-tui-consumption.feature
//!
//! The four TUI-facing scenarios, driven through the MockBackend
//! (embedded-transport equivalent), plus the bootstrap subscriber
//! invariant. The WS live-wire counterpart is covered at the wire level by
//! rpc-server/tests/rpc433_session_event_fan_out.rs; the App-level
//! behaviour is transport-agnostic. Helpers live in
//! tests/session_events_support/ (BUG-355: one test file per feature).

use codelet_test_helpers::{OrFail, TestResult};

use codelet_fspec_tui::Action;
use codelet_rpc_types::WorkUnitContext;

mod common;

mod session_events_support;
use session_events_support::{
    actions_after_subscribers_ran, app_with_mock, booted_app, sid, tabs_for, wait_for_action,
};

// =============================================================================
// Scenario: Starting the TUI against a daemon with open sessions seeds a
//           tab for each of them
// =============================================================================

#[tokio::test]
async fn starting_the_tui_against_a_daemon_with_open_sessions_seeds_a_tab_for_each_of_them(
) -> TestResult {
    // @step Given a daemon whose session manager already holds two live sessions (created from another machine)
    // The connect-time open-session snapshot (RPC-433
    // `Envelope::SessionSnapshot`) is the mock's scripted seed.
    let (mock, mut app) = app_with_mock();
    mock.seed_session_snapshot(vec![sid("S-snap-A"), sid("S-snap-B")]);

    // @step When the TUI boots against that daemon (either transport)
    // `App::bootstrap` folds EVERY snapshot session through the
    // `Action::SessionCreated` handler before `spawn_session_restore`.
    app.bootstrap().await.or_fail("bootstrap")?;

    // @step Then the TUI's session list contains a tab for each of those two sessions, seeded from the open-session snapshot before any session-creation RPC was issued by this client
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        2,
        "both open-session snapshot entries must seed tabs"
    );
    assert_eq!(tabs_for(&app, &sid("S-snap-A")), 1);
    assert_eq!(tabs_for(&app, &sid("S-snap-B")), 1);
    assert_eq!(
        mock.create_session_calls(),
        0,
        "the snapshot must seed tabs WITHOUT issuing a create_session RPC"
    );
    // Rule R4: replay is idempotent (a reconnect re-runs the snapshot).
    app.dispatch(Action::SessionCreated(sid("S-snap-A")));
    app.dispatch(Action::SessionCreated(sid("S-snap-B")));
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        2,
        "replaying the snapshot over existing tabs must be a no-op"
    );
    Ok(())
}

// =============================================================================
// Scenario: A remote session creation adds a tab live (no poll)
// =============================================================================

#[tokio::test]
async fn a_remote_session_creation_adds_a_tab_live_no_poll() -> TestResult {
    // @step Given a TUI is connected to a daemon over WebSocket with no sessions open yet
    let (mock, mut app) = booted_app().await?;
    assert!(
        app.agent_view_store().open_sessions().is_empty(),
        "precondition: no open tabs at bootstrap"
    );

    // @step When a session is created from another machine connected to the same daemon
    // The session-created subscriber folds the broadcast onto the bus
    // (the same path the WS transport's session-event splitter feeds).
    let remote = sid("S-remote-1");
    mock.push_session_created(remote.clone());
    let action = wait_for_action(
        &mut app,
        |a| matches!(a, Action::SessionObserved(info) if info.id == "S-remote-1"),
    )
    .await
    .or_fail("the session-created event must reach the action bus")?;
    app.dispatch(action);

    // @step Then a new session tab appears in the TUI's session list without any poll or user action
    assert_eq!(
        tabs_for(&app, &remote),
        1,
        "a live remote creation appends a tab"
    );
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        1,
        "exactly one tab"
    );
    Ok(())
}

// =============================================================================
// Scenario: A remote session removal closes its tab with focus and board
//           cleanup
// =============================================================================

#[tokio::test]
async fn a_remote_session_removal_closes_its_tab_with_focus_and_board_cleanup() -> TestResult {
    // @step Given a TUI connected to a daemon shows two live session tabs, one of which is focused and has a work unit attached on the board
    let (mock, mut app) = booted_app().await?;
    let gone = sid("S-gone");
    let keep = sid("S-keep");
    app.dispatch(Action::SessionCreated(keep.clone()));
    app.dispatch(Action::SessionCreated(gone.clone()));
    app.agent_view_store_mut()
        .set_current_work_unit(Some("WU-A".to_string()), Some("implementing".to_string()));
    app.agent_view_store_mut().set_work_unit_context(
        gone.clone(),
        WorkUnitContext {
            id: "WU-A".to_string(),
            title: "WU-A".to_string(),
            status: "implementing".to_string(),
        },
    );
    app.board_store_mut().attach_session("WU-A", gone.clone());
    assert_eq!(app.agent_view_store().current_session(), Some(&gone));
    assert!(app.board_store().session_for("WU-A").is_some());

    // @step When that session is closed from another machine connected to the same daemon
    mock.push_session_removed(gone.clone());
    let action = wait_for_action(
        &mut app,
        |a| matches!(a, Action::SessionRemoved(s) if s.value == "S-gone"),
    )
    .await
    .or_fail("the session-removed event must reach the action bus")?;
    app.dispatch(action);

    // @step Then the TUI removes that session's tab, the focused session becomes the surviving one, and the board's work-unit attachment for the removed session is released
    assert_eq!(tabs_for(&app, &gone), 0, "the doomed session's tab closes");
    assert_eq!(tabs_for(&app, &keep), 1, "the surviving tab must remain");
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&keep),
        "focus must fall back to the surviving session"
    );
    assert!(
        app.board_store().session_for("WU-A").is_none(),
        "the board's work-unit attachment for the removed session must be released"
    );
    Ok(())
}

// =============================================================================
// Scenario: Removal events are idempotent for already-closed tabs
// =============================================================================

#[tokio::test]
async fn removal_events_are_idempotent_for_already_closed_tabs() -> TestResult {
    // @step Given a TUI connected to a daemon and a session that has already been closed locally (its tab already removed)
    let (_mock, mut app) = app_with_mock();
    let closed = sid("S-closed");
    let keep = sid("S-keep-1");
    app.dispatch(Action::SessionCreated(keep.clone()));
    // The local close path removes the tab BEFORE the daemon's event.
    app.agent_view_store_mut().remove_session_if_open(&closed);
    assert_eq!(tabs_for(&app, &closed), 0, "precondition: closed locally");

    // @step When the TUI processes a SessionRemoved event for that already-closed session
    // Fired TWICE to pin the double-fire no-op (local close + remote event).
    app.dispatch(Action::SessionRemoved(closed.clone()));
    app.dispatch(Action::SessionRemoved(closed));

    // @step Then the TUI ignores the event as a no-op (no panic, no duplicate cleanup, no effect on focus or the board)
    assert_eq!(tabs_for(&app, &keep), 1, "the surviving tab is untouched");
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&keep),
        "focus must be unaffected by a removal of an already-closed session"
    );
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        1,
        "no tab must be lost or duplicated by the idempotent removal"
    );
    Ok(())
}

// Bootstrap invariant (rules R4/R5, supporting test): the session-removed
// subscriber is spawned and the degenerate defaults (closed receiver +
// empty snapshot) leak no events and seed no tabs.
#[tokio::test]
async fn bootstrap_spawns_the_session_removed_subscriber_with_degenerate_defaults() -> TestResult {
    let (mock, mut app) = booted_app().await?;
    assert_eq!(
        app.subscriber_task_count(),
        common::BOOTSTRAP_SUBSCRIBER_TASKS,
        "bootstrap must spawn the full subscriber set (BUG-351: one shared total)"
    );
    let stale: Vec<Action> = actions_after_subscribers_ran(&mut app)
        .await
        .into_iter()
        .filter(|a| matches!(a, Action::SessionRemoved(_)))
        .collect();
    assert!(
        stale.is_empty(),
        "a closed session-removed receiver must never emit SessionRemoved; got {stale:?}"
    );
    assert!(
        app.agent_view_store().open_sessions().is_empty(),
        "an empty session_snapshot must seed no tabs"
    );
    drop(mock); // the backend stays alive for the subscriber's lifetime
    Ok(())
}
