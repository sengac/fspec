//! Cross-transport parity tests (RPC-005, post-RPC-006 watcher lift).
//!
//! Feature: spec/features/dual-transport-parity.feature
//!
//! Covers two scenarios on a single feature file (1:1 file mapping):
//!   - Scenario: Both transports produce semantically identical results for the same call
//!   - Scenario: Both transport calls reach the same shared service implementation
//!
//! After RPC-006 the "fixture" is materialised on disk in a shared temp
//! workspace observed by a real `WorkUnitsWatcher` — both transports
//! still reach the same SINGLE `SharedFspecService` instance.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_embedded::EmbeddedTransport;
use codelet_rpc_server::{bind_and_serve, ws_client_connect};
use common::{connect_with_retry, make_workspace};
use std::sync::Arc;
use tarpc::context;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_3_both_transports_produce_semantically_identical_results() {
    // @step Given I have an embedded FspecServiceClient and a WebSocket FspecServiceClient connected to the same shared service impl seeded with a fixture of two WorkUnitInfo records
    let (_dir, path) = make_workspace(&[
        ("AUTH-001", "User Login", "done"),
        ("AUTH-002", "Password reset", "implementing"),
    ]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).unwrap());
    let service = Arc::new(SharedFspecService::new(Arc::clone(&watcher)));

    let embedded = EmbeddedTransport::new(tokio::runtime::Handle::current(), Arc::clone(&service));
    let embedded_client = embedded.client();

    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve failed");
    let ws_stream = connect_with_retry(addr.port()).await;
    let ws_client = ws_client_connect(ws_stream)
        .await
        .expect("ws_client_connect failed");

    // @step When I call list_work_units through both clients
    let embedded_result = embedded_client
        .list_work_units(context::current())
        .await
        .expect("embedded RPC must succeed");
    let ws_result = ws_client
        .rpc
        .list_work_units(context::current())
        .await
        .expect("websocket RPC must succeed");

    // @step Then both calls return Ok and the two returned Vec<WorkUnitInfo> values are equal under PartialEq
    assert_eq!(
        embedded_result, ws_result,
        "cross-transport parity: results must be equal under PartialEq"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_4_both_transport_calls_reach_the_same_shared_service_implementation() {
    // @step Given the shared FspecService implementation increments a list_work_units invocation counter on every call, and I have an embedded FspecServiceClient and a WebSocket FspecServiceClient connected to that single shared impl
    let (_dir, path) = make_workspace(&[
        ("AUTH-001", "User Login", "done"),
        ("AUTH-002", "Password reset", "implementing"),
    ]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).unwrap());
    let service = Arc::new(SharedFspecService::new(Arc::clone(&watcher)));
    assert_eq!(service.list_work_units_calls(), 0, "counter starts at zero");

    let embedded = EmbeddedTransport::new(tokio::runtime::Handle::current(), Arc::clone(&service));
    let embedded_client = embedded.client();

    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve failed");
    let ws_stream = connect_with_retry(addr.port()).await;
    let ws_client = ws_client_connect(ws_stream)
        .await
        .expect("ws_client_connect failed");

    // @step When I call list_work_units once on the embedded client and once on the WebSocket client
    embedded_client
        .list_work_units(context::current())
        .await
        .expect("embedded RPC must succeed");
    ws_client
        .rpc
        .list_work_units(context::current())
        .await
        .expect("websocket RPC must succeed");

    // @step Then the shared invocation counter has been incremented exactly twice
    assert_eq!(
        service.list_work_units_calls(),
        2,
        "shared service impl must be reached by both transports"
    );
}
