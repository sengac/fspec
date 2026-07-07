//! RPC-416 — Inline reconnect status in scrollback (replace-in-place + auto-dismiss).
//!
//! Feature: spec/features/inline-reconnect-status-in-scrollback.feature
//!
//! These integration tests pin the target behaviour: the reconnect UI is a
//! set of INLINE scrollback lines in the ORIGINATING (focused-at-disconnect)
//! session — NOT the `DisconnectDialog` modal. On `Action::Disconnected` a
//! single `Reconnecting` line is pushed; `Action::Reconnecting(n)` updates it
//! in place; `Action::Reconnected` replaces it in place with a `Reconnected`
//! success line which then auto-dismisses after a short delay. The modal must
//! never appear.
//!
//! RED PHASE NOTE: the current dispatch path pushes a `DisconnectDialog`
//! compositor layer on `Action::Disconnected` (app/dispatch.rs:39-41) and
//! removes it on `Action::Reconnected` (dispatch_reconnect.rs:29). It pushes
//! NOTHING into scrollback and arms NO auto-dismiss timer. Therefore every
//! behavioural assertion below (inline line present, single line, replaced in
//! place, removed after delay, revert on re-drop, originating-session
//! targeting, no modal) FAILS against the current code. These tests are
//! expected to be RED until the inline-reconnect implementation lands.
//!
//! The tests deliberately reference ONLY existing public API (`Action`,
//! `App`, scrollback accessors). The forthcoming
//! `Action::ClearReconnectNotice { session_id, seq }` auto-dismiss action is
//! NOT named directly — it is exercised generically via `pump_actions`, which
//! dispatches whatever the timer emits onto the bus. This keeps the file a
//! clean BEHAVIOUR-red (compiles now, fails on assertions) rather than a
//! compile-red.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;

mod common;
use common::MockBackend;

/// Stable id of the DisconnectDialog compositor layer that RPC-416 removes.
const DISCONNECT_DIALOG_ID: &str = "disconnect-dialog";

/// Upper bound on the auto-dismiss delay (design: ~1.5-2s). Tests wait a
/// little past this to prove the success line has (or has not) been removed.
const DISMISS_WAIT: Duration = Duration::from_millis(2300);

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Build an App on a fresh MockBackend with one open, focused session `id`.
fn app_with_session(id: &str) -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid(id)));
    (app, mock)
}

/// Drain the action bus, dispatching every queued action. Also dispatches any
/// action emitted by the auto-dismiss timer (e.g. the future
/// `Action::ClearReconnectNotice`) without naming that variant.
fn pump_actions(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

/// Source texts of every scrollback chunk in session `id` whose text mentions
/// a reconnect state ("Reconnecting" or "Reconnected").
fn reconnect_lines(app: &App, id: &SessionId) -> Vec<String> {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| c.source.as_ref().map(|s| s.text.clone()))
        .filter(|t| t.contains("Reconnect"))
        .collect()
}

fn reconnect_line_count(app: &App, id: &SessionId) -> usize {
    reconnect_lines(app, id).len()
}

fn modal_present(app: &App) -> bool {
    app.compositor().contains(DISCONNECT_DIALOG_ID)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Disconnect shows an inline reconnecting line in the focused session
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_shows_an_inline_reconnecting_line_in_the_focused_session() {
    // @step Given a focused session with an open transcript
    let (mut app, _mock) = app_with_session("s-1");

    // @step When the connection drops and Action::Disconnected is dispatched
    app.dispatch(Action::Disconnected);

    // @step Then the focused session's scrollback gains a single inline reconnecting status line
    let lines = reconnect_lines(&app, &sid("s-1"));
    assert_eq!(
        lines.len(),
        1,
        "exactly one inline reconnecting line expected, got: {lines:?}"
    );
    assert!(
        lines[0].contains("Reconnecting"),
        "the inline line must be a reconnecting status, got: {:?}",
        lines[0]
    );

    // @step And no DisconnectDialog modal is present on the compositor
    assert!(
        !modal_present(&app),
        "the DisconnectDialog modal must not be pushed for an inline disconnect"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The DisconnectDialog modal never appears for a disconnect or
//           reconnect flow
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_disconnectdialog_modal_never_appears_for_a_disconnect_or_reconnect_flow() {
    // @step Given a focused session with an open transcript
    let (mut app, _mock) = app_with_session("s-1");

    // @step When the connection drops, retries, and then recovers
    let mut ever_present = false;
    app.dispatch(Action::Disconnected);
    ever_present |= modal_present(&app);
    app.dispatch(Action::Reconnecting(1));
    ever_present |= modal_present(&app);
    app.dispatch(Action::Reconnecting(2));
    ever_present |= modal_present(&app);
    app.dispatch(Action::Reconnected);
    ever_present |= modal_present(&app);

    // @step Then the compositor never contains the disconnect-dialog layer at any point in the flow
    assert!(
        !ever_present,
        "the disconnect-dialog layer must never appear during the inline reconnect flow"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Each reconnect attempt updates the same inline line in place
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_reconnect_attempt_updates_the_same_inline_line_in_place() {
    // @step Given a focused session showing an inline reconnecting line after a disconnect
    let (mut app, _mock) = app_with_session("s-1");
    app.dispatch(Action::Disconnected);
    assert_eq!(
        reconnect_line_count(&app, &sid("s-1")),
        1,
        "precondition: one inline reconnecting line after disconnect"
    );

    // @step When Action::Reconnecting(3) is dispatched for the third retry attempt
    app.dispatch(Action::Reconnecting(3));

    // @step Then the same inline line updates in place to show the attempt count
    let lines = reconnect_lines(&app, &sid("s-1"));
    assert_eq!(lines.len(), 1, "still exactly one reconnect line, got: {lines:?}");
    assert!(
        lines[0].contains("attempt 3") || lines[0].contains("(attempt 3)"),
        "the inline line must reflect attempt 3 in place, got: {:?}",
        lines[0]
    );

    // @step And the focused session's scrollback still contains only one reconnect line
    assert_eq!(
        reconnect_line_count(&app, &sid("s-1")),
        1,
        "no additional reconnect lines may be pushed per attempt"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Successful reconnect replaces the inline line in place with a
//           success message
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn successful_reconnect_replaces_the_inline_line_in_place_with_a_success_message() {
    // @step Given a focused session showing an inline reconnecting line after a disconnect
    let (mut app, _mock) = app_with_session("s-1");
    app.dispatch(Action::Disconnected);
    assert_eq!(
        reconnect_line_count(&app, &sid("s-1")),
        1,
        "precondition: one inline reconnecting line after disconnect"
    );

    // @step When the connection recovers and Action::Reconnected is dispatched
    app.dispatch(Action::Reconnected);

    // @step Then the same inline line is replaced in place with a reconnected success message
    let lines = reconnect_lines(&app, &sid("s-1"));
    assert_eq!(lines.len(), 1, "still exactly one reconnect line, got: {lines:?}");
    assert!(
        lines[0].contains("Reconnected"),
        "the inline line must be replaced with a reconnected success message, got: {:?}",
        lines[0]
    );

    // @step And the focused session's scrollback still contains only one reconnect line
    assert_eq!(
        reconnect_line_count(&app, &sid("s-1")),
        1,
        "replace must be in place — no extra reconnect line"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The success line auto-dismisses from scrollback after the short delay
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_success_line_auto_dismisses_from_scrollback_after_the_short_delay() {
    // @step Given a focused session showing an inline reconnected success line after a recovery
    let (mut app, _mock) = app_with_session("s-1");
    app.dispatch(Action::Disconnected);
    app.dispatch(Action::Reconnected);
    let lines = reconnect_lines(&app, &sid("s-1"));
    assert!(
        lines.iter().any(|l| l.contains("Reconnected")),
        "precondition: a reconnected success line must be showing, got: {lines:?}"
    );

    // @step When the auto-dismiss delay elapses and the pending clear action is processed
    tokio::time::sleep(DISMISS_WAIT).await;
    pump_actions(&mut app);

    // @step Then the success line is removed from the focused session's scrollback
    let after = reconnect_lines(&app, &sid("s-1"));
    assert!(
        !after.iter().any(|l| l.contains("Reconnected")),
        "the success line must be removed after the auto-dismiss delay, got: {after:?}"
    );

    // @step And the focused session's scrollback contains no reconnect line
    assert_eq!(
        reconnect_line_count(&app, &sid("s-1")),
        0,
        "no reconnect line may remain after auto-dismiss"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A re-drop during the success window cancels the dismiss and
//           reverts to reconnecting
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_re_drop_during_the_success_window_cancels_the_dismiss_and_reverts_to_reconnecting() {
    // @step Given a focused session showing an inline reconnected success line after a recovery
    let (mut app, _mock) = app_with_session("s-1");
    app.dispatch(Action::Disconnected);
    app.dispatch(Action::Reconnected);
    let lines = reconnect_lines(&app, &sid("s-1"));
    assert!(
        lines.iter().any(|l| l.contains("Reconnected")),
        "precondition: a reconnected success line must be showing, got: {lines:?}"
    );

    // @step When the connection drops again before the auto-dismiss delay elapses
    app.dispatch(Action::Disconnected);

    // @step Then the inline line reverts to a reconnecting status
    let reverted = reconnect_lines(&app, &sid("s-1"));
    assert_eq!(reverted.len(), 1, "still exactly one reconnect line, got: {reverted:?}");
    assert!(
        reverted[0].contains("Reconnecting") && !reverted[0].contains("Reconnected"),
        "the line must revert to a reconnecting status, got: {:?}",
        reverted[0]
    );

    // @step And the line is still present after the original auto-dismiss delay would have elapsed
    tokio::time::sleep(DISMISS_WAIT).await;
    pump_actions(&mut app);
    let still = reconnect_lines(&app, &sid("s-1"));
    assert!(
        still.iter().any(|l| l.contains("Reconnecting")),
        "the reconnecting line must survive — the superseded dismiss timer must be cancelled, got: {still:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Replace and remove target the originating session even after
//           focus changes
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn replace_and_remove_target_the_originating_session_even_after_focus_changes() {
    // @step Given a focused session A showing an inline reconnecting line after a disconnect
    let mock = Arc::new(MockBackend::new());
    // Script create_session to return B's id so the reconnect re-bootstrap's
    // create_session(None) is an idempotent no-op (B already open) and never
    // steals focus back to a spurious new session.
    mock.script_create_session(sid("B"));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("A")));
    app.dispatch(Action::Disconnected);
    assert_eq!(
        reconnect_line_count(&app, &sid("A")),
        1,
        "precondition: A shows one inline reconnecting line"
    );

    // @step And a second session B is created and becomes focused
    app.dispatch(Action::SessionCreated(sid("B")));
    assert_eq!(
        app.current_session(),
        Some(sid("B")),
        "precondition: B is now the focused session"
    );

    // @step When the connection recovers and Action::Reconnected is dispatched
    app.dispatch(Action::Reconnected);
    tokio::time::sleep(Duration::from_millis(50)).await;

    // @step Then session A's scrollback shows the reconnected success line
    let a_lines = reconnect_lines(&app, &sid("A"));
    assert!(
        a_lines.iter().any(|l| l.contains("Reconnected")),
        "the success line must land in the ORIGINATING session A, got: {a_lines:?}"
    );

    // @step And session B's scrollback contains no reconnect line
    assert_eq!(
        reconnect_line_count(&app, &sid("B")),
        0,
        "the currently-focused session B must not receive a reconnect line"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Closing the originating session before the timer fires is a
//           silent no-op
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn closing_the_originating_session_before_the_timer_fires_is_a_silent_no_op() {
    // @step Given a focused session A showing an inline reconnected success line after a recovery
    let (mut app, _mock) = app_with_session("A");
    app.dispatch(Action::Disconnected);
    app.dispatch(Action::Reconnected);
    let a_lines = reconnect_lines(&app, &sid("A"));
    assert!(
        a_lines.iter().any(|l| l.contains("Reconnected")),
        "precondition: A shows a reconnected success line, got: {a_lines:?}"
    );

    // @step When session A is closed before the auto-dismiss delay elapses
    let removed = app.agent_view_store_mut().remove_session_if_open(&sid("A"));
    assert!(removed, "session A must be closed before the timer fires");

    // @step And the pending clear action is processed after the delay
    tokio::time::sleep(DISMISS_WAIT).await;
    pump_actions(&mut app);

    // @step Then no panic occurs and no stale reconnect line remains
    assert!(
        app.agent_view_store().session_context_for(&sid("A")).is_none(),
        "session A must remain closed — the clear must be a silent no-op"
    );
    assert_eq!(
        reconnect_line_count(&app, &sid("A")),
        0,
        "no stale reconnect line may remain for a closed session"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A re-drop after the originating session closed shows a fresh
//           reconnecting line in the focused session
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_re_drop_after_the_originating_session_closed_shows_a_fresh_reconnecting_line_in_the_focused_session(
) {
    // @step Given a focused session A showing an inline reconnected success line after a recovery
    let mock = Arc::new(MockBackend::new());
    // Keep the reconnect re-bootstrap's create_session(None) idempotent (A is
    // already open) so it never spawns a spurious session or steals focus.
    mock.script_create_session(sid("A"));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("A")));
    app.dispatch(Action::Disconnected);
    app.dispatch(Action::Reconnected);
    let a_lines = reconnect_lines(&app, &sid("A"));
    assert!(
        a_lines.iter().any(|l| l.contains("Reconnected")),
        "precondition: A shows a reconnected success line, got: {a_lines:?}"
    );

    // @step And session A is closed and session B becomes focused
    app.dispatch(Action::SessionCreated(sid("B")));
    assert_eq!(
        app.current_session(),
        Some(sid("B")),
        "precondition: B must be the focused session"
    );
    let removed = app.agent_view_store_mut().remove_session_if_open(&sid("A"));
    assert!(removed, "session A must be closed before the re-drop");

    // @step When the connection drops again before the auto-dismiss delay elapses
    app.dispatch(Action::Disconnected);

    // @step Then session B's scrollback shows an inline reconnecting line
    let b_lines = reconnect_lines(&app, &sid("B"));
    assert_eq!(
        b_lines.len(),
        1,
        "the re-drop must push a fresh reconnecting line into the focused session B, got: {b_lines:?}"
    );
    assert!(
        b_lines[0].contains("Reconnecting") && !b_lines[0].contains("Reconnected"),
        "session B's fresh line must be a reconnecting status, got: {:?}",
        b_lines[0]
    );

    // @step And no stale reconnect notice remains tracked for the closed session A
    // Tracking must have moved to B: a subsequent successful reconnect updates
    // B's line in place. Had the notice still pointed at the dead session A, the
    // replace would target the closed A and B's line would never become success.
    app.dispatch(Action::Reconnected);
    let b_after = reconnect_lines(&app, &sid("B"));
    assert!(
        b_after.iter().any(|l| l.contains("Reconnected")),
        "tracking must have moved to B — a reconnect must update B's line, got: {b_after:?}"
    );
    assert_eq!(
        reconnect_line_count(&app, &sid("A")),
        0,
        "no stale reconnect line may remain for the closed session A"
    );
}
