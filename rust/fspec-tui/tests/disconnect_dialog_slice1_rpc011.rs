//! Slice 1 (CR-1 baseline) integration tests — RPC-011 disconnect dialog
//! + Action::Disconnected/Reconnecting/Reconnected wiring.
//!
//! Feature: spec/features/disconnect-dialog-cr1-baseline.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios. The tests below
//! cover the first sub-slice of RPC-011 — the CR-1 baseline absorbed from
//! the RPC-010 review (spec/attachments/RPC-010/review-findings.md, finding
//! CR-1). Auto-reconnect supervisor + backoff scheduling are tested in
//! Slice 2 (separate test file).
//!
//! Scenarios covered:
//!   1. "WebSocketFspecBackend surfaces WS disconnect as Action::Disconnected"
//!   2. "Action::Disconnected pushes an inline reconnecting line, not the DisconnectDialog modal"
//!
//! TUI-132 removed the DisconnectDialog (never shown since RPC-416) with
//! its q/r key scenarios.

use codelet_test_helpers::{OrFail, OrFailErr, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::transport::websocket::WebSocketFspecBackend;
use codelet_fspec_tui::{Action, App, FspecBackend};
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
async fn websocketfspecbackend_surfaces_ws_disconnect_as_action_disconnected() -> TestResult {
    // @step Given a fspec daemon is running on 127.0.0.1:<port> and a fspec client is attached via WebSocketFspecBackend::connect_with_supervisor
    let (_dir, service) = common::temp_service()?;
    let (addr, stats, _server_join) = common::start_ws_server_with_stats(service).await?;
    let (action_tx, mut action_rx) = unbounded_channel::<Action>();
    let url = common::ws_url(addr)?;
    let backend = WebSocketFspecBackend::connect_with_supervisor(url, action_tx.clone())
        .await
        .or_fail("connect_with_supervisor must succeed against the live daemon")?;

    // @step And the client's App has finished bootstrap (work-units seeded, session created, three subscriber tasks alive)
    let backend_arc: Arc<dyn FspecBackend> = Arc::new(backend);
    let mut app = App::new(backend_arc.clone());
    app.bootstrap()
        .await
        .or_fail("bootstrap must succeed against the live daemon")?;

    // @step When the daemon process is killed
    // We simulate `daemon process is killed` by triggering the same
    // graceful-drain path the daemon takes on SIGTERM: notify
    // ServerStats.shutdown_signal so each per-connection task sends a
    // WS Close{going_away} frame and exits. The client supervisor
    // observes the chunks broadcast closing immediately afterwards.
    codelet_rpc_server::request_shutdown(&stats);

    // @step Then within one render tick the App's action bus receives an Action::Disconnected message
    let observed = codelet_test_helpers::deadlock_guard("Action::Disconnected", async {
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
    .or_fail("the deadlock guard expired before the action bus yielded")?;
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
    let msg = format!("{:?}", err.or_fail_err("expected an error")?);
    assert!(
        msg.contains("Disconnected") || msg.contains("disconnected"),
        "error must indicate disconnected state, got {msg}"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 2: Action::Disconnected pushes an inline reconnecting line, NOT
//             the DisconnectDialog modal (RPC-416 replaced the modal)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_pushes_inline_reconnecting_line_not_the_modal() -> TestResult {
    // @step Given an App driving a ratatui TestBackend with a WebSocketFspecBackend whose connection has just dropped
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let (mut app, _terminal) = test_app(backend)?;
    // RPC-416: the inline notice targets the FOCUSED session.
    app.dispatch(Action::SessionCreated(codelet_rpc_types::SessionId::new(
        "s-1",
    )));

    // @step When the action loop processes Action::Disconnected
    app.send_action(Action::Disconnected)
        .or_fail("send_action must succeed")?;
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
    Ok(())
}
