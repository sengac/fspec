//! WebSocket backend smoke + source-shape tests (RPC-008).
//!
//! Feature: spec/features/fspec-tui-ws-backend.feature
//!
//! Scenario covered: "WebSocketFspecBackend smoke test round-trips
//! list_work_units across the WS wire" — boots a real `bind_and_serve`
//! rpc-server on 127.0.0.1:0, builds a `WebSocketFspecBackend::connect(ws_url)`,
//! observes the same Vec<WorkUnitInfo> the embedded backend would return
//! (cross-transport parity), then subscribes via `work_units_rx()` and
//! observes the initial WorkUnitsUpdate snapshot frame from RPC-006 within 5
//! seconds. (BUG-667 retired the source-shape scenario that pinned
//! `tokio_tungstenite::connect_async`; every connect goes through
//! `codelet_rpc_server::ws_connect`.)

use codelet_test_helpers::{OrFail, TestResult};

use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc_types::WorkUnitInfo;

mod common;

/// Smoke test: WebSocketFspecBackend + cross-transport parity with the
/// embedded backend (architecture rule [4]: zero-cost passthrough on
/// subscription, one-line delegate on RPCs).
#[tokio::test]
async fn websocket_backend_round_trips_list_work_units_across_the_ws_wire() -> TestResult {
    // @step Given a `bind_and_serve` rpc-server running on 127.0.0.1:0 with a tempdir-backed WorkUnitsWatcher
    let (_dir, service) = common::temp_service()?;
    let (addr, _join) = common::start_ws_server(std::sync::Arc::clone(&service)).await?;
    let ws_url = common::ws_url(addr)?;

    // @step And a WebSocketFspecBackend constructed via `WebSocketFspecBackend::connect(ws_url).await?`
    let ws_backend = WebSocketFspecBackend::connect(ws_url)
        .await
        .or_fail("WebSocketFspecBackend::connect")?;

    // @step When the test calls `backend.list_work_units().await`
    let actual_via_ws: Vec<WorkUnitInfo> = ws_backend
        .list_work_units()
        .await
        .or_fail("list_work_units over WS")?;

    // @step Then the returned Vec<WorkUnitInfo> equals what an EmbeddedFspecBackend wrapping the same service would have returned
    let embedded_backend = EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        std::sync::Arc::clone(&service),
    );
    let actual_via_embedded: Vec<WorkUnitInfo> = embedded_backend
        .list_work_units()
        .await
        .or_fail("list_work_units over embedded transport")?;

    let mut ws_ids: Vec<String> = actual_via_ws.iter().map(|w| w.id.clone()).collect();
    let mut embedded_ids: Vec<String> = actual_via_embedded.iter().map(|w| w.id.clone()).collect();
    ws_ids.sort();
    embedded_ids.sort();
    assert_eq!(
        ws_ids, embedded_ids,
        "cross-transport parity: WS list_work_units must match embedded list_work_units"
    );
    assert_eq!(
        ws_ids,
        vec!["AUTH-001".to_string(), "AUTH-002".to_string()],
        "list_work_units must round-trip the seeded fixture exactly"
    );

    // @step When the test subscribes via `backend.work_units_rx()`
    let mut rx = ws_backend.work_units_rx();

    // @step Then the initial WorkUnitsUpdate snapshot frame from RPC-006 is observed within 5 seconds
    let snapshot: Vec<WorkUnitInfo> =
        codelet_test_helpers::deadlock_guard("the work-units push", rx.recv())
            .await
            .or_fail("the deadlock guard expired")?
            .or_fail("WS broadcast channel closed")?;
    let mut snapshot_ids: Vec<String> = snapshot.iter().map(|w| w.id.clone()).collect();
    snapshot_ids.sort();
    assert_eq!(
        snapshot_ids,
        vec!["AUTH-001".to_string(), "AUTH-002".to_string()],
        "initial WS WorkUnitsUpdate frame must mirror the seeded fixture"
    );
    Ok(())
}
