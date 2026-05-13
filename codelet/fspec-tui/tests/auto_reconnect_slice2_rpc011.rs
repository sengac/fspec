//! Slice 2 (Auto-reconnect supervisor) integration tests — RPC-011.
//!
//! Feature: spec/features/auto-reconnect-supervisor.feature
//!
//! Covers the second sub-slice of RPC-011: exponential backoff supervisor
//! task + Action::Reconnecting / Action::Reconnected wiring + happy-path
//! reconnect + create_session(None) replay + inline dialog text update +
//! ServerGoingAway observable from the client.
//!
//! Red phase: requires `WebSocketFspecBackend::connect_with_supervisor`,
//! `Action::Reconnecting(u32)`, `Action::Reconnected`, `Action::ManualReconnect`,
//! plus the supervisor task itself. Compile failure IS the red signal for
//! these scenarios — none of the supervisor / action variants exist yet.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use codelet_fspec_tui::transport::websocket::WebSocketFspecBackend;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;
use tokio::sync::mpsc::unbounded_channel;

mod common;

use common::{render_one_frame, start_ws_server_with_stats, temp_service, test_app, ws_url, MockBackend};

/// Helper: drain the App's action bus, dispatching each Action.
fn pump_actions(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

/// Helper: render the App and return the screen as a single string.
fn render_to_string(
    terminal: &mut ratatui::Terminal<ratatui::backend::TestBackend>,
    app: &mut App,
) -> String {
    let buf = render_one_frame(terminal, app);
    let mut out = String::with_capacity((buf.area.width * buf.area.height) as usize);
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 6: Auto-reconnect backoff schedule
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_reconnect_backoff_schedule() {
    // @step Given a fspec client whose daemon has just died
    // @step And the supervisor task has been spawned by WebSocketFspecBackend::connect_with_supervisor
    let (action_tx, mut action_rx) = unbounded_channel::<Action>();
    // Point the supervisor at a closed port so connect_async always fails.
    // The supervisor must keep retrying with the expected backoff schedule.
    let url = url::Url::parse("ws://127.0.0.1:1/").expect("static url parse");
    // connect_with_supervisor returns Err on first attempt failure but
    // the supervisor task continues to retry in the background. We use
    // the variant that returns a backend handle even before first connect
    // succeeds — pure-supervisor mode (see rule [21]).
    let _ = WebSocketFspecBackend::connect_with_supervisor(url, action_tx.clone()).await;

    // @step When the daemon stays dead for 60 seconds
    // We don't actually wait 60s — we capture the timestamps of the
    // first 7 Action::Reconnecting(n) frames and verify the delay
    // between them matches the expected schedule.
    let mut observed: Vec<(u32, Instant)> = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    while observed.len() < 7 && Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_secs(10), action_rx.recv()).await {
            Ok(Some(Action::Reconnecting(n))) => observed.push((n, Instant::now())),
            Ok(Some(_)) => continue,
            Ok(None) => break,
            Err(_) => break,
        }
    }

    assert!(
        observed.len() >= 7,
        "supervisor must emit at least 7 Reconnecting attempts within 15s, got {}",
        observed.len()
    );

    // @step Then the supervisor emits Action::Reconnecting(attempt) frames with the following delays before each attempt:
    //   | attempt | delay_ms |
    //   | 1       | 250      |
    //   | 2       | 500      |
    //   | 3       | 1000     |
    //   | 4       | 2000     |
    //   | 5       | 5000     |
    //   | 6       | 5000     |
    //   | 7       | 5000     |
    let expected_ms: [u64; 6] = [250, 500, 1000, 2000, 5000, 5000];
    // Allow ±150ms jitter per gap to absorb tokio scheduler variability.
    for (i, expected) in expected_ms.iter().enumerate() {
        let dt = observed[i + 1].1.saturating_duration_since(observed[i].1);
        let dt_ms = dt.as_millis() as u64;
        let lower = expected.saturating_sub(150);
        let upper = expected + 1000; // generous upper bound under CI load
        assert!(
            dt_ms >= lower && dt_ms <= upper,
            "gap {i} between attempt {} and attempt {} = {dt_ms}ms; expected ~{expected}ms",
            observed[i].0,
            observed[i + 1].0,
        );
    }

    // @step And the attempt counter is strictly monotonically increasing
    for window in observed.windows(2) {
        assert!(
            window[1].0 > window[0].0,
            "attempt counter must be strictly increasing: {} then {}",
            window[0].0,
            window[1].0,
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 7: Auto-reconnect happy path
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn auto_reconnect_happy_path() {
    // @step Given a fspec client whose daemon has just died
    let (_dir, service) = temp_service();
    let (addr, stats, server_join) =
        start_ws_server_with_stats(Arc::clone(&service)).await;
    let (action_tx, mut action_rx) = unbounded_channel::<Action>();
    let url = ws_url(addr);
    let _backend = WebSocketFspecBackend::connect_with_supervisor(url.clone(), action_tx.clone())
        .await
        .expect("initial supervisor connect must succeed");

    // @step And the DisconnectDialog is topmost showing "auto-reconnecting (attempt 1)…"
    codelet_rpc_server::request_shutdown(&stats);
    server_join.abort();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Drain until we see Disconnected and at least one Reconnecting.
    let mut saw_disconnected = false;
    let mut saw_reconnecting = false;
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline && !(saw_disconnected && saw_reconnecting) {
        match tokio::time::timeout(Duration::from_millis(500), action_rx.recv()).await {
            Ok(Some(Action::Disconnected)) => saw_disconnected = true,
            Ok(Some(Action::Reconnecting(_))) => saw_reconnecting = true,
            Ok(Some(_)) => continue,
            _ => break,
        }
    }
    assert!(saw_disconnected, "must observe Action::Disconnected");
    assert!(saw_reconnecting, "must observe Action::Reconnecting(...) at least once");

    // @step When a new fspec daemon binds the same port within 2 seconds
    // Rebind on the same port the supervisor is trying. We need to
    // rebind to addr's exact port — bind_and_serve currently uses 127.0.0.1:0
    // so we explicitly pass `addr` here.
    let bind_addr = format!("127.0.0.1:{}", addr.port());
    let (new_addr, _stats, _new_join) =
        codelet_rpc_server::bind_and_serve(&bind_addr, Arc::clone(&service))
            .await
            .expect("rebind on same port");
    assert_eq!(new_addr.port(), addr.port(), "rebind must use same port");

    // @step Then the supervisor's next connect_async succeeds
    // @step And the supervisor re-issues list_work_units + create_session(None) on the new client
    // @step And it respawns the three subscriber tasks against the new chunks/logs/work_units broadcasts
    // @step And it emits Action::Reconnected on the App action bus
    let mut saw_reconnected = false;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !saw_reconnected {
        match tokio::time::timeout(Duration::from_secs(8), action_rx.recv()).await {
            Ok(Some(Action::Reconnected)) => saw_reconnected = true,
            Ok(Some(_)) => continue,
            _ => break,
        }
    }
    assert!(
        saw_reconnected,
        "supervisor must emit Action::Reconnected after the new daemon binds"
    );

    // @step And the App pops the DisconnectDialog from the Compositor
    // @step And the WorkUnitsListView re-seeds from the snapshot returned by the new daemon
    // (These two are App-level behaviours covered by the dispatch logic
    // wired to Action::Reconnected — covered by the inline-text scenario
    // below; here we only assert the supervisor's responsibility.)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 8: Reconnect re-issues create_session(None) and replaces
//             active session id
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconnect_reissues_create_session_and_replaces_active_session_id() {
    // @step Given a fspec client with active_session = SessionId("S-old") before disconnect
    let mock = Arc::new(MockBackend::new());
    mock.script_create_session(SessionId::new("S-new"));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let (mut app, _terminal) = test_app(backend);
    // Pre-seed the App's active session to "S-old" via direct dispatch
    // (bypassing bootstrap; bootstrap would otherwise call create_session
    // and use the scripted "S-new" too early).
    app.dispatch(Action::SessionCreated(SessionId::new("S-old")));
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("S-old")),
        "precondition: active session must be S-old"
    );

    // @step When the supervisor reconnects against a fresh daemon
    // Surface assertion: dispatching Action::Reconnected triggers the
    // App-side reconnect-bootstrap path (re-issue list_work_units +
    // create_session(None)). The supervisor itself runs in the WS
    // backend; here we exercise the App's reaction to Reconnected.
    let initial_calls = mock.create_session_calls();
    app.send_action(Action::Reconnected).expect("send_action");
    pump_actions(&mut app);
    // Yield so any tokio::spawn'd create_session call completes.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // @step Then it calls backend.create_session(None) and gets back SessionId("S-new")
    assert!(
        mock.create_session_calls() > initial_calls,
        "Reconnected must trigger a fresh create_session(None) call"
    );

    // @step And it emits Action::SessionCreated(SessionId("S-new")) onto the action bus
    // @step And the App's repl_active_session() returns Some(SessionId("S-new"))
    pump_actions(&mut app);
    tokio::time::sleep(Duration::from_millis(50)).await;
    pump_actions(&mut app);
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("S-new")),
        "after Reconnected, current_session must reflect the new session id"
    );

    // @step And the REPL transcript is NOT destructively truncated (old transcript lines remain on screen)
    // (Surface assertion: AgentReplView's transcript field still contains
    // pre-reconnect state. Covered by exposing AgentReplView::transcript_len
    // accessor in implementation phase; here we assert via the active-session
    // swap only.)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 9: Auto-reconnect Reconnecting Action updates the dialog text inline
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_reconnect_reconnecting_action_updates_dialog_text_inline() {
    // @step Given a TestBackend App with the DisconnectDialog topmost
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let (mut app, mut terminal) = test_app(backend);
    app.send_action(Action::Disconnected).unwrap();
    pump_actions(&mut app);
    let initial_layer_count = app.compositor().len();
    assert_eq!(
        app.compositor().topmost_id(),
        Some("disconnect-dialog".to_string()),
        "precondition: DisconnectDialog topmost"
    );

    // @step When the action loop dispatches Action::Reconnecting(3)
    app.send_action(Action::Reconnecting(3)).unwrap();
    pump_actions(&mut app);

    // @step Then the rendered Buffer contains "auto-reconnecting (attempt 3)…"
    let rendered = render_to_string(&mut terminal, &mut app);
    assert!(
        rendered.contains("auto-reconnecting (attempt 3)"),
        "rendered buffer must contain 'auto-reconnecting (attempt 3)'. Got:\n{rendered}"
    );

    // @step And no new dialog layer is pushed (the existing DisconnectDialog mutates state)
    assert_eq!(
        app.compositor().len(),
        initial_layer_count,
        "Reconnecting must mutate existing dialog, not push a new layer"
    );
    assert_eq!(
        app.compositor().topmost_id(),
        Some("disconnect-dialog".to_string()),
        "DisconnectDialog must still be topmost (no new layer pushed)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 10: Client receives ServerGoingAway when daemon shuts down gracefully
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn client_receives_server_going_away_when_daemon_shuts_down_gracefully() {
    // @step Given a fspec client connected to a daemon
    let (_dir, service) = temp_service();
    let (addr, stats, _server_join) =
        start_ws_server_with_stats(Arc::clone(&service)).await;
    let (action_tx, mut action_rx) = unbounded_channel::<Action>();
    let url = ws_url(addr);
    let backend = WebSocketFspecBackend::connect_with_supervisor(url, action_tx.clone())
        .await
        .expect("supervisor connect");
    let backend: Arc<dyn FspecBackend> = Arc::new(backend);
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");

    // @step When the daemon receives SIGTERM and broadcasts a WS Close frame with reason "going_away" (RFC 6455 code 1001)
    codelet_rpc_server::request_shutdown(&stats);

    // @step Then the client's WebSocketFspecBackend observes the close-with-reason inside 100 ms
    // @step And it emits Action::Disconnected onto the App action bus
    let start = Instant::now();
    let mut saw_disconnected = false;
    while Instant::now().duration_since(start) < Duration::from_millis(500) {
        match tokio::time::timeout(Duration::from_millis(50), action_rx.recv()).await {
            Ok(Some(Action::Disconnected)) => {
                saw_disconnected = true;
                break;
            }
            Ok(Some(_)) => continue,
            _ => continue,
        }
    }
    assert!(
        saw_disconnected,
        "client must emit Action::Disconnected within 500ms of going_away"
    );

    // @step And the DisconnectDialog renders the same "daemon disconnected | r to reconnect | q to quit" text
    // @step And the supervisor starts the same 250ms-first-attempt backoff loop
    // (Covered by scenarios 1, 2, and 6. Here we only assert the
    // observability path: Disconnected was emitted.)
}
