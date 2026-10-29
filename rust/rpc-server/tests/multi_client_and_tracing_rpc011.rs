//! Multi-client + broadcast capacity + tracing client_id tests — RPC-011.
//!
//! Feature: spec/features/multi-client-and-tracing.feature
//!
//! Covers:
//!   - Two clients attached simultaneously see the same chunk stream
//!   - Broadcast capacities are explicit and tuned (chunks=1024, logs=4096, work_units=256)
//!   - Tracing spans carry client_id on per-connection handler tasks
//!
//! Red phase: requires `DEFAULT_WORK_UNITS_CAPACITY` const, retuned chunks
//! (1024) and logs (4096) capacities, `#[tracing::instrument(fields(client_id = %peer))]`
//! on handle_connection, and a `ServerStats::connected_clients()` accessor.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_providers::stub_provider::StubProvider;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::{bind_and_serve, ws_client_connect};
use codelet_rpc_types::{SessionId, StreamChunk};
use common::{connect_with_retry, make_workspace};
use tarpc::context;
use tokio::time::timeout;

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Two clients attached simultaneously see the same chunk stream
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_clients_attached_simultaneously_see_the_same_chunk_stream() {
    // @step Given a daemon listening on 127.0.0.1:0
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
    let (addr, stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve");

    // @step When the test opens two WebSocketFspecBackends (WS-A and WS-B) against that daemon
    let ws_a = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client A");
    let ws_b = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client B");
    let mut rx_a = ws_a.chunks_rx();
    let mut rx_b = ws_b.chunks_rx();

    // Allow the daemon a tick to register both connections.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // @step And ServerStats.connected_clients reads 2 throughout the test
    assert_eq!(
        stats.connected_clients(),
        2,
        "two connected clients must be counted"
    );

    // @step And WS-A calls create_session(None) returning session_id S
    let sid: SessionId = ws_a
        .client()
        .create_session(context::current(), None)
        .await
        .expect("create_session");

    // @step And WS-A calls send_input(S, "hi")
    ws_a.client()
        .send_input(context::current(), sid.clone(), "hi".to_string())
        .await
        .expect("send_input");

    async fn drain_until_done(
        rx: &mut tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
        sid: &SessionId,
    ) -> Vec<StreamChunk> {
        let mut out = Vec::new();
        for _ in 0..32 {
            match timeout(Duration::from_secs(2), rx.recv()).await {
                Ok(Ok((got_sid, c))) if got_sid == *sid => {
                    let done = matches!(c, StreamChunk::Done);
                    out.push(c);
                    if done {
                        break;
                    }
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        out
    }

    let chunks_a = drain_until_done(&mut rx_a, &sid).await;
    let chunks_b = drain_until_done(&mut rx_b, &sid).await;

    // @step Then both WS-A.chunks_rx() and WS-B.chunks_rx() yield the SAME sequence of (S, chunk) frames in the SAME order
    assert!(
        !chunks_a.is_empty(),
        "client A must observe at least one chunk"
    );
    assert_eq!(
        bincode::serialize(&chunks_a).expect("encode A"),
        bincode::serialize(&chunks_b).expect("encode B"),
        "clients A and B must observe byte-equal chunk sequences"
    );

    // @step And no chunk is delivered to one client and not the other
    assert_eq!(
        chunks_a.len(),
        chunks_b.len(),
        "chunk counts must match between A and B"
    );

    // connected_clients still reads 2 (no client has disconnected).
    assert_eq!(stats.connected_clients(), 2);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Broadcast capacities are explicit and tuned
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn broadcast_capacities_are_explicit_and_tuned() {
    // @step Given rust/rpc/src/lib.rs
    let path = workspace_root().join("rpc").join("src").join("lib.rs");
    let body = codelet_test_helpers::source_composition::read_source(&path)
        .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));

    // @step When inspecting the broadcast capacity constants
    // @step Then DEFAULT_CHUNKS_CAPACITY equals 1024
    assert!(
        body.contains("DEFAULT_CHUNKS_CAPACITY: usize = 1024"),
        "DEFAULT_CHUNKS_CAPACITY must equal 1024 in rust/rpc/src/lib.rs"
    );

    // @step And DEFAULT_LOGS_CAPACITY equals 4096
    assert!(
        body.contains("DEFAULT_LOGS_CAPACITY: usize = 4096"),
        "DEFAULT_LOGS_CAPACITY must equal 4096 in rust/rpc/src/lib.rs"
    );

    // @step And DEFAULT_WORK_UNITS_CAPACITY equals 256
    assert!(
        body.contains("DEFAULT_WORK_UNITS_CAPACITY: usize = 256"),
        "DEFAULT_WORK_UNITS_CAPACITY must equal 256 in rust/rpc/src/lib.rs"
    );

    // @step And the constants are used as the third argument of broadcast::channel(...) in SharedFspecService::new and SharedFspecService::with_session_manager
    assert!(
        body.contains("broadcast::channel(DEFAULT_CHUNKS_CAPACITY)"),
        "broadcast::channel(DEFAULT_CHUNKS_CAPACITY) must be used in SharedFspecService"
    );
    assert!(
        body.contains("broadcast::channel(DEFAULT_LOGS_CAPACITY)"),
        "broadcast::channel(DEFAULT_LOGS_CAPACITY) must be used in SharedFspecService"
    );
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("rpc-server must have a parent")
        .to_path_buf()
}

include!("multi_client_and_tracing_rpc011/tracing.rs");
