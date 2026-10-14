//! RPC-436 — Multi-client session-cleared event fan-out over WebSocket.
//!
//! Feature: spec/features/session-events-cleared-fan-out.feature
//!
//! This file is the SINGLE test file for the feature (ACDD 1:1 mapping).
//! It covers the wire-side scenario:
//!
//! - Scenario: Session cleared on any client is pushed to all connected
//!   clients
//!
//! The wire-side half of RPC-436: `clear_history` must fire
//! `Envelope::SessionEvent(SessionEvent::Cleared(id))` to EVERY connected
//! client (no per-client filtering). The manager-level "unknown id fires
//! no event" liveness gate lives in the sibling feature
//! session-events-daemon-cleared-broadcast (one test file per feature,
//! ACDD 1:1 mapping — mirrors the RPC-433 split between
//! session-events-daemon-fanout and
//! session-events-daemon-removed-broadcast).
//!
//! RED PHASE NOTE: `SessionEvent::Cleared` did not exist when this file
//! was written — it fails to COMPILE until the variant lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_providers::stub_provider::StubProvider;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::{bind_and_serve, ws_client_connect};
use codelet_rpc_types::SessionEvent;
use common::{connect_with_retry, make_workspace};
use tokio::time::timeout;

/// Bind a daemon with the stub session manager on the CURRENT tokio
/// runtime (the test's) and return (tempdir, addr, stats, join). The
/// caller keeps `tempdir` alive for the whole test.
async fn bind_stub_daemon() -> (
    tempfile::TempDir,
    std::net::SocketAddr,
    codelet_rpc_server::ServerStats,
    tokio::task::JoinHandle<()>,
) {
    let (dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let manager: Arc<dyn SessionManagerHandle> = Arc::new(StubSessionManagerHandle::with_provider(
        Arc::new(StubProvider::new()),
    ));
    let service = Arc::new(SharedFspecService::with_session_manager(
        Arc::clone(&watcher),
        Arc::clone(&manager),
    ));
    let (addr, stats, join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    (dir, addr, stats, join)
}

/// Collect up to `count` events from the session-event stream (3s cap each).
async fn collect_events(
    rx: &mut tokio::sync::broadcast::Receiver<SessionEvent>,
    count: usize,
) -> Vec<SessionEvent> {
    let mut out = Vec::new();
    while out.len() < count {
        match timeout(Duration::from_secs(3), rx.recv()).await {
            Ok(Ok(ev)) => out.push(ev),
            _ => break,
        }
    }
    out
}

// =============================================================================
// Scenario: Session cleared on any client is pushed to all connected clients
// =============================================================================

/// Feature: spec/features/session-events-cleared-fan-out.feature
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_multi_client_cleared_fan_out() {
    // @step Given a codelet-rpc-server is serving a SharedFspecService with an attached session manager
    let (_dir, addr, _stats, _join) = bind_stub_daemon().await;

    // @step And WebSocket clients A and B are connected, client A holds a live session, and both are subscribed to the session-event stream
    let ws_a = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client A connect");
    let ws_b = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client B connect");
    let mut rx_a = ws_a.session_events_rx();
    let mut rx_b = ws_b.session_events_rx();

    let sid = ws_a
        .client()
        .create_session(tarpc::context::current(), None)
        .await
        .expect("client A create_session");

    // @step When client A calls clear_history for that session
    ws_a.client()
        .clear_history(tarpc::context::current(), sid.clone())
        .await
        .expect("client A clear_history rpc")
        .expect("client A clear_history result");

    // @step Then both client A and client B observe a SessionEvent::Cleared carrying the session id on their session-event stream
    // (Each client also observed the Created event for the same session —
    // the fan-out has no per-client filtering — so allow up to 2 events.)
    let events_a = collect_events(&mut rx_a, 2).await;
    let events_b = collect_events(&mut rx_b, 2).await;
    assert!(
        events_a
            .iter()
            .any(|e| matches!(e, SessionEvent::Cleared(id) if *id == sid)),
        "client A must observe SessionEvent::Cleared for its own session (no per-client filtering); got {events_a:?}"
    );
    assert!(
        events_b
            .iter()
            .any(|e| matches!(e, SessionEvent::Cleared(id) if *id == sid)),
        "client B must observe SessionEvent::Cleared for the remote session; got {events_b:?}"
    );
}
