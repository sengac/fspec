//! RPC-017 — cross-transport parity for `FspecBackend::move_work_unit_up/_down`.
//!
//! Feature: spec/features/rpc017-cross-transport-parity.feature
//!
//! Mirrors the RPC-015 cross-transport-parity pattern (checkpoint counts):
//! drive the SAME scripted scenario (seed temp workspace → call the
//! reorder method → re-read the file) against BOTH transports and
//! assert identical post-state.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
fn seed_workspace(backlog: &[&str], done: &[&str]) -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("spec")).expect("mkdir spec/");
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
        serde_json::to_string_pretty(&root).unwrap(),
    )
    .expect("write work-units.json");
    dir
}

fn service_for(repo_path: &Path) -> Arc<SharedFspecService> {
    let watcher = Arc::new(WorkUnitsWatcher::new(repo_path).expect("WorkUnitsWatcher::new"));
    Arc::new(SharedFspecService::new(watcher).with_cwd(repo_path.to_path_buf()))
}

fn read_backlog_ids(workspace: &Path) -> Vec<String> {
    let body = fs::read_to_string(workspace.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    let v: Value = serde_json::from_str(&body).expect("parse JSON");
    v.get("states")
        .and_then(|s| s.get("backlog"))
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(std::string::ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Scenario: EmbeddedFspecBackend.move_work_unit_up persists through the shared helper
#[tokio::test]
async fn embedded_backend_move_work_unit_up_persists_through_shared_helper() {
    // @step Given a SharedFspecService::with_cwd against a temp workspace whose states.backlog == ["A-001", "B-002", "C-003"]
    let dir = seed_workspace(&["A-001", "B-002", "C-003"], &[]);
    let service = service_for(dir.path());
    // @step And an EmbeddedFspecBackend wrapping that shared service
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));

    // @step When backend.move_work_unit_up("C-003".into()).await is invoked
    let result = backend.move_work_unit_up("C-003".to_string()).await;

    // @step Then the awaited result is Ok(())
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    // @step And the workspace's spec/work-units.json now has states.backlog == ["A-001", "C-003", "B-002"]
    let after = read_backlog_ids(dir.path());
    assert_eq!(after, vec!["A-001", "C-003", "B-002"]);
}

/// Scenario: WebSocketFspecBackend.move_work_unit_down crosses tarpc cleanly
#[tokio::test]
async fn websocket_backend_move_work_unit_down_crosses_tarpc_cleanly() {
    // @step Given an rpc-server bound to a SharedFspecService::with_cwd whose states.backlog == ["A-001", "B-002", "C-003"]
    let dir = seed_workspace(&["A-001", "B-002", "C-003"], &[]);
    let service = service_for(dir.path());
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    // @step And a WebSocketFspecBackend connected to that server via the standard ws_server_for test helper
    let backend: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));

    // @step When backend.move_work_unit_down("A-001".into()).await is invoked
    let result = backend.move_work_unit_down("A-001".to_string()).await;

    // @step Then the awaited result is Ok(())
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    // @step And the workspace's spec/work-units.json now has states.backlog == ["B-002", "A-001", "C-003"]
    let after = read_backlog_ids(dir.path());
    assert_eq!(after, vec!["B-002", "A-001", "C-003"]);
}

/// Scenario: Both transports return Err for done-column targets
#[tokio::test]
async fn both_transports_return_err_for_done_column_targets() {
    // @step Given a SharedFspecService::with_cwd whose states.done == ["DONE-001", "DONE-002"]
    let dir = seed_workspace(&[], &["DONE-001", "DONE-002"]);
    let service = service_for(dir.path());

    // @step And both EmbeddedFspecBackend and WebSocketFspecBackend wrapping that shared service
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        Arc::clone(&service),
    ));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    let ws: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));

    // @step When each transport calls move_work_unit_up("DONE-001".into()).await
    let r_embedded = embedded.move_work_unit_up("DONE-001".to_string()).await;
    let r_ws = ws.move_work_unit_up("DONE-001".to_string()).await;

    // @step Then both calls return Err
    assert!(
        r_embedded.is_err(),
        "embedded transport must return Err for done-column reorder, got {r_embedded:?}"
    );
    assert!(
        r_ws.is_err(),
        "ws transport must return Err for done-column reorder, got {r_ws:?}"
    );
}

/// Scenario: napi::move_work_unit_up is wired through the same shared helper
#[test]
fn napi_move_work_unit_up_is_wired_through_the_same_shared_helper() {
    // @step Given codelet/napi/src/work_units_watcher.rs after RPC-017 lands
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("napi")
        .join("src")
        .join("work_units_watcher.rs");
    let body = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

    // @step When a developer reads the file source raw
    // @step Then the file contains the substring "pub fn move_work_unit_up"
    assert!(
        body.contains("pub fn move_work_unit_up"),
        "codelet/napi/src/work_units_watcher.rs must export `pub fn move_work_unit_up`"
    );
    // @step And the file contains the substring "pub fn move_work_unit_down"
    assert!(
        body.contains("pub fn move_work_unit_down"),
        "codelet/napi/src/work_units_watcher.rs must export `pub fn move_work_unit_down`"
    );
    // @step And both function bodies contain the substring "codelet_core::work_units_write::move_work_unit"
    assert!(
        body.contains("codelet_core::work_units_write::move_work_unit"),
        "napi reorder exports must delegate to codelet_core::work_units_write::move_work_unit"
    );
}

/// Scenario: FspecService::move_work_unit_up delegates through SharedFspecService::cwd to the shared helper
#[tokio::test]
async fn fspec_service_move_work_unit_up_delegates_through_shared_service_cwd() {
    // @step Given a SharedFspecService constructed via with_cwd against a temp workspace whose states.backlog == ["A-001", "B-002"]
    let dir = seed_workspace(&["A-001", "B-002"], &[]);
    let service = service_for(dir.path());
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));

    // @step When client.move_work_unit_up(context::current(), "B-002") is invoked
    let result = backend.move_work_unit_up("B-002".to_string()).await;

    // @step Then the call returns Ok(())
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    // @step And the workspace's spec/work-units.json now has states.backlog == ["B-002", "A-001"]
    let after = read_backlog_ids(dir.path());
    assert_eq!(after, vec!["B-002", "A-001"]);
}

/// Scenario: FspecService::move_work_unit_down delegates through SharedFspecService::cwd to the shared helper
#[tokio::test]
async fn fspec_service_move_work_unit_down_delegates_through_shared_service_cwd() {
    // @step Given a SharedFspecService constructed via with_cwd against a temp workspace whose states.backlog == ["A-001", "B-002"]
    let dir = seed_workspace(&["A-001", "B-002"], &[]);
    let service = service_for(dir.path());
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));

    // @step When client.move_work_unit_down(context::current(), "A-001") is invoked
    let result = backend.move_work_unit_down("A-001".to_string()).await;

    // @step Then the call returns Ok(())
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    // @step And the workspace's spec/work-units.json now has states.backlog == ["B-002", "A-001"]
    let after = read_backlog_ids(dir.path());
    assert_eq!(after, vec!["B-002", "A-001"]);
}

/// Scenario: FspecService::move_work_unit_up returns Err when no cwd is attached
#[tokio::test]
async fn fspec_service_move_work_unit_up_returns_err_without_cwd() {
    // @step Given a SharedFspecService constructed via new() WITHOUT with_cwd
    let dir = seed_workspace(&["A-001", "B-002"], &[]);
    let watcher = Arc::new(WorkUnitsWatcher::new(dir.path()).expect("WorkUnitsWatcher::new"));
    let service = Arc::new(SharedFspecService::new(watcher));
    let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ));

    // @step When client.move_work_unit_up(context::current(), "A-001") is invoked
    let result = backend.move_work_unit_up("A-001".to_string()).await;

    // @step Then the call returns Err
    assert!(
        result.is_err(),
        "expected Err when no cwd is attached, got {result:?}"
    );
}
