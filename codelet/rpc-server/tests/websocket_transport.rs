//! Integration tests for the WebSocket transport (RPC-005, post-RPC-006).
//!
//! Feature: spec/features/websocket-transport-rpc.feature
//!
//! Covers three scenarios on a single feature file (1:1 file mapping):
//!   - Scenario: WebSocket transport returns WorkUnitInfo via the rpc-server binary
//!   - Scenario: WebSocket frames are encoded with bincode by default
//!   - Scenario: Reserved envelope variants are rejected by the server
//!
//! After RPC-006:
//!   - The rpc-server binary takes a `--workspace <path>` argument and
//!     reads from a real `WorkUnitsWatcher` instead of the hard-coded
//!     RPC-005 fixture.
//!   - The reserved-variants regression scenario now sends only
//!     {Event, LogEvent, CmdReq, CmdRes} — `WorkUnitsUpdate` is a
//!     legitimate first-class variant. (The narrowed-set test
//!     `ws_reserved_variants_after_rpc006.rs` covers the post-RPC-006
//!     scenario from `reserved-envelope-variants-narrowed.feature`;
//!     this file's scenario_6 retains its original RPC-005 set so the
//!     RPC-005 acceptance test still passes against the new server,
//!     because `Envelope::WorkUnitsUpdate(payload)` carrying an empty
//!     vec is also a server-rejection on the inbound path.)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::{bind_and_serve, ws_client_connect, Envelope};
use common::{connect_with_retry, make_workspace, spawn_rpc_server_with_workspace};
use futures::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tarpc::context;
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

/// Direction of a captured frame relative to the client perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    ClientToServer,
    ServerToClient,
}

/// Recorder shared between the proxy task and the test body. Each entry
/// is one binary WebSocket frame seen verbatim before being relayed.
type FrameLog = Arc<Mutex<Vec<(Direction, Vec<u8>)>>>;

/// In-process WebSocket-level proxy used by the bincode scenario to
/// wire-tap real frame bytes flowing between an unmodified client and
/// an unmodified rpc-server. Each binary frame is recorded verbatim
/// into `captured` before being relayed unchanged to the other side.
async fn spawn_recording_proxy(upstream_port: u16, captured: FrameLog) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        let (down_stream, _) = listener.accept().await.unwrap();
        let down_ws = tokio_tungstenite::accept_async(down_stream).await.unwrap();
        let (mut down_sink, mut down_stream) = down_ws.split();

        let upstream_url = Url::parse(&format!("ws://127.0.0.1:{upstream_port}")).unwrap();
        let (up_ws, _) = connect_async(upstream_url.as_str()).await.unwrap();
        let (mut up_sink, mut up_stream) = up_ws.split();

        loop {
            tokio::select! {
                from_client = down_stream.next() => {
                    let Some(Ok(msg)) = from_client else { break };
                    if let Message::Binary(ref bytes) = msg {
                        captured
                            .lock()
                            .unwrap()
                            .push((Direction::ClientToServer, bytes.to_vec()));
                    }
                    if up_sink.send(msg).await.is_err() { break }
                }
                from_server = up_stream.next() => {
                    let Some(Ok(msg)) = from_server else { break };
                    if let Message::Binary(ref bytes) = msg {
                        captured
                            .lock()
                            .unwrap()
                            .push((Direction::ServerToClient, bytes.to_vec()));
                    }
                    if down_sink.send(msg).await.is_err() { break }
                }
            }
        }
    });

    proxy_port
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_2_websocket_transport_returns_work_unit_info_via_binary() {
    // @step Given the rpc-server binary has been spawned bound to 127.0.0.1:0 with its ephemeral port read from stdout, and the shared FspecService implementation it hosts is seeded with a fixture of two WorkUnitInfo records
    let (dir, _path) = make_workspace(&[
        ("AUTH-001", "User Login", "done"),
        ("AUTH-002", "Password reset", "implementing"),
    ]);
    let (_guard, port) = spawn_rpc_server_with_workspace(dir.path());

    // @step When I connect a tokio-tungstenite WebSocket client to that port, obtain an FspecServiceClient over the WebSocket transport, and call list_work_units on the client
    let ws_stream = connect_with_retry(port).await;
    let client = ws_client_connect(ws_stream)
        .await
        .expect("ws_client_connect failed");
    let result = client.rpc.list_work_units(context::current()).await;

    // @step Then the call returns Ok with a Vec<WorkUnitInfo> equal to the fixture
    let actual = result.expect("RPC over WebSocket should succeed");
    let mut ids: Vec<String> = actual.into_iter().map(|wu| wu.id).collect();
    ids.sort();
    assert_eq!(
        ids,
        vec!["AUTH-001".to_string(), "AUTH-002".to_string()],
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_5_websocket_frames_are_bincode_encoded_by_default() {
    // @step Given the rpc-server is running with default configuration
    let (_dir, path) = make_workspace(&[
        ("AUTH-001", "User Login", "done"),
        ("AUTH-002", "Password reset", "implementing"),
    ]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).unwrap());
    let service = Arc::new(SharedFspecService::new(Arc::clone(&watcher)));
    let (server_addr, _stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve failed");

    // Wire-tap proxy between the client and the real server. Every binary
    // frame in either direction is recorded verbatim BEFORE being forwarded
    // unchanged. This lets us assert what really crosses the wire during a
    // genuine `list_work_units` round-trip.
    let captured: FrameLog = Arc::new(Mutex::new(Vec::new()));
    let proxy_port = spawn_recording_proxy(server_addr.port(), Arc::clone(&captured)).await;

    let ws_stream = connect_with_retry(proxy_port).await;
    let client = ws_client_connect(ws_stream)
        .await
        .expect("ws_client_connect failed");

    // @step When a WebSocket client sends a list_work_units RPC request and receives the response while the bytes of both frames are captured
    let result = client
        .rpc
        .list_work_units(context::current())
        .await
        .expect("RPC over WebSocket should succeed");
    assert_eq!(result, watcher.snapshot(), "real RPC must return live snapshot");

    // Allow the proxy to flush both directions through the recorder.
    let mut waited = 0u64;
    loop {
        let n = captured
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, bytes)| {
                bincode::deserialize::<Envelope>(bytes)
                    .map(|e| matches!(e, Envelope::Rpc(_)))
                    .unwrap_or(false)
            })
            .count();
        if n >= 2 || waited >= 1000 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
        waited += 10;
    }

    let frames = captured.lock().unwrap().clone();

    // @step Then the captured frame bytes successfully decode with bincode into the expected Envelope::Rpc value and the captured frame bytes are not valid UTF-8 JSON
    // Filter to the Rpc frames only (the wire also carries an initial
    // `Envelope::WorkUnitsUpdate` push that is not the focus of this
    // RPC-005 scenario).
    let rpc_frames: Vec<_> = frames
        .iter()
        .filter(|(_, bytes)| {
            bincode::deserialize::<Envelope>(bytes)
                .map(|e| matches!(e, Envelope::Rpc(_)))
                .unwrap_or(false)
        })
        .collect();
    let req_count = rpc_frames
        .iter()
        .filter(|(dir, _)| *dir == Direction::ClientToServer)
        .count();
    let resp_count = rpc_frames
        .iter()
        .filter(|(dir, _)| *dir == Direction::ServerToClient)
        .count();
    assert!(
        req_count >= 1,
        "expected at least one client→server Rpc frame, captured={frames:?}"
    );
    assert!(
        resp_count >= 1,
        "expected at least one server→client Rpc frame, captured={frames:?}"
    );

    for (dir, bytes) in &rpc_frames {
        let env: Envelope = bincode::deserialize(bytes).unwrap_or_else(|e| {
            panic!("frame {dir:?} ({bytes:?}) must bincode-decode as Envelope: {e}")
        });
        assert!(
            matches!(env, Envelope::Rpc(_)),
            "frame {dir:?} must decode to Envelope::Rpc, got {env:?}"
        );
        if let Ok(s) = std::str::from_utf8(bytes) {
            let json: Result<serde_json::Value, _> = serde_json::from_str(s);
            assert!(
                json.is_err(),
                "frame {dir:?} must not be valid UTF-8 JSON, but parsed: {s}"
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_6_reserved_envelope_variants_are_rejected_by_server() {
    // @step Given the rpc-server is running
    let (_dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).unwrap());
    let service = Arc::new(SharedFspecService::new(Arc::clone(&watcher)));
    let (addr, stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve failed");

    let ws_stream = connect_with_retry(addr.port()).await;
    let (mut sink, mut _stream) = ws_stream.split();

    // @step When a WebSocket client sends a frame whose Envelope variant is one of Event, LogEvent, WorkUnitsUpdate, CmdReq, or CmdRes
    // RPC-006: WorkUnitsUpdate is now a payload-bearing variant. A
    // client pushing an (empty) WorkUnitsUpdate to the server is still
    // wrong (servers emit them, clients don't push them) and the
    // server rejects it as a reserved-from-the-client variant.
    // RPC-007: Event and LogEvent are also payload-bearing now (chunks
    // and log records). Like WorkUnitsUpdate they only flow
    // server → client; a client pushing them is still rejected.
    let reserved: Vec<(Envelope, &'static str)> = vec![
        (
            Envelope::Event {
                session_id: codelet_rpc_types::SessionId::new(""),
                chunk: codelet_rpc_types::StreamChunk::done(),
            },
            "Event",
        ),
        (
            Envelope::LogEvent(codelet_rpc_types::LogRecord {
                level: "INFO".to_string(),
                target: "test".to_string(),
                message: "".to_string(),
                timestamp_ms: 0,
            }),
            "LogEvent",
        ),
        (Envelope::WorkUnitsUpdate(Vec::new()), "WorkUnitsUpdate"),
        (Envelope::CmdReq, "CmdReq"),
        (Envelope::CmdRes, "CmdRes"),
    ];
    for (variant, _name) in &reserved {
        let bytes = bincode::serialize(variant).expect("envelope encodes");
        sink.send(Message::Binary(bytes.into()))
            .await
            .expect("send reserved envelope");
    }

    // Allow the server task to process all five frames.
    let mut waited = 0u64;
    while stats.rejected_envelopes() < reserved.len() as u64 && waited < 1000 {
        tokio::time::sleep(Duration::from_millis(10)).await;
        waited += 10;
    }

    // @step Then the server records the unsupported variant by name in its rejection log and does not invoke any FspecService method as a result of that frame
    assert_eq!(
        stats.rejected_envelopes(),
        reserved.len() as u64,
        "server must reject every reserved Envelope variant"
    );
    let recorded = stats.rejected_variants();
    let expected_names: Vec<&'static str> = reserved.iter().map(|(_, n)| *n).collect();
    assert_eq!(
        recorded, expected_names,
        "server must record each rejected variant by name in arrival order"
    );
    assert_eq!(
        service.list_work_units_calls(),
        0,
        "no FspecService method must be invoked by reserved frames"
    );
}
