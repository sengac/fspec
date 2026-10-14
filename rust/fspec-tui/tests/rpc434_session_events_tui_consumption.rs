//! RPC-434 — TUI consumes session lifecycle events (created/removed) +
//! connect-time snapshot replay.
//!
//! Feature: spec/features/session-events-tui-consumption.feature
//!
//! This file owns the four TUI-facing scenarios of the feature, driven
//! through the MockBackend (embedded-transport equivalent):
//!
//!   #1 Starting the TUI against a daemon with open sessions seeds a
//!      tab for each of them (connect-time snapshot replay)
//!   #2 A remote session creation adds a tab live (no poll)
//!   #3 A remote session removal closes its tab with focus and board
//!      cleanup
//!   #4 Removal events are idempotent for already-closed tabs
//!
//! RED PHASE NOTE: `Action::SessionRemoved`, the
//! `FspecBackend::session_removed_rx` / `FspecBackend::session_snapshot`
//! trait surface, `MockBackend::push_session_removed` /
//! `MockBackend::seed_session_snapshot`, the App-level
//! `handle_session_removed` handler and the bootstrap snapshot replay do
//! NOT exist yet — this file fails to COMPILE until RPC-434's
//! implementation lands. The WS live-wire counterpart (client A creates,
//! client B's tabs react) is covered at the wire level by
//! rpc-server/tests/rpc433_session_event_fan_out.rs; the App-level
//! behaviour is transport-agnostic, so these mock-driven tests pin the
//! TUI half.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, WorkUnitContext};

mod common;
use common::MockBackend;

fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

/// Count the open tabs whose SessionContext id matches `id`.
fn tabs_for(app: &App, id: &SessionId) -> usize {
    app.agent_view_store()
        .open_sessions()
        .iter()
        .filter(|c| &c.id == id)
        .count()
}

/// Drain the App's action bus until a matching action arrives or 2s
/// elapses (mirrors `wait_for_action` in
/// spawned_subordinate_session_registration_parity_rpc385.rs). Drains
/// every queued action each tick so a backlog cannot starve the
/// predicate.
async fn wait_for_action<F: Fn(&Action) -> bool>(app: &mut App, pred: F) -> Option<Action> {
    let deadline = std::time::Instant::now() + Duration::from_millis(2000);
    while std::time::Instant::now() < deadline {
        while let Some(action) = app.try_recv_action() {
            if pred(&action) {
                return Some(action);
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    None
}

// =============================================================================
// Scenario: Starting the TUI against a daemon with open sessions seeds a
//           tab for each of them
// =============================================================================

#[tokio::test]
async fn starting_the_tui_against_a_daemon_with_open_sessions_seeds_a_tab_for_each_of_them() {
    // @step Given a daemon whose session manager already holds two live sessions (created from another machine)
    // The connect-time open-session snapshot (RPC-433
    // `Envelope::SessionSnapshot`) is the mock's scripted seed — the
    // same payload the WS server pushes on connect.
    let mock = Arc::new(MockBackend::new());
    mock.seed_session_snapshot(vec![sid("S-snap-A"), sid("S-snap-B")]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When the TUI boots against that daemon (either transport)
    // `App::bootstrap` must fold EVERY snapshot session through the
    // existing `Action::SessionCreated` handler (idempotent append),
    // synchronously BEFORE `spawn_session_restore`.
    app.bootstrap().await.expect("bootstrap");

    // @step Then the TUI's session list contains a tab for each of those two sessions, seeded from the open-session snapshot before any session-creation RPC was issued by this client
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        2,
        "both open-session snapshot entries must seed tabs, got: {:?}",
        app.agent_view_store()
            .open_sessions()
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(tabs_for(&app, &sid("S-snap-A")), 1);
    assert_eq!(tabs_for(&app, &sid("S-snap-B")), 1);
    assert_eq!(
        mock.create_session_calls(),
        0,
        "the snapshot must seed tabs WITHOUT issuing a create_session RPC"
    );

    // Supporting invariant (rule R4): replay is idempotent — re-running
    // the same snapshot (e.g. a reconnect) must not duplicate tabs.
    app.dispatch(Action::SessionCreated(sid("S-snap-A")));
    app.dispatch(Action::SessionCreated(sid("S-snap-B")));
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        2,
        "replaying the snapshot over existing tabs must be a no-op (idempotent append)"
    );
}

// =============================================================================
// Scenario: A remote session creation adds a tab live (no poll)
// =============================================================================

#[tokio::test]
async fn a_remote_session_creation_adds_a_tab_live_no_poll() {
    // @step Given a TUI is connected to a daemon over WebSocket with no sessions open yet
    // The 7th bootstrap subscriber (session_removed_rx) + the existing
    // 5th (session_created_rx) are spawned by `bootstrap`; the mock's
    // session-created broadcast is the WS wire stream's equivalent.
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    assert!(
        app.agent_view_store().open_sessions().is_empty(),
        "precondition: no open tabs at bootstrap"
    );

    // @step When a session is created from another machine connected to the same daemon
    // The subscriber task folds the broadcast frame into
    // Action::SessionCreated on the action bus (the same path the WS
    // transport's session-event splitter feeds).
    let remote = sid("S-remote-1");
    mock.push_session_created(remote.clone());
    let action = wait_for_action(
        &mut app,
        |a| matches!(a, Action::SessionCreated(s) if s.value == "S-remote-1"),
    )
    .await
    .expect("the session-created event must reach the action bus");
    app.dispatch(action);

    // @step Then a new session tab appears in the TUI's session list without any poll or user action
    assert_eq!(
        tabs_for(&app, &remote),
        1,
        "a live remote creation must append a tab with no poll"
    );
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        1,
        "exactly one tab must exist"
    );
}

// =============================================================================
// Scenario: A remote session removal closes its tab with focus and board
//           cleanup
// =============================================================================

#[tokio::test]
async fn a_remote_session_removal_closes_its_tab_with_focus_and_board_cleanup() {
    // @step Given a TUI connected to a daemon shows two live session tabs, one of which is focused and has a work unit attached on the board
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    // "Connected to a daemon" = bootstrap has run — the 8th subscriber
    // (session_removed_rx) is what folds the mock's removed broadcast
    // onto the action bus below.
    app.bootstrap().await.expect("bootstrap");
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
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&gone),
        "precondition: the session with the board-attached work unit is focused"
    );
    assert!(
        app.board_store().session_for("WU-A").is_some(),
        "precondition: WU-A is attached to the doomed session on the board"
    );

    // @step When that session is closed from another machine connected to the same daemon
    // The bootstrap session-removed subscriber folds the broadcast frame
    // into Action::SessionRemoved on the action bus; the App dispatch arm
    // routes it to the idempotent handle_session_removed handler.
    mock.push_session_removed(gone.clone());
    let action = wait_for_action(
        &mut app,
        |a| matches!(a, Action::SessionRemoved(s) if s.value == "S-gone"),
    )
    .await
    .expect("the session-removed event must reach the action bus");
    app.dispatch(action);

    // @step Then the TUI removes that session's tab, the focused session becomes the surviving one, and the board's work-unit attachment for the removed session is released
    assert_eq!(
        tabs_for(&app, &gone),
        0,
        "the remote removal must close the doomed session's tab"
    );
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
}

// =============================================================================
// Scenario: Removal events are idempotent for already-closed tabs
// =============================================================================

#[tokio::test]
async fn removal_events_are_idempotent_for_already_closed_tabs() {
    // @step Given a TUI connected to a daemon and a session that has already been closed locally (its tab already removed)
    // (The mock is moved into the App: this test dispatches directly and
    // never pushes, so the Arc's keep-alive role is enough.)
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    let closed = sid("S-closed");
    let keep = sid("S-keep-1");
    app.dispatch(Action::SessionCreated(keep.clone()));
    // Local close path: the tab is already gone when the remote event
    // arrives (the local close path removes synchronously BEFORE the
    // daemon fires the event).
    app.agent_view_store_mut().remove_session_if_open(&closed);
    assert_eq!(
        tabs_for(&app, &closed),
        0,
        "precondition: the tab is closed locally"
    );

    // @step When the TUI processes a SessionRemoved event for that already-closed session
    // Direct dispatch (idempotency is a handler property, transport
    // independent) — fired TWICE to pin the double-fire no-op
    // (local close path + remote event).
    app.dispatch(Action::SessionRemoved(closed.clone()));
    app.dispatch(Action::SessionRemoved(closed));

    // @step Then the TUI ignores the event as a no-op (no panic, no duplicate cleanup, no effect on focus or the board)
    assert_eq!(
        tabs_for(&app, &keep),
        1,
        "the surviving tab must be untouched by a stale removal"
    );
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
}

// =============================================================================
// Bootstrap invariant (rules R4/R5): the session-removed subscriber task
// is spawned; the degenerate defaults (closed receiver + empty snapshot)
// leak no events and seed no tabs.
// =============================================================================

#[tokio::test]
async fn bootstrap_spawns_the_session_removed_subscriber_with_degenerate_defaults() {
    // @step Given an App constructed against a MockBackend (degenerate defaults: closed session-removed receiver, empty snapshot)
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When the App's bootstrap runs
    app.bootstrap().await.expect("bootstrap");

    // @step Then the session-removed subscriber task is alive alongside the other seven
    // (work_units + chunks + logs + status_changes + session_created +
    // checkpoints_progress + git_state = 7; RPC-434 adds the 8th:
    // session_removed; RPC-437 adds the 9th: session_cleared).
    assert_eq!(
        app.subscriber_task_count(),
        9,
        "bootstrap must spawn the new session-removed subscriber task (7 pre-RPC-434)"
    );

    // @step And the degenerate (closed) session-removed receiver must not leak SessionRemoved events onto the bus
    tokio::time::sleep(Duration::from_millis(100)).await;
    let stale = wait_for_action(&mut app, |a| matches!(a, Action::SessionRemoved(_))).await;
    assert!(
        stale.is_none(),
        "a closed session-removed receiver must never emit SessionRemoved; got {stale:?}"
    );
    assert!(
        app.agent_view_store().open_sessions().is_empty(),
        "an empty session_snapshot must seed no tabs"
    );
    let _ = mock; // keep the backend alive for the subscriber's lifetime
}
