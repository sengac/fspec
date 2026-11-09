//! RPC-055 — Cross-transport parity for the /debug RPC surface.
//!
//! Feature: spec/features/rpc055-slash-debug-cross-transport-parity.feature
//!
//! Drives identical scripted scenarios against EmbeddedFspecBackend AND
//! WebSocketFspecBackend, constructed against the SAME deterministic
//! StubSessionManagerHandle. Mirrors the RPC-054 cross-transport pattern.

#![allow(clippy::await_holding_lock, clippy::too_many_lines)]

use codelet_test_helpers::{OrFail, TestResult};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use tempfile::TempDir;

fn workspace_with_seed(cwd: &Path) -> Result<(), codelet_test_helpers::TestError> {
    fs::create_dir_all(cwd.join("spec")).or_fail("mkdir spec/")?;
    fs::write(
        cwd.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .or_fail("write work-units.json")?;
    Ok(())
}

fn build_service() -> Result<
    (
        TempDir,
        Arc<SharedFspecService>,
        Arc<StubSessionManagerHandle>,
    ),
    codelet_test_helpers::TestError,
> {
    let temp = tempfile::tempdir().or_fail("tempdir")?;
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd)?;
    let watcher = Arc::new(WorkUnitsWatcher::new(&cwd).or_fail("watcher")?);
    let stub = Arc::new(StubSessionManagerHandle::new());
    let handle: Arc<dyn SessionManagerHandle> = stub.clone();
    let service = Arc::new(SharedFspecService::with_session_manager(watcher, handle).with_cwd(cwd));
    Ok((temp, service, stub))
}

async fn dual_backends(
    service: Arc<SharedFspecService>,
) -> Result<(Arc<dyn FspecBackend>, Arc<dyn FspecBackend>), codelet_test_helpers::TestError> {
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .or_fail("bind_and_serve")?;
    let url = url::Url::parse(&format!("ws://{addr}/")).or_fail("ws url")?;
    let websocket: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url)
            .await
            .or_fail("connect")?,
    );
    Ok((embedded, websocket))
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket set_debug_directory both reach the stub
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn set_debug_directory_round_trips_identically_across_transports() -> TestResult {
    // @step Given a StubSessionManagerHandle behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    let (_temp, service, stub) = build_service()?;
    let (embedded, websocket) = dual_backends(service).await?;

    let initial = stub.set_debug_directory_calls();

    // @step When set_debug_directory("/tmp/dbg-A") is called via the embedded transport
    let em = embedded.set_debug_directory("/tmp/dbg-A".to_string()).await;
    assert!(em.is_ok(), "embedded set_debug_directory: {em:?}");

    // @step And set_debug_directory("/tmp/dbg-B") is called via the WebSocket transport
    let ws = websocket
        .set_debug_directory("/tmp/dbg-B".to_string())
        .await;
    assert!(ws.is_ok(), "websocket set_debug_directory: {ws:?}");

    // @step Then the stub's set_debug_directory_calls counter equals 2
    let final_calls = stub.set_debug_directory_calls();
    assert_eq!(
        final_calls - initial,
        2,
        "stub.set_debug_directory_calls() should increment by 2 (once per transport)",
    );

    // @step And both calls return Ok(())
    assert!(em.is_ok() && ws.is_ok());
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket toggle_debug both reach the stub
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn toggle_debug_round_trips_identically_across_transports() -> TestResult {
    // @step Given a StubSessionManagerHandle behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    let (_temp, service, stub) = build_service()?;
    let (embedded, websocket) = dual_backends(service).await?;

    // @step And a session s-1 has been created on the stub
    let session_id = stub
        .create_session_checked(None)
        .or_fail("the stub creates the session")?;

    let initial = stub.toggle_debug_calls();

    // @step When toggle_debug(s-1, "/tmp/dbg-A") is called via the embedded transport
    let em = embedded
        .toggle_debug(session_id.clone(), "/tmp/dbg-A".to_string())
        .await
        .or_fail("embedded toggle_debug")?;

    // @step And toggle_debug(s-1, "/tmp/dbg-B") is called via the WebSocket transport
    let ws = websocket
        .toggle_debug(session_id.clone(), "/tmp/dbg-B".to_string())
        .await
        .or_fail("websocket toggle_debug")?;

    // @step Then the stub's toggle_debug_calls counter equals 2
    let final_calls = stub.toggle_debug_calls();
    assert_eq!(
        final_calls - initial,
        2,
        "stub.toggle_debug_calls() should increment by 2 (once per transport)",
    );

    // @step And both calls return Ok with a non-empty path string
    assert!(
        !em.is_empty(),
        "embedded toggle_debug returned empty string"
    );
    assert!(
        !ws.is_empty(),
        "websocket toggle_debug returned empty string"
    );
    Ok(())
}
