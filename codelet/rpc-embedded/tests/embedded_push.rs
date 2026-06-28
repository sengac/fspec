//! Integration tests for the embedded push channel (RPC-006).
//!
//! Feature: spec/features/embedded-work-units-push.feature
//!
//! - Scenario: list_work_units returns a live snapshot from the real WorkUnitsWatcher
//! - Scenario: Embedded transport exposes the watcher's broadcast subscription directly
//!
//! The embedded transport exposes `EmbeddedTransport::work_units_rx()` which
//! returns a clone of `WorkUnitsWatcher::subscribe()` directly — no envelope
//! encoding, no fan-out task on the embedded read path (zero-cost path).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_embedded::EmbeddedTransport;
use codelet_rpc_types::WorkUnitInfo;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tarpc::context;
use tempfile::TempDir;
use tokio::time::timeout;

fn workspace_with(units: &[(&str, &str, &str)]) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let spec_dir = dir.path().join("spec");
    fs::create_dir_all(&spec_dir).expect("mkdir spec");
    let path = spec_dir.join("work-units.json");
    write_workspace(&path, units);
    (dir, path)
}

fn write_workspace(path: &Path, units: &[(&str, &str, &str)]) {
    let mut entries = String::new();
    for (i, (id, title, status)) in units.iter().enumerate() {
        if i > 0 {
            entries.push(',');
        }
        entries.push_str(&format!(
            r#""{id}":{{"id":"{id}","title":"{title}","type":"story","status":"{status}"}}"#,
        ));
    }
    let json = format!(r#"{{"workUnits":{{{entries}}}}}"#);
    fs::write(path, json).expect("write work-units.json");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_list_work_units_returns_a_live_snapshot_from_the_real_watcher() {
    // @step Given a temporary workspace whose spec/work-units.json file declares two work units and a SharedFspecService constructed from `Arc::new(WorkUnitsWatcher::new(workspace)?)`
    let (_dir, path) = workspace_with(&[
        ("AUTH-001", "Login", "done"),
        ("AUTH-002", "Reset password", "implementing"),
    ]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let service = Arc::new(SharedFspecService::new(Arc::clone(&watcher)));

    // @step When I construct an EmbeddedTransport with the current tokio runtime Handle, obtain an FspecServiceClient, and call list_work_units on the client
    let transport = EmbeddedTransport::new(tokio::runtime::Handle::current(), service);
    let client = transport.client();
    let actual: Vec<WorkUnitInfo> = client
        .list_work_units(context::current())
        .await
        .expect("RPC must succeed");

    // @step Then the call returns Ok with a Vec<WorkUnitInfo> equal to the live snapshot derived from the spec/work-units.json file and not equal to the RPC-005 default_fixture
    let mut ids: Vec<String> = actual.iter().map(|wu| wu.id.clone()).collect();
    ids.sort();
    assert_eq!(
        ids,
        vec!["AUTH-001".to_string(), "AUTH-002".to_string()],
        "list_work_units must return the live workspace snapshot"
    );
    // Cross-check against the RPC-005 fixture (AUTH-001 + AUTH-002 with
    // specific title/status). The live snapshot here uses different
    // statuses, proving we are NOT serving the RPC-005 default fixture.
    let titles: Vec<String> = actual.iter().map(|wu| wu.title.clone()).collect();
    assert!(
        titles.contains(&"Login".to_string()),
        "actual result must include workspace-defined titles, got {titles:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_embedded_transport_exposes_the_watchers_broadcast_subscription_directly() {
    // @step Given a SharedFspecService backed by a real WorkUnitsWatcher and an EmbeddedTransport built from the current tokio runtime Handle
    let (_dir, path) = workspace_with(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let service = Arc::new(SharedFspecService::new(Arc::clone(&watcher)));
    let transport = EmbeddedTransport::new(tokio::runtime::Handle::current(), service);

    // @step When I call EmbeddedTransport::work_units_rx() to obtain a broadcast::Receiver<Vec<WorkUnitInfo>>, mutate spec/work-units.json once, and wait up to one second on the receiver
    let mut rx = transport.work_units_rx();
    write_workspace(
        &path,
        &[
            ("AUTH-001", "Login", "done"),
            ("AUTH-002", "Reset password", "implementing"),
        ],
    );
    let payload: Vec<WorkUnitInfo> = timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("embedded receiver must observe broadcast within 1s")
        .expect("broadcast not closed");

    // @step Then the receiver yields the post-mutation Vec<WorkUnitInfo> and the transport source contains no bincode encode call on the embedded push path
    let mut ids: Vec<String> = payload.into_iter().map(|wu| wu.id).collect();
    ids.sort();
    assert_eq!(ids, vec!["AUTH-001".to_string(), "AUTH-002".to_string()],);
    // The "no bincode encode" half of the Then is enforced by the
    // source-shape regression test in rpc_006_source_shape.rs::
    // scenario_embedded_push_path_has_no_bincode_serialize.
}
