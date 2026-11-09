//! RPC-445: workspace-keyed work-unit pushes.
//!
//! Feature: spec/features/workspace-work-units-push.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::{bind_and_serve, ws_client_connect};
use codelet_rpc_types::WorkspaceWorkUnits;
use common::connect_with_retry;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tarpc::context;
use tokio::sync::broadcast;

fn write_units(project: &Path, unit: &str) {
    std::fs::write(
        project.join("spec/work-units.json"),
        format!(
            r#"{{"workUnits":{{"{unit}":{{"id":"{unit}","title":"{unit} work","type":"story","status":"backlog"}}}}}}"#
        ),
    )
    .expect("write work-units.json");
}

fn project(root: &Path, name: &str, unit: &str) -> PathBuf {
    let path = root.join(name);
    std::fs::create_dir_all(path.join("spec")).expect("mkdir spec");
    write_units(&path, unit);
    std::fs::canonicalize(path).expect("canonicalize project")
}

fn service(start: &Path) -> Arc<SharedFspecService> {
    let watcher = Arc::new(WorkUnitsWatcher::new(start).expect("watcher"));
    Arc::new(SharedFspecService::new(watcher).with_cwd(start.to_path_buf()))
}

/// Wait for a push for `workspace` whose units contain `unit`.
async fn expect_push(
    rx: &mut broadcast::Receiver<WorkspaceWorkUnits>,
    workspace: &Path,
    unit: &str,
) -> WorkspaceWorkUnits {
    let wanted = workspace.to_string_lossy().into_owned();
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        loop {
            match rx.recv().await {
                Ok(push) if push.workspace == wanted && push.units.iter().any(|u| u.id == unit) => {
                    return push
                }
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => panic!("workspace stream closed"),
            }
        }
    })
    .await
    .expect("workspace push within 10s")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn an_opened_workspaces_changes_reach_the_service_stream_tagged_with_its_path() {
    // @step Given a daemon service started in "alpha" that has opened "beta"
    let data = tempfile::tempdir().expect("data dir");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data.path().to_path_buf()).expect("data dir");
    let root = tempfile::tempdir().expect("root");
    let alpha = project(root.path(), "alpha", "ALPHA-001");
    let beta = project(root.path(), "beta", "BETA-001");
    let service = service(&alpha);
    let mut rx = service.workspace_work_units_rx();
    drop(service.workspace(Some(&beta)).expect("open beta"));

    // @step When beta's work units change on disk
    // TEST-042: no sleep; beta's watcher is registered when it is opened.
    write_units(&beta, "BETA-002");

    // @step Then the service's workspace stream delivers beta's new work units tagged with beta's canonical path
    let push = expect_push(&mut rx, &beta, "BETA-002").await;
    assert_eq!(push.workspace, beta.to_string_lossy());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn an_opened_workspaces_changes_are_pushed_to_websocket_clients() {
    // @step Given a WebSocket client connected to a daemon started in "alpha"
    let data = tempfile::tempdir().expect("data dir");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data.path().to_path_buf()).expect("data dir");
    let root = tempfile::tempdir().expect("root");
    let alpha = project(root.path(), "alpha", "ALPHA-001");
    let beta = project(root.path(), "beta", "BETA-001");
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service(&alpha))
        .await
        .expect("bind_and_serve");
    let client = ws_client_connect(connect_with_retry(addr.port()).await)
        .await
        .expect("ws client");
    let mut rx = client.workspace_work_units_rx();

    // @step And the client has read the work units of "beta"
    let units = client
        .client()
        .list_workspace_work_units(context::current(), beta.to_string_lossy().into_owned())
        .await
        .expect("rpc")
        .expect("beta work units");
    assert_eq!(units.len(), 1);

    // @step When beta's work units change on disk
    // TEST-042: no sleep; beta's watcher is registered when it is opened.
    write_units(&beta, "BETA-002");

    // @step Then the client receives beta's new work units tagged with beta's canonical path
    let push = expect_push(&mut rx, &beta, "BETA-002").await;
    assert_eq!(push.workspace, beta.to_string_lossy());
}
