//! RPC-059 — Cross-transport parity for the /loop RPC surface.
//!
//! Feature: spec/features/rpc059-loop-cross-transport-parity.feature
//!
//! Drives identical scripted scenarios against EmbeddedFspecBackend AND
//! WebSocketFspecBackend, constructed against the SAME deterministic
//! StubSessionManagerHandle. Mirrors the RPC-058 cross-transport parity
//! pattern.

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
use codelet_rpc_types::{RegisteredLoop, SessionId};
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

fn sample_loop(id: &str, session_id: &str, interval: u32, prompt: &str) -> RegisteredLoop {
    RegisteredLoop {
        id: id.to_string(),
        session_id: SessionId::new(session_id),
        prompt: prompt.to_string(),
        interval_seconds: interval,
        created_at: "2026-05-24T00:00:00Z".to_string(),
        expires_at: "2026-05-27T00:00:00Z".to_string(),
        last_run_at: None,
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket loop_add both reach the stub
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn loop_add_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with a RegisteredLoop { id: "a1b2c3d4", session_id: SessionId::new("s-1"), prompt: "check the build", interval_seconds: 30, created_at: "2026-05-24T00:00:00Z", expires_at: "2026-05-27T00:00:00Z", last_run_at: None } behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    let (_temp, service, stub) = build_service();
    stub.seed_registered_loop(sample_loop("a1b2c3d4", "s-1", 30, "check the build"));
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.loop_add_calls();

    // @step When loop_add is called via the embedded transport with session_id "s-1" and interval_seconds 30 and prompt "check the build"
    let em = embedded
        .loop_add(SessionId::new("s-1"), 30, "check the build".to_string())
        .await
        .expect("embedded loop_add");

    // @step And loop_add is called via the WebSocket transport with session_id "s-1" and interval_seconds 30 and prompt "check the build"
    let ws = websocket
        .loop_add(SessionId::new("s-1"), 30, "check the build".to_string())
        .await
        .expect("websocket loop_add");

    // @step Then the stub's loop_add_calls counter equals 2
    assert_eq!(
        stub.loop_add_calls() - initial,
        2,
        "loop_add_calls should increment by 2"
    );

    // @step And both calls return Ok(RegisteredLoop) with byte-identical field values
    assert_eq!(em, ws, "embedded and websocket loop_add must match");
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket loop_cancel both reach the stub
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn loop_cancel_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded to return Ok(true) for loop_cancel behind both transports
    let (_temp, service, stub) = build_service();
    stub.seed_loop_cancel_result(true);
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.loop_cancel_calls();

    // @step When loop_cancel is called via the embedded transport with id "a1b2c3d4"
    let em = embedded
        .loop_cancel("a1b2c3d4".to_string())
        .await
        .expect("embedded loop_cancel");

    // @step And loop_cancel is called via the WebSocket transport with id "a1b2c3d4"
    let ws = websocket
        .loop_cancel("a1b2c3d4".to_string())
        .await
        .expect("websocket loop_cancel");

    // @step Then the stub's loop_cancel_calls counter equals 2
    assert_eq!(
        stub.loop_cancel_calls() - initial,
        2,
        "loop_cancel_calls should increment by 2"
    );

    // @step And both calls return Ok(true)
    assert!(em);
    assert!(ws);
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket loop_list both reach the stub
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn loop_list_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with two RegisteredLoop rows for session "s-1" behind both transports
    let (_temp, service, stub) = build_service();
    stub.seed_registered_loops(vec![
        sample_loop("a1b2c3d4", "s-1", 30, "check build"),
        sample_loop("e5f6g7h8", "s-1", 300, "check status"),
    ]);
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.loop_list_calls();

    // @step When loop_list is called via the embedded transport for session_id "s-1"
    let em = embedded
        .loop_list(SessionId::new("s-1"))
        .await
        .expect("embedded loop_list");

    // @step And loop_list is called via the WebSocket transport for session_id "s-1"
    let ws = websocket
        .loop_list(SessionId::new("s-1"))
        .await
        .expect("websocket loop_list");

    // @step Then the stub's loop_list_calls counter equals 2
    assert_eq!(
        stub.loop_list_calls() - initial,
        2,
        "loop_list_calls should increment by 2"
    );

    // @step And both calls return a Vec of length 2
    assert_eq!(em.len(), 2);
    assert_eq!(ws.len(), 2);

    // @step And each entry has identical id, session_id, prompt, interval_seconds, created_at, expires_at, last_run_at fields across the two transports
    for (e, w) in em.iter().zip(ws.iter()) {
        assert_eq!(e, w);
    }
}
