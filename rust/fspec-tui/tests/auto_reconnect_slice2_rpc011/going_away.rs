//! The graceful-shutdown scenario of the auto-reconnect slice.

use super::*;
use codelet_test_helpers::{OrFail, TestResult};

// ─────────────────────────────────────────────────────────────────────────
// Scenario 10: Client receives ServerGoingAway when daemon shuts down gracefully
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn client_receives_server_going_away_when_daemon_shuts_down_gracefully() -> TestResult {
    // @step Given a fspec client connected to a daemon
    let (_dir, service) = temp_service()?;
    let (addr, stats, _server_join) = start_ws_server_with_stats(Arc::clone(&service)).await?;
    let (action_tx, mut action_rx) = unbounded_channel::<Action>();
    let url = ws_url(addr)?;
    let backend = WebSocketFspecBackend::connect_with_supervisor(url, action_tx.clone())
        .await
        .or_fail("supervisor connect")?;
    let backend: Arc<dyn FspecBackend> = Arc::new(backend);
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;

    // @step When the daemon receives SIGTERM and broadcasts a WS Close frame with reason "going_away" (RFC 6455 code 1001)
    codelet_rpc_server::request_shutdown(&stats);

    // @step Then the client's WebSocketFspecBackend observes the close-with-reason inside 100 ms
    // @step And it emits Action::Disconnected onto the App action bus
    let saw_disconnected = wait_for_action(&mut action_rx, |action| {
        matches!(action, Action::Disconnected)
    })
    .await;
    assert!(
        saw_disconnected,
        "client must emit Action::Disconnected after going_away"
    );

    // @step And the focused session's scrollback shows an inline reconnecting line after the disconnect
    // @step And the supervisor starts the same 250ms-first-attempt backoff loop
    // (The inline presentation is asserted directly by the happy-path and
    // Scenario 9 tests above; the backoff loop is covered by scenarios 1, 2,
    // and 6. Here we only assert the observability path: Disconnected was
    // emitted onto the App action bus.)
    Ok(())
}
