//! Cross-transport push parity (RPC-006).
//!
//! Feature: spec/features/cross-transport-work-units-parity.feature
//!
//! - Scenario: Both transports produce byte-identical WorkUnitsUpdate payloads for the same mutation
//!
//! The same workspace is observed by ONE shared `WorkUnitsWatcher` exposed
//! through both an embedded transport AND a WS-served `FspecWsClient`. A
//! single mutation must produce equal `Vec<WorkUnitInfo>` on both
//! receivers, and re-encoding each via `Envelope::WorkUnitsUpdate` must
//! yield identical bincode bytes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_embedded::EmbeddedTransport;
use codelet_rpc_server::{bind_and_serve, ws_client_connect, Envelope};
use codelet_rpc_types::WorkUnitInfo;
use common::{connect_with_retry, make_workspace, write_workspace};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_both_transports_produce_byte_identical_work_units_update_payloads() {
    // @step Given the same temporary workspace observed by one shared WorkUnitsWatcher exposed through both an EmbeddedTransport and an rpc-server-backed WebSocket client
    let (_dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let service = Arc::new(SharedFspecService::new(Arc::clone(&watcher)));

    let embedded = EmbeddedTransport::new(tokio::runtime::Handle::current(), Arc::clone(&service));
    let mut embedded_rx = embedded.work_units_rx();

    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve");
    let ws = connect_with_retry(addr.port()).await;
    let ws_client = ws_client_connect(ws)
        .await
        .expect("ws_client_connect failed");
    let mut ws_rx = ws_client.work_units_rx();
    // Drain the initial WS snapshot so the next recv reflects the mutation.
    let _initial = timeout(Duration::from_secs(2), ws_rx.recv())
        .await
        .expect("initial WS frame within 2s")
        .expect("broadcast not closed");

    // @step When I mutate spec/work-units.json once and collect the resulting Vec<WorkUnitInfo> from EmbeddedTransport::work_units_rx() and from FspecWsClient::work_units_rx()
    write_workspace(
        &path,
        &[
            ("AUTH-001", "Login", "done"),
            ("AUTH-002", "Reset password", "implementing"),
        ],
    );
    let embedded_payload: Vec<WorkUnitInfo> = timeout(Duration::from_secs(2), embedded_rx.recv())
        .await
        .expect("embedded must observe within 2s")
        .expect("embedded broadcast not closed");
    let ws_payload: Vec<WorkUnitInfo> = timeout(Duration::from_secs(2), ws_rx.recv())
        .await
        .expect("ws must observe within 2s")
        .expect("ws broadcast not closed");

    // @step Then the two Vec<WorkUnitInfo> values are equal under PartialEq and the bincode encoding of each via Envelope::WorkUnitsUpdate produces identical byte sequences
    assert_eq!(
        embedded_payload, ws_payload,
        "cross-transport push: payload values must be equal under PartialEq"
    );
    let embedded_bytes =
        bincode::serialize(&Envelope::WorkUnitsUpdate(embedded_payload.clone())).unwrap();
    let ws_bytes = bincode::serialize(&Envelope::WorkUnitsUpdate(ws_payload.clone())).unwrap();
    assert_eq!(
        embedded_bytes, ws_bytes,
        "cross-transport push: bincode-of-Envelope must be byte-identical"
    );
}
