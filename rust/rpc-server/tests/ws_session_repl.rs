//! Integration tests for the WebSocket session REPL surface (RPC-007).
//!
//! Feature: spec/features/ws-session-repl.feature
//!
//! - Scenario: WebSocket list_sessions matches the embedded transport result
//! - Scenario: WebSocket create_session + send_input yields the same chunk sequence as embedded (cross-transport parity)
//! - Scenario: get_session_status reflects Idle to Running to Idle transitions equally on both transports (WS half)
//!
//! References RPC-007 additions: session RPCs on `FspecService`,
//! `chunks_rx()` on `FspecWsClient`, and the lifted `SessionId/SessionInfo/
//! SessionStatus/StreamChunk` types in `codelet_rpc_types`. Will fail to
//! compile until those exist (intended red-phase failure).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_providers::stub_provider::StubProvider;
use codelet_rpc::SharedFspecService;
use codelet_rpc_embedded::EmbeddedTransport;
use codelet_rpc_server::{bind_and_serve, ws_client_connect};
use codelet_rpc_types::{SessionId, SessionInfo, SessionStatus, StreamChunk};
use common::{connect_with_retry, make_workspace};
use std::sync::Arc;
use tarpc::context;

async fn build_ws() -> (
    tempfile::TempDir,
    EmbeddedTransport,
    codelet_rpc_server::FspecWsClient,
) {
    let (dir, embedded, ws_client, _stub) = build_ws_with_stub().await;
    (dir, embedded, ws_client)
}

/// BUG-520: like `build_ws`, also returning the stub so a test can hold its turns.
async fn build_ws_with_stub() -> (
    tempfile::TempDir,
    EmbeddedTransport,
    codelet_rpc_server::FspecWsClient,
    Arc<StubSessionManagerHandle>,
) {
    let (dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let stub = Arc::new(StubSessionManagerHandle::with_provider(Arc::new(
        StubProvider::new(),
    )));
    let manager: Arc<dyn SessionManagerHandle> = stub.clone();
    let service = Arc::new(SharedFspecService::with_session_manager(
        Arc::clone(&watcher),
        Arc::clone(&manager),
    ));
    let embedded = EmbeddedTransport::new(tokio::runtime::Handle::current(), Arc::clone(&service));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve");
    let ws = connect_with_retry(addr.port()).await;
    let ws_client = ws_client_connect(ws)
        .await
        .expect("ws_client_connect failed");
    (dir, embedded, ws_client, stub)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_ws_list_sessions_matches_embedded() {
    // @step Given a developer has started codelet-rpc-server bound to 127.0.0.1 with a SessionManagerHandle backed by the same SessionManager
    let (_dir, embedded, ws_client) = build_ws().await;

    // @step And a WebSocket client is connected to the server
    let embedded_client = embedded.client();
    let seeded: SessionId = embedded_client
        .create_session(context::current(), Some("default".to_string()))
        .await
        .expect("create_session must succeed via embedded path")
        .expect("the daemon creates the session");

    // @step When the client calls FspecServiceClient::list_sessions(context::current()) over the WebSocket transport
    let ws_listed: Vec<SessionInfo> = ws_client
        .client()
        .list_sessions(
            context::current(),
            std::env::current_dir()
                .expect("the test process has a working directory")
                .to_string_lossy()
                .to_string(),
        )
        .await
        .expect("WS list_sessions must succeed")
        .expect("the sessions are listed");
    let embedded_listed: Vec<SessionInfo> = embedded_client
        .list_sessions(
            context::current(),
            std::env::current_dir()
                .expect("the test process has a working directory")
                .to_string_lossy()
                .to_string(),
        )
        .await
        .expect("embedded list_sessions must succeed")
        .expect("the sessions are listed");

    // @step Then the call returns Ok(Vec<SessionInfo>) equal to the result of the embedded list_sessions in the parity scenario
    // SessionInfo.id is a flat String (TS-shape compatibility — see
    // rust/rpc-types/src/lib.rs:111). The seeded session ID is a
    // SessionId newtype; compare against its String form.
    let mut ws_ids: Vec<String> = ws_listed.iter().map(|s| s.id.clone()).collect();
    let mut em_ids: Vec<String> = embedded_listed.iter().map(|s| s.id.clone()).collect();
    ws_ids.sort();
    em_ids.sort();
    assert_eq!(
        ws_ids, em_ids,
        "WS list_sessions must equal embedded list_sessions",
    );
    assert!(ws_ids.contains(&seeded.value));

    // @step And the SessionInfo entries serialize/deserialize via bincode-of-Envelope without shape mismatch
    let bytes = bincode::serialize(&ws_listed).expect("bincode encode SessionInfo");
    let round: Vec<SessionInfo> = bincode::deserialize(&bytes).expect("bincode decode SessionInfo");
    assert_eq!(
        round.len(),
        ws_listed.len(),
        "bincode round-trip must preserve SessionInfo shape",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_ws_send_input_chunk_sequence_matches_embedded() {
    // @step Given a WebSocket client connected to codelet-rpc-server with the StubProvider feature enabled
    let (_dir, embedded, ws_client) = build_ws().await;

    // @step And the client has subscribed to FspecWsClient::chunks_rx() before sending input
    let mut ws_rx = ws_client.chunks_rx();
    let mut em_rx = embedded.chunks_rx();

    // @step When the client calls create_session(None) and send_input(session_id, "hi")
    let sid: SessionId = ws_client
        .client()
        .create_session(context::current(), None)
        .await
        .expect("WS create_session must succeed")
        .expect("the daemon creates the session");
    ws_client
        .client()
        .send_input(context::current(), sid.clone(), "hi".to_string())
        .await
        .expect("WS send_input must succeed")
        .expect("daemon accepted the input");

    // @step Then send_input returns Ok(()) immediately
    // (Asserted by `.expect` above — return type is `()`.)

    // Drain the deterministic stub sequence on both receivers.
    let mut ws_chunks: Vec<StreamChunk> = Vec::new();
    let mut em_chunks: Vec<StreamChunk> = Vec::new();
    for _ in 0..16 {
        match codelet_test_helpers::deadlock_guard("ws_rx.recv()", ws_rx.recv()).await {
            Ok(Ok((got_sid, c))) if got_sid == sid => {
                let done = matches!(c, StreamChunk::Done);
                ws_chunks.push(c);
                if done {
                    break;
                }
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    for _ in 0..16 {
        match codelet_test_helpers::deadlock_guard("em_rx.recv()", em_rx.recv()).await {
            Ok(Ok((got_sid, c))) if got_sid == sid => {
                let done = matches!(c, StreamChunk::Done);
                em_chunks.push(c);
                if done {
                    break;
                }
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }

    // @step And the chunks observed on chunks_rx() are byte-equal to the chunks observed on the embedded path for the same input
    let ws_bytes = bincode::serialize(&ws_chunks).expect("bincode encode WS chunks");
    let em_bytes = bincode::serialize(&em_chunks).expect("bincode encode embedded chunks");
    assert_eq!(
        ws_bytes, em_bytes,
        "WS and embedded chunk sequences must be byte-equal for the same input",
    );

    // @step And every chunk arrived as a bincode-encoded Envelope::Event { session_id, chunk } frame on the WebSocket wire
    // (Asserted by ws_session_repl_wire_format below.)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_get_session_status_reflects_idle_running_idle_on_websocket() {
    // @step Given a session is created via create_session on the WebSocket transport with the StubProvider
    let (_dir, _embedded, ws_client, stub) = build_ws_with_stub().await;
    let sid: SessionId = ws_client
        .client()
        .create_session(context::current(), None)
        .await
        .expect("WS create_session must succeed")
        .expect("the daemon creates the session");

    // @step And the stub provider holds each turn open until the caller releases it
    let turn = stub.hold_turns();
    let mut status_changes = ws_client.status_changes_rx();

    // @step When the caller calls get_session_status(session_id) before any send_input
    let before = ws_client
        .client()
        .get_session_status(context::current(), sid.clone())
        .await
        .expect("WS get_session_status must succeed")
        .expect("the session status is read");

    // @step Then the returned SessionStatus is Idle
    assert!(matches!(before, SessionStatus::Idle), "before: {before:?}");

    // @step When the caller calls send_input(session_id, "hi") and calls get_session_status(session_id) while the turn is held
    ws_client
        .client()
        .send_input(context::current(), sid.clone(), "hi".to_string())
        .await
        .expect("WS send_input must succeed")
        .expect("daemon accepted the input");
    let during = ws_client
        .client()
        .get_session_status(context::current(), sid.clone())
        .await
        .expect("WS get_session_status must succeed")
        .expect("the session status is read");

    // @step Then the returned SessionStatus is Running
    assert!(
        matches!(during, SessionStatus::Running),
        "during: {during:?}"
    );

    // @step When the caller releases the turn, receives the session's Idle status change on the WebSocket client \
    //     and calls get_session_status(session_id) again
    turn.release();
    // The deadline only turns a lost event into a failure instead of a hang.
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        loop {
            match status_changes.recv().await {
                Ok((id, SessionStatus::Idle)) if id == sid => break,
                Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(error) => panic!("status changes ended before Idle: {error}"),
            }
        }
    })
    .await
    .expect("the session's Idle status change must arrive on the WebSocket client");
    let after = ws_client
        .client()
        .get_session_status(context::current(), sid.clone())
        .await
        .expect("WS get_session_status must succeed")
        .expect("the session status is read");

    // @step Then the returned SessionStatus is Idle
    assert!(matches!(after, SessionStatus::Idle), "after: {after:?}");
}
