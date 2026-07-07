//! Slice 1 (CR-1 baseline) integration tests — RPC-011 disconnect dialog
//! + Action::Disconnected/Reconnecting/Reconnected wiring.
//!
//! Feature: spec/features/disconnect-dialog-cr1-baseline.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios. The five tests below
//! cover the first sub-slice of RPC-011 — the CR-1 baseline absorbed from
//! the RPC-010 review (spec/attachments/RPC-010/review-findings.md, finding
//! CR-1). Auto-reconnect supervisor + backoff scheduling are tested in
//! Slice 2 (separate test file).
//!
//! Scenarios covered:
//!   1. "WebSocketFspecBackend surfaces WS disconnect as Action::Disconnected"
//!   2. "DisconnectDialog is pushed at Priority::Critical when Action::Disconnected fires"
//!   3. "DisconnectDialog swallows navigation keys while topmost"
//!   4. "Pressing q in DisconnectDialog exits the client cleanly"
//!   5. "Pressing r in DisconnectDialog triggers a manual reconnect that resets backoff"
//!
//! Red phase: types `Action::Disconnected`, `Action::Reconnecting(u32)`,
//! `Action::Reconnected`, `DisconnectDialog`, and
//! `WebSocketFspecBackend::connect_with_supervisor` do NOT yet exist —
//! this test file will not compile until they are added in the implementing
//! phase. Compile failure IS the red signal for these scenarios.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::components::disconnect_dialog::DisconnectDialog;
use codelet_fspec_tui::transport::websocket::WebSocketFspecBackend;
use codelet_fspec_tui::{synth_key, Action, App, FspecBackend};
use crossterm::event::KeyCode;
use tokio::sync::mpsc::unbounded_channel;

mod common;

use common::{test_app, MockBackend};

/// Helper: drain the App's action bus by calling `try_recv_action` until
/// it returns None, dispatching each Action through `app.dispatch` so
/// the compositor + RootView observe them. Mirrors the run-loop body's
/// `Some(action) = self.action_rx.recv()` arm.
fn pump_actions(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 1: WebSocketFspecBackend surfaces WS disconnect as
//             Action::Disconnected
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: WebSocketFspecBackend surfaces WS disconnect as Action::Disconnected
///
/// Red phase: requires `WebSocketFspecBackend::connect_with_supervisor`
/// and `BackendError::Disconnected`. Tests the supervisor's drop-detect
/// path by spawning a real bind_and_serve, connecting via the supervisor
/// constructor, then aborting the server's JoinHandle to simulate
/// `daemon process is killed`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn websocketfspecbackend_surfaces_ws_disconnect_as_action_disconnected() {
    // @step Given a fspec daemon is running on 127.0.0.1:<port> and a fspec client is attached via WebSocketFspecBackend::connect_with_supervisor
    let (_dir, service) = common::temp_service();
    let (addr, stats, _server_join) = common::start_ws_server_with_stats(service).await;
    let (action_tx, mut action_rx) = unbounded_channel::<Action>();
    let url = common::ws_url(addr);
    let backend = WebSocketFspecBackend::connect_with_supervisor(url, action_tx.clone())
        .await
        .expect("connect_with_supervisor must succeed against the live daemon");

    // @step And the client's App has finished bootstrap (work-units seeded, session created, three subscriber tasks alive)
    let backend_arc: Arc<dyn FspecBackend> = Arc::new(backend);
    let mut app = App::new(backend_arc.clone());
    app.bootstrap()
        .await
        .expect("bootstrap must succeed against the live daemon");

    // @step When the daemon process is killed
    // We simulate `daemon process is killed` by triggering the same
    // graceful-drain path the daemon takes on SIGTERM: notify
    // ServerStats.shutdown_signal so each per-connection task sends a
    // WS Close{going_away} frame and exits. The client supervisor
    // observes the chunks broadcast closing immediately afterwards.
    codelet_rpc_server::request_shutdown(&stats);
    tokio::time::sleep(Duration::from_millis(200)).await;

    // @step Then within one render tick the App's action bus receives an Action::Disconnected message
    let observed = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if let Some(action) = action_rx.recv().await {
                if matches!(action, Action::Disconnected) {
                    return true;
                }
            } else {
                return false;
            }
        }
    })
    .await
    .expect("action bus must yield within 1s");
    assert!(
        observed,
        "supervisor must emit Action::Disconnected when WS drops"
    );

    // @step And the WebSocketFspecBackend's internal client slot becomes None
    let is_disconnected = backend_arc.health().await.map(|_| false).unwrap_or(true);
    assert!(
        is_disconnected,
        "after daemon abort, health() must fail (client slot is None)"
    );

    // @step And subsequent RPC calls on the backend return Err(BackendError::Disconnected) rather than panicking or hanging
    let err = backend_arc.list_work_units().await;
    assert!(
        err.is_err(),
        "list_work_units after disconnect must return Err, got {err:?}"
    );
    let msg = format!("{:?}", err.unwrap_err());
    assert!(
        msg.contains("Disconnected") || msg.contains("disconnected"),
        "error must indicate disconnected state, got {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 2: Action::Disconnected pushes an inline reconnecting line, NOT
//             the DisconnectDialog modal (RPC-416 replaced the modal)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_pushes_inline_reconnecting_line_not_the_modal() {
    // @step Given an App driving a ratatui TestBackend with a WebSocketFspecBackend whose connection has just dropped
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let (mut app, _terminal) = test_app(backend);
    // RPC-416: the inline notice targets the FOCUSED session.
    app.dispatch(Action::SessionCreated(codelet_rpc_types::SessionId::new("s-1")));

    // @step When the action loop processes Action::Disconnected
    app.send_action(Action::Disconnected)
        .expect("send_action must succeed");
    pump_actions(&mut app);

    // @step Then no DisconnectDialog modal layer is pushed onto the Compositor
    assert!(
        !app.compositor().contains("disconnect-dialog"),
        "RPC-416: the DisconnectDialog modal must never be auto-pushed on Disconnected"
    );

    // @step And the focused session's scrollback gains a single inline reconnecting status line
    let lines: Vec<String> = app
        .agent_view_store()
        .session_context_for(&codelet_rpc_types::SessionId::new("s-1"))
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| c.source.as_ref().map(|s| s.text.clone()))
        .filter(|t| t.contains("Reconnect"))
        .collect();
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
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 3: DisconnectDialog swallows navigation keys while topmost
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_dialog_swallows_navigation_keys_while_topmost() {
    // @step Given a TestBackend App with the DisconnectDialog currently topmost
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let (mut app, _terminal) = test_app(backend);
    // RPC-416: Disconnected no longer auto-pushes the modal, so the
    // dialog is constructed + pushed directly to exercise its CR-1
    // key-swallow behaviour (which still runs at events.rs Stage 1).
    app.compositor_mut().push(Box::new(DisconnectDialog::new()));
    assert_eq!(
        app.compositor().topmost_id(),
        Some("disconnect-dialog".to_string()),
        "precondition: DisconnectDialog must be topmost"
    );
    let initial_selected = app
        .board_store()
        .selected_index_for(app.board_store().focused_column());
    let initial_focused_column = app.board_store().focused_column().to_string();
    let initial_view = app.active_view();

    // @step When the user presses 'j', 'k', '?', and Tab in sequence
    let _ = app.handle_event(&synth_key(KeyCode::Char('j')));
    let _ = app.handle_event(&synth_key(KeyCode::Char('k')));
    let _ = app.handle_event(&synth_key(KeyCode::Char('?')));
    let _ = app.handle_event(&synth_key(KeyCode::Tab));

    // @step Then the WorkUnitsListView selection index does not change
    assert_eq!(
        app.board_store()
            .selected_index_for(&initial_focused_column),
        initial_selected,
        "BoardStore selection must not change while DisconnectDialog topmost"
    );

    // @step And the HelpDialog is not pushed onto the Compositor
    assert_ne!(
        app.compositor().topmost_id(),
        Some("help-dialog".to_string()),
        "HelpDialog must NOT be pushed while DisconnectDialog topmost"
    );

    // @step And the focused pane does not flip between WorkUnits and Repl
    // RPC-012: there are no panes — only the Navigator's active_view
    // (Board/Agent). Tab is also dropped at the navigator level per
    // RPC-012 rule [19]. Asserting active_view stays Board verifies
    // the dialog still swallows navigation keys.
    assert_eq!(
        app.active_view(),
        initial_view,
        "Navigator active_view must not change while DisconnectDialog topmost"
    );

    // @step And the DisconnectDialog remains topmost
    assert_eq!(
        app.compositor().topmost_id(),
        Some("disconnect-dialog".to_string()),
        "DisconnectDialog must still be topmost after navigation key presses"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 4: Pressing q in DisconnectDialog exits the client cleanly
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pressing_q_in_disconnect_dialog_exits_the_client_cleanly() {
    // @step Given a TestBackend App with the DisconnectDialog currently topmost
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let (mut app, _terminal) = test_app(backend);
    // RPC-416: Disconnected no longer auto-pushes the modal; push it
    // directly to exercise the CR-1 'q' quit binding.
    app.compositor_mut().push(Box::new(DisconnectDialog::new()));
    assert_eq!(
        app.compositor().topmost_id(),
        Some("disconnect-dialog".to_string())
    );
    assert!(
        !app.should_quit(),
        "precondition: should_quit must start false"
    );

    // @step When the user presses 'q'
    let _ = app.handle_event(&synth_key(KeyCode::Char('q')));

    // @step Then the App's should_quit flag becomes true
    assert!(
        app.should_quit(),
        "should_quit must become true after pressing 'q' in DisconnectDialog"
    );

    // @step And App::run returns Ok(()) and the client process exits with status 0
    // (The run loop is asserted via the should_quit flag; the actual
    // Ok(()) return is exercised by the existing q_at_app_level test —
    // we don't drive the full run loop here to keep this scenario hermetic.)

    // @step And no panic backtrace is printed on stderr
    // (Assertion is implicit: this test would panic if any layer paniced.)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 5: Pressing r in DisconnectDialog triggers a manual reconnect
//             that resets backoff
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pressing_r_in_disconnect_dialog_triggers_manual_reconnect_that_resets_backoff() {
    // @step Given a TestBackend App with the DisconnectDialog topmost during a 5-second backoff sleep
    //
    // We model "5-second backoff sleep" by sending Action::Reconnecting(5)
    // first (the supervisor would have emitted this between attempts 5+
    // when the backoff has reached its cap). The DisconnectDialog's
    // 'r' handler must:
    //   (a) cancel the supervisor's current sleep
    //   (b) reset the supervisor's backoff schedule so the NEXT failure
    //       starts at 250ms again.
    //
    // The supervisor exposes a `manual_reconnect_tx: UnboundedSender<()>`
    // accessor on `WebSocketFspecBackend` (per rule [22]) — the App's
    // `r` handler sends to this channel.
    //
    // For this test we use a stand-alone DisconnectDialog + the App's
    // action bus, asserting that `Action::ManualReconnect` is emitted
    // when 'r' is pressed. The supervisor-side cancel/reset behaviour is
    // re-asserted in the Slice 2 backoff-cap test where the real
    // supervisor loop is driven.
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let (mut app, _terminal) = test_app(backend);
    // RPC-416: Disconnected no longer auto-pushes the modal; push it
    // directly to exercise the CR-1 'r' manual-reconnect binding.
    app.compositor_mut().push(Box::new(DisconnectDialog::new()));
    // Advance to attempt 5 (cap reached) so the "during a 5-second sleep"
    // precondition is recorded on the dialog state.
    app.send_action(Action::Reconnecting(5)).unwrap();
    pump_actions(&mut app);
    assert_eq!(
        app.compositor().topmost_id(),
        Some("disconnect-dialog".to_string()),
        "precondition: DisconnectDialog topmost"
    );

    // Drain any actions the bus may already hold so we can assert on
    // the NEW action emitted by 'r'.
    while app.try_recv_action().is_some() {}

    // @step When the user presses 'r'
    let _ = app.handle_event(&synth_key(KeyCode::Char('r')));

    // @step Then the reconnect supervisor cancels the current sleep and attempts connect immediately
    // Surface assertion: `Action::ManualReconnect` is emitted onto the
    // action bus. The supervisor's `manual_reconnect_rx.recv()` arm
    // races against the backoff sleep — receiving from this channel
    // cancels the sleep. The end-to-end cancel-cum-reset behaviour is
    // covered by the Slice 2 backoff-cap scenario.
    let mut saw_manual_reconnect = false;
    while let Some(action) = app.try_recv_action() {
        if matches!(action, Action::ManualReconnect) {
            saw_manual_reconnect = true;
            break;
        }
    }
    assert!(
        saw_manual_reconnect,
        "pressing 'r' in DisconnectDialog must emit Action::ManualReconnect"
    );

    // @step And on the next failure the backoff schedule restarts from 250ms (not 5s)
    // Covered by Slice 2 backoff-cap test where the supervisor's
    // internal state is driven through a full backoff cycle. Here we
    // verify only the trigger — the supervisor's reset-on-manual
    // behaviour is implementation-side and asserted against the live
    // backoff schedule in slice 2.
}
