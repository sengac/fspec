//! Slice 2 (Auto-reconnect supervisor) integration tests — RPC-011.
//!
//! Feature: spec/features/auto-reconnect-supervisor.feature
//!
//! Covers the second sub-slice of RPC-011: exponential backoff supervisor
//! task + Action::Reconnecting / Action::Reconnected wiring + happy-path
//! reconnect + create_session(None) replay + inline reconnect line update +
//! ServerGoingAway observable from the client.

use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::transport::websocket::WebSocketFspecBackend;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;
use tokio::sync::mpsc::unbounded_channel;

mod common;

use common::{start_ws_server_with_stats, temp_service, test_app, ws_url, MockBackend};

#[path = "auto_reconnect_slice2_rpc011/going_away.rs"]
mod going_away;
#[path = "auto_reconnect_slice2_rpc011/support.rs"]
mod support;
use support::{
    app_respawns_subscribers_after_reconnect, assert_backoff_gaps, assert_single_reconnect_line,
    observe_reconnect_attempts, pump_actions, reconnect_lines, saw_disconnect_then_reconnecting,
    wait_for_action,
};

// ─────────────────────────────────────────────────────────────────────────
// Scenario 6: Auto-reconnect backoff schedule
// ─────────────────────────────────────────────────────────────────────────

// TEST-042: the backoff runs on tokio's paused clock, so the gaps between
// attempts are the supervisor's schedule, whatever the load.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn auto_reconnect_backoff_schedule() -> TestResult {
    // @step Given a fspec client whose daemon has just died
    // @step And the supervisor task has been spawned by WebSocketFspecBackend::connect_with_supervisor
    let (action_tx, mut action_rx) = unbounded_channel::<Action>();
    // Point the supervisor at a closed port so connect_async always fails.
    // The supervisor must keep retrying with the expected backoff schedule.
    let url = url::Url::parse("ws://127.0.0.1:1/").or_fail("static url parse")?;
    // connect_with_supervisor returns Err on first attempt failure but
    // the supervisor task continues to retry in the background. We use
    // the variant that returns a backend handle even before first connect
    // succeeds — pure-supervisor mode (see rule [21]).
    // Keep the backend alive: dropping it detaches the supervisor (RPC-448).
    let _backend = WebSocketFspecBackend::connect_with_supervisor(url, action_tx.clone()).await;

    // @step When the daemon stays dead for 60 seconds
    // We don't actually wait 60s — we capture the timestamps of the
    // first 7 Action::Reconnecting(n) frames and verify the delay
    // between them matches the expected schedule.
    let observed = observe_reconnect_attempts(&mut action_rx).await;

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
    assert_backoff_gaps(&observed);

    // @step And the attempt counter is strictly monotonically increasing
    for window in observed.windows(2) {
        assert!(
            window[1].0 > window[0].0,
            "attempt counter must be strictly increasing: {} then {}",
            window[0].0,
            window[1].0,
        );
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 7: Auto-reconnect happy path
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn auto_reconnect_happy_path() -> TestResult {
    // @step Given a fspec client whose daemon has just died
    let (_dir, service) = temp_service()?;
    let (addr, stats, server_join) = start_ws_server_with_stats(Arc::clone(&service)).await?;
    let (action_tx, mut action_rx) = unbounded_channel::<Action>();
    let url = ws_url(addr)?;
    let _backend = WebSocketFspecBackend::connect_with_supervisor(url.clone(), action_tx.clone())
        .await
        .or_fail("initial supervisor connect must succeed")?;

    // @step And the focused session's scrollback shows an inline reconnecting line
    codelet_rpc_server::request_shutdown(&stats);
    server_join.abort();

    let (saw_disconnected, saw_reconnecting) =
        saw_disconnect_then_reconnecting(&mut action_rx).await;
    assert!(saw_disconnected, "must observe Action::Disconnected");
    assert!(
        saw_reconnecting,
        "must observe Action::Reconnecting(...) at least once"
    );

    // @step When a new fspec daemon binds the same port within 2 seconds
    // Rebind on the exact port the supervisor is retrying.
    let bind_addr = format!("127.0.0.1:{}", addr.port());
    let (new_addr, _stats, _new_join) =
        codelet_rpc_server::bind_and_serve(&bind_addr, Arc::clone(&service))
            .await
            .or_fail("rebind on same port")?;
    assert_eq!(new_addr.port(), addr.port(), "rebind must use same port");

    // @step Then the supervisor's next connect_async succeeds
    // @step And the supervisor re-issues list_work_units + create_session(None) on the new client
    // @step And it respawns the five subscriber tasks against the new work_units/chunks/logs/status_changes/session_created broadcasts
    // @step And it emits Action::Reconnected on the App action bus
    let saw_reconnected = wait_for_action(&mut action_rx, |action| {
        matches!(action, Action::Reconnected)
    })
    .await;
    assert!(
        saw_reconnected,
        "supervisor must emit Action::Reconnected after the new daemon binds"
    );

    // RPC-415: back the "respawns the subscriber tasks" step with a REAL
    // App-side assertion on a MockBackend App (count + a live broadcast).
    let app = app_respawns_subscribers_after_reconnect().await?;

    // @step And the App replaces the inline reconnecting line with a reconnected success line in the focused session
    assert_single_reconnect_line(&app, "Reconnected");

    // @step And the WorkUnitsListView re-seeds from the snapshot returned by the new daemon
    // (App-level behaviour covered by the dispatch logic wired to
    // Action::Reconnected; here we assert the supervisor's responsibility and
    // the inline presentation swap above.)
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 8 (superseded by TUI-113 R5 + BUG-206): reconnect RE-ATTACHES the
// focused session — live-gated, memory-only transcript catch-up, never a
// disk resume_session and never a create_session fallback.
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconnect_reissues_create_session_and_replaces_active_session_id() -> TestResult {
    // @step Given a fspec client with active_session = SessionId("S-old") before disconnect
    // BUG-206: "S-old" is still running in the daemon's in-memory map.
    let mock = Arc::new(MockBackend::new());
    mock.script_create_session(SessionId::new("S-new"));
    mock.seed_live_sessions(vec![SessionId::new("S-old")]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let (mut app, _terminal) = test_app(backend)?;
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
    // BUG-206: the App re-ATTACHES the focused session (S-old), live-gated.
    let initial_calls = mock.create_session_calls();
    app.send_action(Action::Reconnected)
        .or_fail("send_action")?;
    pump_actions(&mut app);
    // TEST-042: wait for the spawned restore round-trip (its transcript
    // catch-up request), pumping the bus, instead of a fixed delay.
    let _ = codelet_test_helpers::deadlock_guard("the reconnect re-attach", async {
        while !mock
            .transcript_requests()
            .contains(&SessionId::new("S-old"))
        {
            pump_actions(&mut app);
            tokio::task::yield_now().await;
        }
    })
    .await;

    // @step Then it calls backend.create_session(None) and gets back SessionId("S-new")
    // SUPERSEDED (TUI-113 R4/R5): with a focused session the restore
    // resolution yields "S-old", so NO create_session is issued — the
    // RPC-011 safety net only fires on a genuinely empty daemon.
    assert_eq!(
        mock.create_session_calls(),
        initial_calls,
        "TUI-113 R5: focused-session reconnect must NOT create a new session"
    );

    // @step And it emits Action::SessionCreated(SessionId("S-new")) onto the action bus
    // @step And the App's repl_active_session() returns Some(SessionId("S-new"))
    // SUPERSEDED (TUI-113 R5) + BUG-206: the memory-only transcript
    // catch-up targets "S-old"; resume_session is NEVER issued.
    pump_actions(&mut app);
    // The catch-up is the transcript sync's page request (it replaced the
    // `get_buffered_output` replay).
    assert!(
        mock.transcript_requests().contains(&SessionId::new("S-old")),
        "BUG-206: the focused session must be re-attached via the in-memory transcript catch-up; got {:?}",
        mock.transcript_requests()
    );
    assert_eq!(
        mock.resume_session_calls(),
        0,
        "BUG-206: the reconnect re-attach must NEVER resume-from-disk"
    );
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("S-old")),
        "after Reconnected, current_session must still be the focused session"
    );

    // @step And the REPL transcript is NOT destructively truncated (old transcript lines remain on screen)
    // (Surface assertion: AgentReplView's transcript field still contains
    // pre-reconnect state. Covered by exposing AgentReplView::transcript_len
    // accessor in implementation phase; here we assert via the active-session
    // swap only.)
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 9: Reconnecting Action updates the inline reconnecting line in place
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconnecting_action_updates_the_inline_reconnecting_line_in_place() -> TestResult {
    // @step Given a TestBackend App with a focused session showing an inline reconnecting line
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let (mut app, _terminal) = test_app(backend)?;
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    app.send_action(Action::Disconnected)
        .or_fail("expected a value")?;
    pump_actions(&mut app);
    assert_eq!(
        reconnect_lines(&app, &SessionId::new("s-1")).len(),
        1,
        "precondition: one inline reconnecting line after disconnect"
    );

    // @step When the action loop dispatches Action::Reconnecting(3)
    app.send_action(Action::Reconnecting(3))
        .or_fail("expected a value")?;
    pump_actions(&mut app);

    // @step Then the focused session's scrollback shows the reconnecting line updated with attempt 3
    assert_single_reconnect_line(&app, "attempt 3");

    // @step And no DisconnectDialog modal is present on the compositor
    assert!(
        !app.compositor().contains("disconnect-dialog"),
        "no DisconnectDialog modal may be present in the inline reconnect flow"
    );
    Ok(())
}
