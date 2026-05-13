//! WebSocket WorkUnitsUpdate push integration tests (RPC-006).
//!
//! Feature: spec/features/websocket-work-units-push.feature
//!
//! Covers the WS-side fan-out behaviour:
//!   - Scenario: WebSocket client receives an initial WorkUnitsUpdate frame on connection
//!   - Scenario: WebSocket client receives a WorkUnitsUpdate frame on file mutation
//!   - Scenario: WorkUnitsUpdate frames are encoded with bincode and not JSON
//!
//! On connect, the per-connection fan-out task publishes an immediate
//! `Envelope::WorkUnitsUpdate(snapshot)` frame BEFORE any workspace
//! mutation. Subsequent file mutations produce one `WorkUnitsUpdate`
//! frame per debounced event, bincode-encoded over the existing envelope
//! pump. The client's `FspecWsClient::work_units_rx()` broadcast receiver
//! observes the same payloads.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_rpc_server::{ws_client_connect, Envelope};
use codelet_rpc_types::WorkUnitInfo;
use common::{connect_with_retry, make_workspace, spawn_rpc_server_with_workspace, write_workspace};
use futures::StreamExt;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_websocket_client_receives_an_initial_work_units_update_frame_on_connection() {
    // @step Given the rpc-server binary spawned bound to 127.0.0.1:0 over a temporary workspace whose spec/work-units.json declares two work units, with its ephemeral port read from stdout
    let (dir, _path) = make_workspace(&[
        ("AUTH-001", "Login", "done"),
        ("AUTH-002", "Reset password", "implementing"),
    ]);
    let (_guard, port) = spawn_rpc_server_with_workspace(dir.path());

    // @step When a WebSocket client connects to that port and reads exactly one frame from the inbound channel before any file mutation
    let ws = connect_with_retry(port).await;
    let (_sink, mut stream) = ws.split();
    let msg = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("server must publish initial frame within 2s")
        .expect("ws stream not closed")
        .expect("ws frame ok");
    let bytes = match msg {
        Message::Binary(b) => b.to_vec(),
        other => panic!("expected binary frame, got {other:?}"),
    };

    // @step Then the frame decodes with bincode into Envelope::WorkUnitsUpdate(Vec<WorkUnitInfo>) carrying the two work units from the workspace
    let env: Envelope = bincode::deserialize(&bytes).expect("bincode-decode envelope");
    let payload = match env {
        Envelope::WorkUnitsUpdate(payload) => payload,
        other => panic!(
            "initial frame must be WorkUnitsUpdate(Vec<WorkUnitInfo>), got {:?}",
            other.variant_name()
        ),
    };
    let mut ids: Vec<String> = payload.into_iter().map(|wu| wu.id).collect();
    ids.sort();
    assert_eq!(
        ids,
        vec!["AUTH-001".to_string(), "AUTH-002".to_string()],
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_websocket_client_receives_a_work_units_update_frame_on_file_mutation() {
    // @step Given the rpc-server binary spawned bound to 127.0.0.1:0 over a temporary workspace and a connected WebSocket client whose initial snapshot frame has been consumed
    let (dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let (_guard, port) = spawn_rpc_server_with_workspace(dir.path());

    let ws = connect_with_retry(port).await;
    let client = ws_client_connect(ws)
        .await
        .expect("ws_client_connect failed");
    let mut rx = client.work_units_rx();
    // Drain the initial snapshot frame so the next recv() reflects the mutation.
    let _initial: Vec<WorkUnitInfo> = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("initial frame must arrive within 2s")
        .expect("broadcast not closed");

    // @step When I append a third work unit to spec/work-units.json and the client waits up to one second on FspecWsClient::work_units_rx()
    write_workspace(
        &path,
        &[
            ("AUTH-001", "Login", "done"),
            ("AUTH-002", "Reset password", "implementing"),
            ("AUTH-003", "Two factor auth", "specifying"),
        ],
    );
    let received: Vec<WorkUnitInfo> = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("client must observe push within 2s")
        .expect("broadcast not closed");

    // @step Then the receiver yields a Vec<WorkUnitInfo> containing all three work units and the corresponding inbound frame on the wire decoded with bincode as Envelope::WorkUnitsUpdate carrying the same payload
    let mut ids: Vec<String> = received.into_iter().map(|wu| wu.id).collect();
    ids.sort();
    assert_eq!(
        ids,
        vec![
            "AUTH-001".to_string(),
            "AUTH-002".to_string(),
            "AUTH-003".to_string(),
        ],
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_work_units_update_frames_are_bincode_encoded_and_not_json() {
    // @step Given a connected WebSocket client and a workspace mutation that triggers exactly one push frame
    let (dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let (_guard, port) = spawn_rpc_server_with_workspace(dir.path());

    let ws = connect_with_retry(port).await;
    let (_sink, mut stream) = ws.split();
    // Drain the initial snapshot so the next frame on the wire is the
    // mutation push, in isolation.
    let _initial = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("initial frame within 2s")
        .expect("ws not closed")
        .expect("ws frame ok");

    write_workspace(
        &path,
        &[
            ("AUTH-001", "Login", "done"),
            ("AUTH-002", "Reset password", "implementing"),
        ],
    );

    // @step When the client captures the raw bytes of the inbound frame
    let msg = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("push frame within 2s")
        .expect("ws not closed")
        .expect("ws frame ok");
    let bytes = match msg {
        Message::Binary(b) => b.to_vec(),
        other => panic!("expected binary frame, got {other:?}"),
    };

    // @step Then the captured bytes successfully decode with bincode into Envelope::WorkUnitsUpdate(Vec<WorkUnitInfo>) and the captured bytes are not valid UTF-8 JSON
    let env: Envelope = bincode::deserialize(&bytes).expect("bincode-decode envelope");
    assert!(
        matches!(env, Envelope::WorkUnitsUpdate(_)),
        "push frame must decode as Envelope::WorkUnitsUpdate, got {:?}",
        env.variant_name()
    );
    if let Ok(s) = std::str::from_utf8(&bytes) {
        let json: Result<serde_json::Value, _> = serde_json::from_str(s);
        assert!(
            json.is_err(),
            "WorkUnitsUpdate frame must not be valid UTF-8 JSON, but parsed: {s}"
        );
    }
}
