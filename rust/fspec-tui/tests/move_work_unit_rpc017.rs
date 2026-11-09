//! RPC-017 — cross-transport parity for `FspecBackend::move_work_unit_up/_down`.
//!
//! Feature: spec/features/rpc017-cross-transport-parity.feature
//!
//! Mirrors the RPC-015 cross-transport-parity pattern (checkpoint counts):
//! drive the SAME scripted scenario (seed temp workspace → call the
//! reorder method → re-read the file) against BOTH transports and
//! assert identical post-state.

use codelet_test_helpers::{OrFail, TestResult};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use serde_json::Value;
use tempfile::TempDir;

/// Seed a temp workspace whose `spec/work-units.json::states.backlog`
/// contains the supplied ids in order, plus optional done-column ids.
fn seed_workspace(
    backlog: &[&str],
    done: &[&str],
) -> Result<TempDir, codelet_test_helpers::TestError> {
    let dir = tempfile::tempdir().or_fail("tempdir")?;
    fs::create_dir_all(dir.path().join("spec")).or_fail("mkdir spec/")?;
    let mut work_units = serde_json::Map::new();
    for id in backlog {
        work_units.insert(
            id.to_string(),
            serde_json::json!({"id": id, "title": id, "type": "story", "status": "backlog"}),
        );
    }
    for id in done {
        work_units.insert(
            id.to_string(),
            serde_json::json!({"id": id, "title": id, "type": "story", "status": "done"}),
        );
    }
    let states = serde_json::json!({
        "backlog": backlog,
        "specifying": [],
        "testing": [],
        "implementing": [],
        "validating": [],
        "done": done,
        "blocked": [],
    });
    let root = serde_json::json!({
        "meta": {"version": "1.0.0", "lastUpdated": "2026-01-01T00:00:00.000Z"},
        "workUnits": Value::Object(work_units),
        "states": states,
    });
    fs::write(
        dir.path().join("spec").join("work-units.json"),
        serde_json::to_string_pretty(&root).or_fail("expected a value")?,
    )
    .or_fail("write work-units.json")?;
    Ok(dir)
}

fn service_for(
    repo_path: &Path,
) -> Result<Arc<SharedFspecService>, codelet_test_helpers::TestError> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo_path).or_fail("WorkUnitsWatcher::new")?);
    Ok(Arc::new(
        SharedFspecService::new(watcher).with_cwd(repo_path.to_path_buf()),
    ))
}

/// RPC-450: the workspace argument naming `path`.
fn workspace(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn read_backlog_ids(workspace: &Path) -> Result<Vec<String>, codelet_test_helpers::TestError> {
    let body = fs::read_to_string(workspace.join("spec").join("work-units.json"))
        .or_fail("read work-units.json")?;
    let v: Value = serde_json::from_str(&body).or_fail("parse JSON")?;
    Ok(v.get("states")
        .and_then(|s| s.get("backlog"))
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(std::string::ToString::to_string))
                .collect()
        })
        .unwrap_or_default())
}

/// Scenario: EmbeddedFspecBackend.move_work_unit_up persists through the shared helper
#[tokio::test]
async fn embedded_backend_move_work_unit_up_persists_through_shared_helper() -> TestResult {
    // @step Given a SharedFspecService::with_cwd against a temp workspace whose states.backlog == ["A-001", "B-002", "C-003"]
    let dir = seed_workspace(&["A-001", "B-002", "C-003"], &[])?;
    let service = service_for(dir.path())?;
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));

    // @step When backend.move_work_unit_up(workspace, "C-003".into()).await is invoked
    let result = backend
        .move_work_unit_up(workspace(dir.path()), "C-003".to_string())
        .await;

    // @step Then the awaited result is Ok(())
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    // @step And the workspace's spec/work-units.json now has states.backlog == ["A-001", "C-003", "B-002"]
    let after = read_backlog_ids(dir.path())?;
    assert_eq!(after, vec!["A-001", "C-003", "B-002"]);
    Ok(())
}

/// Scenario: WebSocketFspecBackend.move_work_unit_down crosses tarpc cleanly
#[tokio::test]
async fn websocket_backend_move_work_unit_down_crosses_tarpc_cleanly() -> TestResult {
    // @step Given an rpc-server bound to a SharedFspecService::with_cwd whose states.backlog == ["A-001", "B-002", "C-003"]
    let dir = seed_workspace(&["A-001", "B-002", "C-003"], &[])?;
    let service = service_for(dir.path())?;
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .or_fail("bind_and_serve")?;
    let url = url::Url::parse(&format!("ws://{addr}/")).or_fail("ws url")?;
    // @step And a WebSocketFspecBackend connected to that server via the standard ws_server_for test helper
    let backend: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url)
            .await
            .or_fail("connect")?,
    );

    // @step When backend.move_work_unit_down(workspace, "A-001".into()).await is invoked
    let result = backend
        .move_work_unit_down(workspace(dir.path()), "A-001".to_string())
        .await;

    // @step Then the awaited result is Ok(())
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    // @step And the workspace's spec/work-units.json now has states.backlog == ["B-002", "A-001", "C-003"]
    let after = read_backlog_ids(dir.path())?;
    assert_eq!(after, vec!["B-002", "A-001", "C-003"]);
    Ok(())
}

/// Scenario: Both transports return Err for done-column targets
#[tokio::test]
async fn both_transports_return_err_for_done_column_targets() -> TestResult {
    // @step Given a SharedFspecService::with_cwd whose states.done == ["DONE-001", "DONE-002"]
    let dir = seed_workspace(&[], &["DONE-001", "DONE-002"])?;
    let service = service_for(dir.path())?;

    // @step And both EmbeddedFspecBackend and WebSocketFspecBackend wrapping that shared service
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        Arc::clone(&service),
    ));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .or_fail("bind_and_serve")?;
    let url = url::Url::parse(&format!("ws://{addr}/")).or_fail("ws url")?;
    let ws: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url)
            .await
            .or_fail("connect")?,
    );

    // @step When each transport calls move_work_unit_up(workspace, "DONE-001".into()).await
    let r_embedded = embedded
        .move_work_unit_up(workspace(dir.path()), "DONE-001".to_string())
        .await;
    let r_ws = ws
        .move_work_unit_up(workspace(dir.path()), "DONE-001".to_string())
        .await;

    // @step Then both calls return Err
    assert!(
        r_embedded.is_err(),
        "embedded transport must return Err for done-column reorder, got {r_embedded:?}"
    );
    assert!(
        r_ws.is_err(),
        "ws transport must return Err for done-column reorder, got {r_ws:?}"
    );
    Ok(())
}

/// Scenario: FspecService::move_work_unit_up delegates through SharedFspecService::cwd to the shared helper
#[tokio::test]
async fn fspec_service_move_work_unit_up_delegates_through_shared_service_cwd() -> TestResult {
    // @step Given a SharedFspecService constructed via with_cwd against a temp workspace whose states.backlog == ["A-001", "B-002"]
    let dir = seed_workspace(&["A-001", "B-002"], &[])?;
    let service = service_for(dir.path())?;
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));

    // @step When client.move_work_unit_up(context::current(), workspace, "B-002") is invoked
    let result = backend
        .move_work_unit_up(workspace(dir.path()), "B-002".to_string())
        .await;

    // @step Then the call returns Ok(())
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    // @step And the workspace's spec/work-units.json now has states.backlog == ["B-002", "A-001"]
    let after = read_backlog_ids(dir.path())?;
    assert_eq!(after, vec!["B-002", "A-001"]);
    Ok(())
}

/// Scenario: FspecService::move_work_unit_down delegates through SharedFspecService::cwd to the shared helper
#[tokio::test]
async fn fspec_service_move_work_unit_down_delegates_through_shared_service_cwd() -> TestResult {
    // @step Given a SharedFspecService constructed via with_cwd against a temp workspace whose states.backlog == ["A-001", "B-002"]
    let dir = seed_workspace(&["A-001", "B-002"], &[])?;
    let service = service_for(dir.path())?;
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));

    // @step When client.move_work_unit_down(context::current(), workspace, "A-001") is invoked
    let result = backend
        .move_work_unit_down(workspace(dir.path()), "A-001".to_string())
        .await;

    // @step Then the call returns Ok(())
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    // @step And the workspace's spec/work-units.json now has states.backlog == ["B-002", "A-001"]
    let after = read_backlog_ids(dir.path())?;
    assert_eq!(after, vec!["B-002", "A-001"]);
    Ok(())
}

/// Scenario: FspecService::move_work_unit_up returns Err for a workspace that does not exist
#[tokio::test]
async fn fspec_service_move_work_unit_up_returns_err_for_a_missing_workspace() -> TestResult {
    // @step Given a SharedFspecService started in a temp workspace
    let dir = seed_workspace(&["A-001", "B-002"], &[])?;
    let service = service_for(dir.path())?;
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));

    // @step When client.move_work_unit_up(context::current(), missing_workspace, "A-001") is invoked
    let missing = workspace(&dir.path().join("missing-workspace"));
    let result = backend
        .move_work_unit_up(missing, "A-001".to_string())
        .await;

    // @step Then the call returns Err
    assert!(
        result.is_err(),
        "expected Err for a missing workspace, got {result:?}"
    );
    Ok(())
}
