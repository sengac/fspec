//! Integration tests for the WebSocket LogEvent push channel and bincode wire format (RPC-007).
//!
//! Feature: spec/features/ws-log-event.feature
//! Feature: spec/features/embedded-log-event.feature (WS half of cross-transport scenario)
//!
//! - Scenario: Tracing emissions are observable as LogEvent on both transports (WS half)
//! - Scenario: Event and LogEvent ride bincode-encoded Envelope on the WebSocket wire
//! - Scenario: Forwarding log records over the WebSocket does not feed the log broadcast (BUG-572)
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
use tarpc::context;
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
    // (Asserted by rust/rpc-embedded/tests/embedded_log_event.rs.)

    // @step And a WebSocket client is connected and subscribed to FspecWsClient::logs_rx()
    let ws = connect_with_retry(addr.port()).await;
    let ws_client = ws_client_connect(ws).await.expect("ws_client_connect");
    let mut rx = ws_client.logs_rx();

    // @step And an embedded caller is subscribed to EmbeddedTransport::logs_rx()
    // (Asserted on the embedded side.)

    // @step When the host emits tracing::info!("hello")
    // @step Then the WebSocket client receives an Envelope::LogEvent(LogRecord) frame with message "hello" and level INFO
    // The broadcast carries every tracing event in the process (including
    // unrelated tarpc-internal trace lines), so drain until the
    // user-emitted "hello" record arrives. TEST-042: the server's log
    // fan-out for this connection may subscribe after the first emit (a
    // record sent before then reaches no one), so "hello" is emitted again
    // after every drained batch until one arrives, under the deadlock guard.
    let record: LogRecord =
        codelet_test_helpers::deadlock_guard("the \"hello\" log record", async {
            loop {
                tracing::info!("hello");
                for _ in 0..64 {
                    match rx.try_recv() {
                        Ok(next) if next.message == "hello" => return next,
                        Ok(_) | Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {}
                        Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                            tokio::task::yield_now().await;
                        }
                        Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                            panic!("broadcast closed before the hello record")
                        }
                    }
                }
            }
        })
        .await
        .expect("the hello record arrives");
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
        .expect("create_session")
        .expect("the daemon creates the session");
    ws_client
        .client()
        .send_input(context::current(), sid.clone(), "hi".to_string())
        .await
        .expect("send_input")
        .expect("daemon accepted the input");
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

/// BUG-572: the crates the log broadcast is carried by. A record from one of
/// them below WARN reaching the client means forwarding fed the broadcast.
fn is_transport_record_below_warn(record: &LogRecord) -> bool {
    let transport = ["tungstenite", "tokio_tungstenite", "tarpc"]
        .iter()
        .any(|krate| record.target == *krate || record.target.starts_with(&format!("{krate}::")));
    let below_warn = !matches!(record.level.as_str(), "WARN" | "ERROR");
    transport && below_warn
}

/// Receive the next record, failing on a transport record below WARN or on
/// a lag (a lag here can only come from a flood).
async fn next_checked(rx: &mut tokio::sync::broadcast::Receiver<LogRecord>) -> LogRecord {
    use tokio::sync::broadcast::error::RecvError;
    match rx.recv().await {
        Ok(record) if is_transport_record_below_warn(&record) => {
            panic!("forwarding fed the log broadcast with a transport record: {record:?}")
        }
        Ok(record) => record,
        Err(RecvError::Lagged(skipped)) => {
            panic!("the client's logs channel lagged by {skipped} records: the broadcast flooded")
        }
        Err(RecvError::Closed) => panic!("the logs channel closed"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_forwarding_log_records_does_not_feed_the_log_broadcast() {
    // @step Given a WebSocket client is connected and subscribed to logs_rx()
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
    let mut rx = ws_client.logs_rx();

    // The connection's log fan-out subscribes before the connection serves
    // RPCs, so after one round-trip every record emitted reaches the client.
    ws_client
        .client()
        .health(context::current())
        .await
        .expect("health round-trip");

    // @step When the host emits a "hello" record and, once the client has received it, a "marker" record
    // Writing the "hello" frame makes tungstenite log it. If the transport's
    // logging were broadcast, those records would reach the client before
    // "marker", which is emitted only after "hello" arrived.
    // @step Then no record the client receives before "marker" has a tungstenite, tokio_tungstenite or tarpc target below WARN
    codelet_test_helpers::deadlock_guard("the hello and marker log records", async {
        tracing::info!("bug572 hello");
        while next_checked(&mut rx).await.message != "bug572 hello" {}
        tracing::info!("bug572 marker");
        while next_checked(&mut rx).await.message != "bug572 marker" {}
    })
    .await
    .expect("the marker record arrives with no transport record before it");
}
