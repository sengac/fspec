//! Multi-client unfiltered chunk fan-out (RPC-007).
//!
//! Feature: spec/features/session-rpcs-streamchunk-logevent-push-channels-repl-backend.feature
//!
//! - Scenario: Multi-client unfiltered fan-out delivers every session's chunks to every connected client

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_providers::stub_provider::StubProvider;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::{bind_and_serve, ws_client_connect};
use codelet_rpc_types::{SessionId, StreamChunk};
use common::{connect_with_retry, make_workspace};
use std::sync::Arc;
use std::time::Duration;
use tarpc::context;
use tokio::time::timeout;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_multi_client_unfiltered_chunk_fan_out() {
    // @step Given two WebSocket clients A and B are connected to the same codelet-rpc-server
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
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve");

    let ws_a = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client A connect");
    let ws_b = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("client B connect");

    // @step And both clients have subscribed to FspecWsClient::chunks_rx()
    let mut rx_a = ws_a.chunks_rx();
    let mut rx_b = ws_b.chunks_rx();

    // @step When client A calls create_session(None) and send_input(session_id, "hi")
    let sid: SessionId = ws_a
        .client()
        .create_session(context::current(), None)
        .await
        .expect("client A create_session");
    ws_a.client()
        .send_input(context::current(), sid.clone(), "hi".to_string())
        .await
        .expect("client A send_input");

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

    // @step Then client A observes the StreamChunks for that session on chunks_rx()
    assert!(
        !chunks_a.is_empty(),
        "client A must observe at least one chunk for the active session",
    );

    // @step And client B observes the same StreamChunks for that session on chunks_rx()
    assert_eq!(
        bincode::serialize(&chunks_a).expect("encode A"),
        bincode::serialize(&chunks_b).expect("encode B"),
        "clients A and B must observe byte-equal chunk sequences (unfiltered fan-out)",
    );

    // @step And the server applies no per-client subscription filter in this card
    // (Implied by byte-equal chunk sequences: no filter could produce equal
    // outputs unless every client gets every chunk.)
}
