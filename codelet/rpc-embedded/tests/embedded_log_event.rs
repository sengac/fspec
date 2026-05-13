//! Integration test for the embedded LogEvent push channel (RPC-007).
//!
//! Feature: spec/features/embedded-log-event.feature
//!
//! - Scenario: Tracing emissions are observable as LogEvent on both transports
//!
//! References the new `EmbeddedTransport::logs_rx()` method and the
//! `LogRecord` type that RPC-007 lifts into `codelet_rpc_types`. Will fail to
//! compile until those symbols exist (intended red-phase failure).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::session_manager_handle::{
    SessionManagerHandle, StubSessionManagerHandle,
};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_providers::stub_provider::StubProvider;
use codelet_rpc::SharedFspecService;
use codelet_rpc_embedded::EmbeddedTransport;
use codelet_rpc_types::LogRecord;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;
use tracing::Level;

fn build_transport_with_layer() -> (TempDir, EmbeddedTransport) {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path();
    std::fs::create_dir_all(workspace.join("spec")).expect("mkdir spec");
    std::fs::write(
        workspace.join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("seed work-units.json");
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let manager: Arc<dyn SessionManagerHandle> = Arc::new(
        StubSessionManagerHandle::with_provider(Arc::new(StubProvider::new())),
    );
    let service = Arc::new(SharedFspecService::with_session_manager(
        Arc::clone(&watcher),
        Arc::clone(&manager),
    ));
    // RPC-007: EmbeddedTransport::with_log_layer registers the broadcast
    // tracing layer for the embedded host (sibling of rpc-server's main.rs
    // registration). Without it logs_rx must still exist but observe nothing.
    let transport = EmbeddedTransport::with_log_layer(
        tokio::runtime::Handle::current(),
        service,
    );
    (dir, transport)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_tracing_emit_is_observable_on_embedded_logs_rx() {
    // @step Given codelet-rpc-server has registered the LogRecord tracing::Layer at startup
    // (Asserted by codelet/rpc-server/tests/ws_log_event.rs for the WS half.)

    // @step And an EmbeddedTransport host has registered the same Layer at EmbeddedTransport::with_log_layer
    let (_dir, transport) = build_transport_with_layer();

    // @step And a WebSocket client is connected and subscribed to FspecWsClient::logs_rx()
    // (Asserted by codelet/rpc-server/tests/ws_log_event.rs.)

    // @step And an embedded caller is subscribed to EmbeddedTransport::logs_rx()
    let mut rx = transport.logs_rx();

    // @step When the host emits tracing::info!("hello")
    tracing::info!("hello");

    // @step Then the WebSocket client receives an Envelope::LogEvent(LogRecord) frame with message "hello" and level INFO
    // (Asserted on the WS side.)

    // @step And the embedded caller receives a LogRecord on logs_rx() with the same message and level
    let record: LogRecord = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("logs_rx must yield within 2s")
        .expect("broadcast not closed");
    assert_eq!(
        record.message, "hello",
        "LogRecord.message must match emitted tracing message",
    );
    assert_eq!(
        record.level,
        Level::INFO.as_str(),
        "LogRecord.level must be INFO for tracing::info!",
    );
}
