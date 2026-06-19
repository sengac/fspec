//! RPC-050 — Cross-transport parity for `set_work_unit_context` and
//! `get_work_unit_context`.
//!
//! Feature: spec/features/slash-command-detach-cross-transport-parity.feature
//!
//! Drives the same scripted sequence (`set(Some) → get → set(None)`)
//! against EmbeddedFspecBackend AND WebSocketFspecBackend, constructed
//! against the SAME deterministic StubSessionManagerHandle. Mirrors the
//! RPC-049 parity pattern.

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
use codelet_rpc_types::{SessionId, WorkUnitContext};
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

/// Scenario: set_work_unit_context and get_work_unit_context round-trip
/// identically across both transports
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn set_and_get_work_unit_context_round_trip_identically_across_transports() {
    // @step Given a SharedFspecService wired to a StubSessionManagerHandle
    let (_temp, service, stub) = build_service();

    // @step And an EmbeddedFspecBackend over that service
    // @step And a WebSocketFspecBackend over that same service
    let (embedded, websocket) = dual_backends(service).await;
    let sid = SessionId::new("stub-1");
    let ctx = WorkUnitContext {
        id: "RPC-050".to_string(),
        title: "Work-unit binding".to_string(),
        status: "implementing".to_string(),
    };

    // @step And the initial set_work_unit_context call counter on the stub is 0
    let initial_set_calls = stub.set_work_unit_context_calls();
    let initial_get_calls = stub.get_work_unit_context_calls();

    // @step When backend.set_work_unit_context(SessionId("stub-1"), Some(ctx)) is awaited through each transport
    let em_set_some = embedded
        .set_work_unit_context(sid.clone(), Some(ctx.clone()))
        .await;
    let ws_set_some = websocket
        .set_work_unit_context(sid.clone(), Some(ctx.clone()))
        .await;

    // @step And backend.get_work_unit_context(SessionId("stub-1")) is awaited through each transport
    let em_get = embedded.get_work_unit_context(sid.clone()).await;
    let ws_get = websocket.get_work_unit_context(sid.clone()).await;

    // @step And backend.set_work_unit_context(SessionId("stub-1"), None) is awaited through each transport
    let em_set_none = embedded.set_work_unit_context(sid.clone(), None).await;
    let ws_set_none = websocket.set_work_unit_context(sid.clone(), None).await;

    // @step Then all six awaited calls return Ok
    assert!(em_set_some.is_ok(), "embedded set_some: {em_set_some:?}");
    assert!(ws_set_some.is_ok(), "websocket set_some: {ws_set_some:?}");
    assert!(em_get.is_ok(), "embedded get: {em_get:?}");
    assert!(ws_get.is_ok(), "websocket get: {ws_get:?}");
    assert!(em_set_none.is_ok(), "embedded set_none: {em_set_none:?}");
    assert!(ws_set_none.is_ok(), "websocket set_none: {ws_set_none:?}");

    // @step And the StubSessionManagerHandle's set_work_unit_context call counter increments by exactly 4 (twice per transport)
    assert_eq!(
        stub.set_work_unit_context_calls() - initial_set_calls,
        4,
        "stub.set_work_unit_context_calls() should increment by 4 (twice per transport)",
    );

    // @step And the StubSessionManagerHandle's get_work_unit_context call counter increments by exactly 2 (once per transport)
    assert_eq!(
        stub.get_work_unit_context_calls() - initial_get_calls,
        2,
        "stub.get_work_unit_context_calls() should increment by 2 (once per transport)",
    );

    // @step And each transport's get_work_unit_context call returns the previously-stored WorkUnitContext
    assert_eq!(
        em_get.unwrap(),
        Some(ctx.clone()),
        "embedded get_work_unit_context must return the ctx written by the prior set_some call",
    );
    assert_eq!(
        ws_get.unwrap(),
        Some(ctx),
        "websocket get_work_unit_context must return the ctx written by the prior set_some call",
    );
}
