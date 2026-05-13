//! Cross-transport chunk parity (RPC-007).
//!
//! Feature: spec/features/session-rpcs-streamchunk-logevent-push-channels-repl-backend.feature
//!
//! - Scenario: NAPI co-listener parity (Rust half) — a Rust embedded subscriber
//!   created against the SAME SessionManager singleton observes byte-equal
//!   StreamChunks as the embedded chunks_rx
//!
//! References RPC-007 additions: `chunks_rx()` on both `EmbeddedTransport` and
//! `FspecWsClient`, the lifted `StreamChunk`/`SessionId`, and the new
//! `SessionManagerHandle` trait. Will fail to compile until those exist.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_core::session_manager_handle::{
    SessionManagerHandle, StubSessionManagerHandle,
};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_providers::stub_provider::StubProvider;
use codelet_rpc::SharedFspecService;
use codelet_rpc_embedded::EmbeddedTransport;
use codelet_rpc_server::{bind_and_serve, ws_client_connect};
use codelet_rpc_types::{SessionId, StreamChunk};
use common::{connect_with_retry, make_workspace};
use std::sync::Arc;
use std::time::Duration;
use tarpc::context;
use tokio::time::timeout;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_napi_co_listener_byte_equal_with_embedded_subscriber() {
    // @step Given the SessionManager singleton is shared by both transports via the same SessionManagerHandle
    let (_dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let manager: Arc<dyn SessionManagerHandle> = Arc::new(
        StubSessionManagerHandle::with_provider(Arc::new(StubProvider::new())),
    );
    let service = Arc::new(SharedFspecService::with_session_manager(
        Arc::clone(&watcher),
        Arc::clone(&manager),
    ));

    let embedded =
        EmbeddedTransport::new(tokio::runtime::Handle::current(), Arc::clone(&service));
    let (addr, _stats, _join) =
        bind_and_serve("127.0.0.1:0", Arc::clone(&service))
            .await
            .expect("bind_and_serve");
    let ws_client = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("ws_client_connect");

    // @step And both an embedded subscriber and the WS client are subscribed to chunks_rx
    let mut em_rx = embedded.chunks_rx();
    let mut ws_rx = ws_client.chunks_rx();

    // @step When a session is created and input is sent on either transport
    let sid: SessionId = embedded
        .client()
        .create_session(context::current(), None)
        .await
        .expect("create_session");
    embedded
        .client()
        .send_input(context::current(), sid.clone(), "hi".to_string())
        .await
        .expect("send_input");

    async fn drain(
        rx: &mut tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
        sid: &SessionId,
    ) -> Vec<StreamChunk> {
        let mut out = Vec::new();
        for _ in 0..32 {
            match timeout(Duration::from_secs(2), rx.recv()).await {
                Ok(Ok((got, c))) if got == *sid => {
                    let done = matches!(c, StreamChunk::Done { .. });
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

    let em_chunks = drain(&mut em_rx, &sid).await;
    let ws_chunks = drain(&mut ws_rx, &sid).await;

    // @step Then the embedded subscriber and the WS client observe byte-equal StreamChunks
    assert!(!em_chunks.is_empty(), "embedded subscriber must observe chunks");
    assert!(!ws_chunks.is_empty(), "WS subscriber must observe chunks");
    assert_eq!(
        bincode::serialize(&em_chunks).expect("encode embedded"),
        bincode::serialize(&ws_chunks).expect("encode WS"),
        "embedded and WS subscribers must observe byte-equal chunk sequences \
         when both are connected to the same SessionManagerHandle (NAPI \
         co-listener parity, Rust half)",
    );
}
