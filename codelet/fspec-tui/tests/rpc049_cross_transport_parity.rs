//! RPC-049 — Cross-transport parity for the new `resume_session`
//! aggregate RPC.
//!
//! Feature: spec/features/slash-command-resume.feature
//!
//! Drives identical scripted scenarios against EmbeddedFspecBackend AND
//! WebSocketFspecBackend, constructed against the SAME deterministic
//! StubSessionManagerHandle. Mirrors the RPC-037 parity pattern (shared
//! service + bind_and_serve + WS client).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::await_holding_lock,
    clippy::too_many_lines
)]

use std::fs;
use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::SessionId;
use tempfile::TempDir;

fn workspace_with_seed(cwd: &Path) {
    fs::create_dir_all(cwd.join("spec")).expect("mkdir spec/");
    fs::write(
        cwd.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
}

fn build_service() -> (
    TempDir,
    Arc<SharedFspecService>,
    Arc<StubSessionManagerHandle>,
) {
    let temp = tempfile::tempdir().expect("tempdir");
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd);
    let watcher = Arc::new(WorkUnitsWatcher::new(&cwd).expect("watcher"));
    let stub = Arc::new(StubSessionManagerHandle::new());
    let handle: Arc<dyn SessionManagerHandle> = stub.clone();
    let service = Arc::new(SharedFspecService::with_session_manager(watcher, handle).with_cwd(cwd));
    (temp, service, stub)
}

async fn dual_backends(
    service: Arc<SharedFspecService>,
) -> (Arc<dyn FspecBackend>, Arc<dyn FspecBackend>) {
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    let websocket: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    (embedded, websocket)
}

/// Scenario: Cross-transport parity for resume_session
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn resume_session_round_trips_identically_across_transports() {
    // @step Given a SharedFspecService wired to a StubSessionManagerHandle
    let (_temp, service, stub) = build_service();

    // @step And an EmbeddedFspecBackend over that service
    // @step And a WebSocketFspecBackend over that same service
    let (embedded, websocket) = dual_backends(service).await;
    let sid = SessionId::new("stub-1");

    let initial_calls = stub.resume_session_calls();

    // @step When backend.resume_session(SessionId("stub-1")) is awaited through each transport
    let em_result = embedded.resume_session(sid.clone()).await;
    let ws_result = websocket.resume_session(sid).await;

    // @step Then both calls return Ok(())
    assert!(em_result.is_ok(), "embedded resume_session: {em_result:?}");
    assert!(ws_result.is_ok(), "websocket resume_session: {ws_result:?}");

    // @step And the StubSessionManagerHandle's resume_session call counter increments by 2 (once per transport)
    let final_calls = stub.resume_session_calls();
    assert_eq!(
        final_calls - initial_calls,
        2,
        "stub.resume_session_calls() should increment by 2 (once per transport)",
    );
}
