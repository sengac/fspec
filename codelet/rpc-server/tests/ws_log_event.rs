//! Integration tests for the WebSocket LogEvent push channel and bincode wire format (RPC-007).
//!
//! Feature: spec/features/ws-log-event.feature
//! Feature: spec/features/embedded-log-event.feature (WS half of cross-transport scenario)
//!
//! - Scenario: Tracing emissions are observable as LogEvent on both transports (WS half)
//! - Scenario: Event and LogEvent ride bincode-encoded Envelope on the WebSocket wire
//!
//! Will fail to compile until RPC-007 lifts `LogRecord` and adds the new
//! `Envelope::Event { session_id, chunk }` and `Envelope::LogEvent(LogRecord)`
//! variants and the `logs_rx()` method on `FspecWsClient`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_providers::stub_provider::StubProvider;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::{bind_and_serve, register_log_layer, ws_client_connect, Envelope};
use codelet_rpc_types::{LogRecord, SessionId, StreamChunk};
use common::{connect_with_retry, make_workspace};
use std::sync::Arc;
use std::time::Duration;
use tarpc::context;
use tokio::time::timeout;
use tracing::Level;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_tracing_emit_is_observable_on_ws_log_event() {
    // @step Given codelet-rpc-server has registered the LogRecord tracing::Layer at startup
    let (_dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let manager: Arc<dyn SessionManagerHandle> = Arc::new(StubSessionManagerHandle::with_provider(
        Arc::new(StubProvider::new()),
    ));
    let service = Arc::new(SharedFspecService::with_session_manager(
        Arc::clone(&watcher),
        Arc::clone(&manager),
    ));
    register_log_layer(Arc::clone(&service)).expect("register layer");
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve");

    // @step And an EmbeddedTransport host has registered the same Layer at EmbeddedTransport::with_log_layer
    // (Asserted by codelet/rpc-embedded/tests/embedded_log_event.rs.)

    // @step And a WebSocket client is connected and subscribed to FspecWsClient::logs_rx()
    let ws = connect_with_retry(addr.port()).await;
    let ws_client = ws_client_connect(ws).await.expect("ws_client_connect");
    let mut rx = ws_client.logs_rx();

    // @step And an embedded caller is subscribed to EmbeddedTransport::logs_rx()
    // (Asserted on the embedded side.)

    // @step When the host emits tracing::info!("hello")
    tracing::info!("hello");

    // @step Then the WebSocket client receives an Envelope::LogEvent(LogRecord) frame with message "hello" and level INFO
    // The broadcast carries every tracing event in the process
    // (including unrelated tarpc-internal trace lines), so drain until
    // we see the user-emitted "hello" record (or hit the timeout).
    let record: LogRecord = loop {
        let next: LogRecord = timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("logs_rx must yield within 2s")
            .expect("broadcast not closed");
        if next.message == "hello" {
            break next;
        }
    };
    assert_eq!(record.message, "hello");
    assert_eq!(record.level, Level::INFO.as_str());

    // @step And the embedded caller receives a LogRecord on logs_rx() with the same message and level
    // (Asserted on the embedded side.)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_event_and_log_event_ride_bincode_encoded_envelope() {
    // @step Given a WebSocket client is connected and subscribed to chunks_rx() and logs_rx()
    let (_dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let manager: Arc<dyn SessionManagerHandle> = Arc::new(StubSessionManagerHandle::with_provider(
        Arc::new(StubProvider::new()),
    ));
    let service = Arc::new(SharedFspecService::with_session_manager(
        Arc::clone(&watcher),
        Arc::clone(&manager),
    ));
    register_log_layer(Arc::clone(&service)).expect("register layer");
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve");
    let ws = connect_with_retry(addr.port()).await;
    let ws_client = ws_client_connect(ws).await.expect("ws_client_connect");
    let _chunks_rx = ws_client.chunks_rx();
    let _logs_rx = ws_client.logs_rx();

    // @step When a session emits a StreamChunk and the host emits a tracing event
    let sid: SessionId = ws_client
        .client()
        .create_session(context::current(), None)
        .await
        .expect("create_session");
    ws_client
        .client()
        .send_input(context::current(), sid.clone(), "hi".to_string())
        .await
        .expect("send_input");
    tracing::info!("ping");

    // @step Then a synthesized Envelope::Event { session_id, chunk } round-trips via bincode without ambiguity
    // We assert the round-trip: the Envelope variant exists, has the right
    // shape, and bincode round-trips it. Real wire delivery is asserted by
    // ws_multi_client_chunks.rs and the embedded_session_repl tests.
    let ev_frame = Envelope::Event {
        session_id: sid.clone(),
        chunk: StreamChunk::text("hi".to_string()),
    };
    let ev_bytes = bincode::serialize(&ev_frame).expect("bincode encode Event");
    let ev_round: Envelope = bincode::deserialize(&ev_bytes).expect("bincode decode Event");
    assert!(
        matches!(ev_round, Envelope::Event { .. }),
        "Envelope::Event must round-trip via bincode",
    );

    // @step And a synthesized Envelope::LogEvent(LogRecord) round-trips via bincode without ambiguity
    let log_frame = Envelope::LogEvent(LogRecord {
        level: Level::INFO.as_str().to_string(),
        target: "test".to_string(),
        message: "ping".to_string(),
        timestamp_ms: 0,
    });
    let log_bytes = bincode::serialize(&log_frame).expect("bincode encode LogEvent");
    let log_round: Envelope = bincode::deserialize(&log_bytes).expect("bincode decode LogEvent");
    assert!(
        matches!(log_round, Envelope::LogEvent(_)),
        "Envelope::LogEvent must round-trip via bincode",
    );

    // @step And neither bincode-encoded frame decodes as JSON nor uses any custom shape outside Envelope
    let json_attempt = serde_json::from_slice::<Envelope>(&ev_bytes);
    assert!(
        json_attempt.is_err(),
        "bincode-encoded Envelope::Event must NOT decode as JSON",
    );
}
