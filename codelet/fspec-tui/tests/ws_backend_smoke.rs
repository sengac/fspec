//! WebSocket backend smoke + source-shape tests (RPC-008).
//!
//! Feature: spec/features/fspec-tui-ws-backend.feature
//!
//! Scenarios covered:
//!   - "WebSocketFspecBackend smoke test round-trips list_work_units across
//!     the WS wire" — boots a real `bind_and_serve` rpc-server on
//!     127.0.0.1:0, builds a `WebSocketFspecBackend::connect(ws_url)`,
//!     observes the same Vec<WorkUnitInfo> the embedded backend would
//!     return (cross-transport parity), then subscribes via
//!     `work_units_rx()` and observes the initial WorkUnitsUpdate snapshot
//!     frame from RPC-006 within 5 seconds.
//!   - "WebSocketFspecBackend.connect uses tokio_tungstenite::connect_async
//!     directly" — source-shape assertion against
//!     `codelet/fspec-tui/src/transport/websocket.rs` confirming the
//!     architecture rule [27] / scenario constraints (no helper in
//!     codelet-rpc-server, no envelope/bincode/framing code lives in
//!     codelet/fspec-tui/src/transport/).
//!
//! Both scenarios are co-located here because they cover the same
//! production file (`transport/websocket.rs`) — keeping them in one test
//! file makes a future regression diff easy to spot.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc_types::WorkUnitInfo;

mod common;

/// Smoke test: WebSocketFspecBackend + cross-transport parity with the
/// embedded backend (architecture rule [4]: zero-cost passthrough on
/// subscription, one-line delegate on RPCs).
#[tokio::test]
async fn websocket_backend_round_trips_list_work_units_across_the_ws_wire() {
    // @step Given a `bind_and_serve` rpc-server running on 127.0.0.1:0 with a tempdir-backed WorkUnitsWatcher
    let (_dir, service) = common::temp_service();
    let (addr, _join) = common::start_ws_server(std::sync::Arc::clone(&service)).await;
    let ws_url = common::ws_url(addr);

    // @step And a WebSocketFspecBackend constructed via `WebSocketFspecBackend::connect(ws_url).await?`
    let ws_backend = WebSocketFspecBackend::connect(ws_url)
        .await
        .expect("WebSocketFspecBackend::connect");

    // @step When the test calls `backend.list_work_units().await`
    let actual_via_ws: Vec<WorkUnitInfo> = ws_backend
        .list_work_units()
        .await
        .expect("list_work_units over WS");

    // @step Then the returned Vec<WorkUnitInfo> equals what an EmbeddedFspecBackend wrapping the same service would have returned
    let embedded_backend = EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        std::sync::Arc::clone(&service),
    );
    let actual_via_embedded: Vec<WorkUnitInfo> = embedded_backend
        .list_work_units()
        .await
        .expect("list_work_units over embedded transport");

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
    let snapshot: Vec<WorkUnitInfo> = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("WS work_units_rx::recv timed out after 5s")
        .expect("WS broadcast channel closed");
    let mut snapshot_ids: Vec<String> = snapshot.iter().map(|w| w.id.clone()).collect();
    snapshot_ids.sort();
    assert_eq!(
        snapshot_ids,
        vec!["AUTH-001".to_string(), "AUTH-002".to_string()],
        "initial WS WorkUnitsUpdate frame must mirror the seeded fixture"
    );
}

/// Source-shape assertion (no I/O): inspect the body of
/// `codelet/fspec-tui/src/transport/websocket.rs` to confirm the rule
/// [27] / scenario "WebSocketFspecBackend.connect uses
/// tokio_tungstenite::connect_async directly".
#[test]
fn websocket_backend_connect_uses_tokio_tungstenite_connect_async_directly() {
    // @step Given codelet/fspec-tui/src/transport/websocket.rs exists
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/transport/websocket.rs");
    assert!(
        manifest_dir.exists(),
        "expected fspec-tui WebSocketFspecBackend source at {}",
        manifest_dir.display()
    );

    // @step When I inspect the body of `WebSocketFspecBackend::connect`
    let src = fs::read_to_string(&manifest_dir).expect("read websocket.rs");

    // @step Then it calls `tokio_tungstenite::connect_async(url)` directly
    assert!(
        src.contains("tokio_tungstenite::connect_async("),
        "websocket.rs must call tokio_tungstenite::connect_async directly"
    );

    // @step And it hands the resulting WebSocketStream to `codelet_rpc_server::ws_client_connect()`
    assert!(
        src.contains("ws_client_connect("),
        "websocket.rs must invoke codelet_rpc_server::ws_client_connect on the WS stream"
    );

    // @step And it stores the resulting FspecWsClient on the struct
    //
    // RPC-008 originally required a literal `client: FspecWsClient`
    // field. RPC-011 rule [18] / architecture note [0] wraps it in
    // `Arc<RwLock<Option<FspecWsClient>>>` so the reconnect supervisor
    // can atomically swap the client out on disconnect / reconnect.
    // The SEMANTIC requirement — the resulting FspecWsClient is stored
    // on the struct — is preserved either way; we accept any field
    // whose name is `client` and whose type mentions `FspecWsClient`.
    assert!(
        src.contains("client: FspecWsClient")
            || src.contains("FspecWsClient {")
            || (src.contains("client:") && src.contains("FspecWsClient")),
        "websocket.rs must store the resulting FspecWsClient on the WebSocketFspecBackend struct (expected `client: FspecWsClient`, `FspecWsClient {{`, or a `client:` field whose type mentions `FspecWsClient` — the RPC-011 supervisor wraps it in Arc<RwLock<Option<FspecWsClient>>>)"
    );

    // @step And no envelope, bincode, or framing code lives in codelet/fspec-tui/src/transport/
    let transport_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/transport");
    let mut violations: Vec<String> = Vec::new();
    for entry in fs::read_dir(&transport_dir).expect("read transport/") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let body = fs::read_to_string(&path).expect("read transport rs file");
        let stripped = strip_rust_comments(&body);
        for needle in [
            "Envelope::Rpc",
            "Envelope::WorkUnitsUpdate",
            "Envelope::Event",
            "Envelope::LogEvent",
            "bincode::serialize",
            "bincode::deserialize",
        ] {
            if stripped.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "no envelope/bincode/framing code may live under codelet/fspec-tui/src/transport/. \
         Violations: {violations:?}"
    );
}

/// Strip both `//` line comments and `/* … */` block comments from a
/// Rust source body. Mirrors the helper used by
/// `codelet/rpc-embedded/tests/source_helpers/mod.rs::strip_rust_comments`
/// — duplicated locally because that helper is not yet exported and the
/// dev-dependency policy keeps this crate's test tree self-contained.
fn strip_rust_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => {
                for nc in chars.by_ref() {
                    if nc == '\n' {
                        out.push(nc);
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                while let Some(nc) = chars.next() {
                    if nc == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        break;
                    }
                }
            }
            other => out.push(other),
        }
    }
    out
}
