//! Integration tests for the embedded session REPL push channel (RPC-007).
//!
//! Feature: spec/features/embedded-session-repl.feature
//!
//! - Scenario: Embedded list_sessions returns the same Vec<SessionInfo> as the underlying SessionManager
//! - Scenario: Embedded create_session + send_input emits at least one StreamChunk on chunks_rx within 5s
//! - Scenario: interrupt(session_id) flips state and emits StreamChunk::Interrupted on chunks_rx
//! - Scenario: get_session_status reflects Idle to Running to Idle transitions equally on both transports
//!
//! These tests reference types (`SessionId`, `SessionInfo`, `SessionStatus`,
//! `StreamChunk`) that RPC-007 will lift into `codelet_rpc_types`, the new
//! `SessionManagerHandle` trait that RPC-007 will introduce in `codelet_core`,
//! and the new `chunks_rx()` method on `EmbeddedTransport` that RPC-007 will
//! add. They will fail to compile until the implementing phase wires up those
//! symbols — that is the intended red-phase failure mode under ACDD.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_providers::stub_provider::StubProvider;
use codelet_rpc::SharedFspecService;
use codelet_rpc_embedded::EmbeddedTransport;
use codelet_rpc_types::{SessionId, SessionInfo, SessionStatus, StreamChunk};
use std::sync::Arc;
use std::time::Duration;
use tarpc::context;
use tempfile::TempDir;
use tokio::time::{sleep, timeout};

fn build_transport() -> (TempDir, EmbeddedTransport) {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path();
    std::fs::create_dir_all(workspace.join("spec")).expect("mkdir spec");
    std::fs::write(
        workspace.join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("seed work-units.json");

    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let stub = Arc::new(StubProvider::new());
    let manager: Arc<dyn SessionManagerHandle> =
        Arc::new(StubSessionManagerHandle::with_provider(stub));
    let service = Arc::new(SharedFspecService::with_session_manager(
        Arc::clone(&watcher),
        Arc::clone(&manager),
    ));
    let transport = EmbeddedTransport::new(tokio::runtime::Handle::current(), service);
    (dir, transport)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_embedded_list_sessions_matches_underlying_session_manager() {
    // @step Given a host has constructed an EmbeddedTransport from a tokio Handle around a SessionManagerHandle backed by the existing SessionManager singleton
    let (_dir, transport) = build_transport();
    let client = transport.client();

    // @step And the SessionManager already holds at least one active session known to NAPI list_sessions
    let seeded: SessionId = client
        .create_session(context::current(), Some("default".to_string()))
        .await
        .expect("create_session must succeed");

    // @step When a Rust caller invokes FspecServiceClient::list_sessions(context::current()) over the embedded transport
    let listed: Vec<SessionInfo> = client
        .list_sessions(context::current())
        .await
        .expect("list_sessions must succeed");

    // @step Then the call returns Ok(Vec<SessionInfo>) with the same length and SessionId values that NAPI list_sessions would return
    let ids: Vec<SessionId> = listed
        .iter()
        .map(|s| SessionId::new(s.id.clone()))
        .collect();
    assert!(
        ids.contains(&seeded),
        "list_sessions must include the seeded session id {seeded:?}, got {ids:?}",
    );

    // @step And the call does not encode any Envelope frames
    // The "no envelope encoding on the embedded path" half of the Then is
    // enforced by the source-shape regression test in
    // rpc_006_source_shape.rs::scenario_embedded_push_path_has_no_bincode_serialize
    // (widened by RPC-007 to cover the chunks/logs paths as well).
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_embedded_send_input_emits_at_least_one_streamchunk_within_5s() {
    // @step Given an EmbeddedTransport with a SessionManagerHandle wired to the StubProvider behind the test-support feature
    let (_dir, transport) = build_transport();
    let client = transport.client();

    // @step And the caller has subscribed to EmbeddedTransport::chunks_rx() before sending input
    let mut rx = transport.chunks_rx();

    // @step When the caller calls create_session(role: None) and then send_input(session_id, "hi")
    let sid: SessionId = client
        .create_session(context::current(), None)
        .await
        .expect("create_session must succeed");
    client
        .send_input(context::current(), sid.clone(), "hi".to_string())
        .await
        .expect("send_input must succeed");

    // @step Then send_input returns Ok(()) immediately without holding a tarpc stream
    // (Asserted implicitly above — `send_input` returns `()` per the trait
    // definition; if it ever returned a stream the call site would not type-check.)

    // @step And within 5 seconds the chunks_rx receiver yields at least one (SessionId, StreamChunk::Text { .. }) tuple matching the active session
    let (got_sid, chunk) = timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("chunks_rx must yield within 5s")
        .expect("broadcast not closed");
    assert_eq!(
        got_sid, sid,
        "chunk session_id must match the active session"
    );
    assert!(
        matches!(chunk, StreamChunk::Text { .. }),
        "first chunk must be StreamChunk::Text, got {chunk:?}",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_interrupt_flips_state_and_emits_interrupted_chunk() {
    // @step Given a session is actively streaming a stub-provider response on either transport
    let (_dir, transport) = build_transport();
    let client = transport.client();
    let mut rx = transport.chunks_rx();
    let sid: SessionId = client
        .create_session(context::current(), None)
        .await
        .expect("create_session must succeed");
    client
        .send_input(context::current(), sid.clone(), "hi".to_string())
        .await
        .expect("send_input must succeed");

    // @step And the caller is subscribed to chunks_rx() for that session
    // (`rx` above)

    // @step When the caller calls interrupt(session_id)
    sleep(Duration::from_millis(10)).await;
    client
        .interrupt(context::current(), sid.clone())
        .await
        .expect("interrupt must succeed");

    // @step Then the RPC returns Ok(()) immediately
    // (Asserted by `.expect` above — interrupt returns ().)

    // @step And the chunks_rx receiver yields a StreamChunk::Interrupted (or equivalent) for that session
    let mut saw_interrupted = false;
    for _ in 0..32 {
        match timeout(Duration::from_secs(1), rx.recv()).await {
            Ok(Ok((got_sid, StreamChunk::Interrupted { .. }))) if got_sid == sid => {
                saw_interrupted = true;
                break;
            }
            Ok(Ok(_)) => continue,
            Ok(Err(_)) => break,
            Err(_) => break,
        }
    }
    assert!(
        saw_interrupted,
        "chunks_rx must yield StreamChunk::Interrupted for the interrupted session",
    );

    // @step And a subsequent get_session_status(session_id) reports the session as interrupted
    let status: SessionStatus = client
        .get_session_status(context::current(), sid.clone())
        .await
        .expect("get_session_status must succeed");
    assert!(
        matches!(status, SessionStatus::Interrupted),
        "post-interrupt status must be SessionStatus::Interrupted, got {status:?}",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_get_session_status_reflects_idle_running_idle_on_embedded() {
    // @step Given a session is created via create_session on either transport with the StubProvider
    let (_dir, transport) = build_transport();
    let client = transport.client();
    let sid: SessionId = client
        .create_session(context::current(), None)
        .await
        .expect("create_session must succeed");

    // @step When the caller calls get_session_status(session_id) before any send_input
    let before = client
        .get_session_status(context::current(), sid.clone())
        .await
        .expect("get_session_status must succeed");

    // @step Then the returned SessionStatus is Idle
    assert!(
        matches!(before, SessionStatus::Idle),
        "pre-input status must be SessionStatus::Idle, got {before:?}",
    );

    // @step When the caller calls send_input(session_id, "hi") and immediately calls get_session_status(session_id)
    client
        .send_input(context::current(), sid.clone(), "hi".to_string())
        .await
        .expect("send_input must succeed");
    let during = client
        .get_session_status(context::current(), sid.clone())
        .await
        .expect("get_session_status must succeed");

    // @step Then the returned SessionStatus is Running
    assert!(
        matches!(during, SessionStatus::Running),
        "during-stream status must be SessionStatus::Running, got {during:?}",
    );

    // @step When the stub provider has emitted StreamChunk::Done and the caller calls get_session_status(session_id) again
    // Drain the broadcast until Done arrives so the SessionManager has marked
    // the session Idle again.
    let mut rx = transport.chunks_rx();
    for _ in 0..64 {
        match timeout(Duration::from_secs(1), rx.recv()).await {
            Ok(Ok((_, StreamChunk::Done))) => break,
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    sleep(Duration::from_millis(20)).await;
    let after = client
        .get_session_status(context::current(), sid.clone())
        .await
        .expect("get_session_status must succeed");

    // @step Then the returned SessionStatus is Idle
    assert!(
        matches!(after, SessionStatus::Idle),
        "post-stream status must return to SessionStatus::Idle, got {after:?}",
    );

    // @step And the same sequence holds when the parity scenario is run on the other transport
    // The WebSocket parity is asserted in
    // codelet/rpc-server/tests/ws_session_repl.rs::scenario_get_session_status_reflects_idle_running_idle_on_websocket.
}
