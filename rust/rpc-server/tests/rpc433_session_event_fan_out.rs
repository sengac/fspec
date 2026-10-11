//! RPC-433 — Multi-client session lifecycle event fan-out over WebSocket.
//!
//! Feature: spec/features/session-events-daemon-fanout.feature
//!
//! This file is the SINGLE test file for the feature (ACDD 1:1 mapping).
//! It covers all five wire-side scenarios:
//!
//! - Scenario: Session created on any client is pushed to all connected
//!   clients
//! - Scenario: Session destroyed on any client is pushed to all
//!   connected clients
//! - Scenario: A connecting client is re-told about the sessions that
//!   are already open
//! - Scenario: A connecting client always receives an open-session
//!   snapshot even when no sessions are open
//! - Scenario: Client-pushed session event frames are rejected as reserved
//!
//! The first test covers BOTH created/removed fan-out scenarios (the
//! created half runs first, then the removed half — same daemon, same
//! two clients), matching the feature's two-Scenario shape.
//!
//! RED PHASE NOTE (history): `Envelope::SessionEvent`,
//! `FspecWsClient::session_events_rx` and `StubSessionManagerHandle`'s
//! created/removed broadcasts did not exist when this file was written
//! — it failed to COMPILE until RPC-433's implementation landed.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_providers::stub_provider::StubProvider;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::{bind_and_serve, ws_client_connect, Envelope};
use codelet_rpc_types::{SessionEvent, SessionId, SessionInfo};
use common::{connect_with_retry, make_workspace};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tarpc::context;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

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

/// Feature: spec/features/session-events-daemon-fanout.feature
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_multi_client_created_and_removed_fan_out() {
    // @step Given a codelet-rpc-server is serving a SharedFspecService with an attached session manager
    let (_dir, addr, _stats, _join) = bind_stub_daemon().await;

    // @step And WebSocket clients A and B are connected and subscribed to the session-event stream
    let ws_a = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client A connect");
    let ws_b = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client B connect");
    let mut rx_a = ws_a.session_events_rx();
    let mut rx_b = ws_b.session_events_rx();

    // @step When client A calls create_session
    let sid: SessionId = ws_a
        .client()
        .create_session(context::current(), None)
        .await
        .expect("client A create_session");

    // @step Then both client A and client B observe a SessionEvent::Created carrying the new session's SessionInfo on their session-event stream
    let events_a = collect_events(&mut rx_a, 1).await;
    let events_b = collect_events(&mut rx_b, 1).await;
    assert!(
        events_a
            .iter()
            .any(|e| matches!(e, SessionEvent::Created(info) if info.id == sid.value)),
        "client A must observe SessionEvent::Created for its own session (no per-client filtering); got {events_a:?}"
    );
    assert!(
        events_b
            .iter()
            .any(|e| matches!(e, SessionEvent::Created(info) if info.id == sid.value)),
        "client B must observe SessionEvent::Created for the remote session; got {events_b:?}"
    );

    // @step And WebSocket clients A and B are connected, client A holds a live session, and both are subscribed to the session-event stream
    // (already true — the created events above prove the live session + subscriptions)

    // @step When client A calls destroy_session for that session
    ws_a.client()
        .destroy_session(context::current(), sid.clone())
        .await
        .expect("client A destroy_session rpc")
        .expect("client A destroy_session result");

    // @step Then both client A and client B observe a SessionEvent::Removed carrying the session id on their session-event stream
    let removed_a = collect_events(&mut rx_a, 1).await;
    let removed_b = collect_events(&mut rx_b, 1).await;
    assert!(
        removed_a
            .iter()
            .any(|e| matches!(e, SessionEvent::Removed(id) if *id == sid)),
        "client A must observe SessionEvent::Removed for the destroyed session; got {removed_a:?}"
    );
    assert!(
        removed_b
            .iter()
            .any(|e| matches!(e, SessionEvent::Removed(id) if *id == sid)),
        "client B must observe SessionEvent::Removed for the remote destruction; got {removed_b:?}"
    );
}

/// Feature: spec/features/session-events-daemon-fanout.feature
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_connecting_client_receives_open_session_snapshot() {
    // @step Given client A has created a session on a daemon while no other client is connected
    let (_dir, addr, _stats, _join) = bind_stub_daemon().await;
    let ws_a = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client A connect");
    let sid: SessionId = ws_a
        .client()
        .create_session(context::current(), None)
        .await
        .expect("client A create_session");

    // @step And the daemon's in-memory session map holds that session
    let manager_live = ws_a
        .client()
        .list_live_sessions(context::current())
        .await
        .expect("list_live_sessions rpc");
    assert!(
        manager_live.iter().any(|s| s == &sid),
        "precondition: the seeded session must be live in the manager"
    );

    // @step When client B connects to the daemon
    let ws_b = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client B connect");

    // @step Then client B receives an open-session snapshot containing that session as part of the connection handshake, with no RPC poll from the client
    let mut snapshot_rx = ws_b.session_snapshot_rx();
    let snapshot = timeout(Duration::from_secs(3), snapshot_rx.recv())
        .await
        .expect("client B must receive an open-session snapshot during the handshake")
        .expect("snapshot broadcast must not be closed");
    assert!(
        snapshot.iter().any(|info| info.id == sid.value),
        "the connect-time snapshot must contain the pre-existing live session; got {snapshot:?}"
    );
}

/// Feature: spec/features/session-events-daemon-fanout.feature
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_connecting_client_receives_empty_snapshot_when_no_sessions() {
    // @step Given a daemon whose session manager has no live sessions
    let (_dir, addr, _stats, _join) = bind_stub_daemon().await;
    let probe = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("probe connect");
    let live = probe
        .client()
        .list_live_sessions(context::current())
        .await
        .expect("list_live_sessions rpc");
    assert!(live.is_empty(), "precondition: no live sessions");

    // @step When a client connects to the daemon
    let ws = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client connect");

    // @step Then the client receives an EMPTY open-session snapshot during the connection handshake
    let mut snapshot_rx = ws.session_snapshot_rx();
    let snapshot = timeout(Duration::from_secs(3), snapshot_rx.recv())
        .await
        .expect("even with no sessions, the handshake must deliver a (empty) snapshot")
        .expect("snapshot broadcast must not be closed");
    assert!(
        snapshot.is_empty(),
        "the snapshot must be EMPTY when no sessions are live; got {snapshot:?}"
    );
}

/// Feature: spec/features/session-events-daemon-fanout.feature
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_client_pushed_session_event_frames_rejected() {
    // @step Given a WebSocket client connected to the daemon
    let (_dir, addr, stats, _join) = bind_stub_daemon().await;
    let ws = connect_with_retry(addr.port()).await;
    let (mut sink, _stream) = ws.split();

    let base_rejected = stats.rejected_envelopes();
    let base_variants = stats.rejected_variants();

    // @step When the client sends a raw Envelope::SessionEvent frame and a raw Envelope::SessionSnapshot frame
    sink.send(Message::Binary(
        bincode::serialize(&Envelope::SessionEvent(SessionEvent::Removed(
            SessionId::new("00000000-0000-0000-0000-000000000000"),
        )))
        .expect("encode SessionEvent")
        .into(),
    ))
    .await
    .expect("send SessionEvent");

    sink.send(Message::Binary(
        bincode::serialize(&Envelope::SessionSnapshot(Vec::<SessionInfo>::new()))
            .expect("encode SessionSnapshot")
            .into(),
    ))
    .await
    .expect("send SessionSnapshot");

    // Allow the server task to process both frames.
    let mut waited = 0u64;
    while stats.rejected_envelopes() < base_rejected + 2 && waited < 2_000 {
        tokio::time::sleep(Duration::from_millis(10)).await;
        waited += 10;
    }

    // @step Then the server counts both frames in rejected_envelopes, records the variant names SessionEvent and SessionSnapshot in rejected_variants, and the connection stays healthy
    assert_eq!(
        stats.rejected_envelopes(),
        base_rejected + 2,
        "both client-pushed session frames must be counted as rejected"
    );
    let variants = stats.rejected_variants();
    assert_eq!(
        variants.len(),
        base_variants.len() + 2,
        "both rejected variant names must be recorded"
    );
    assert!(
        variants
            .windows(2)
            .any(|w| w == ["SessionEvent", "SessionSnapshot"]),
        "rejected_variants must name SessionEvent + SessionSnapshot in send order; got {variants:?}"
    );
}
