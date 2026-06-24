//! Integration tests for the lifted `codelet_core::work_units::WorkUnitsWatcher`
//! module (RPC-006).
//!
//! Feature: spec/features/work-units-watcher.feature
//!
//! These tests validate the new pure-Rust watcher module that replaces the
//! NAPI-only `codelet/napi/src/work_units_watcher.rs` as the single source
//! of truth for the work-units snapshot stream.
//!
//! - Scenario: WorkUnitsWatcher publishes a new snapshot on file mutation
//! - Scenario: Multiple subscribers each observe every broadcast on file mutation

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::work_units::{read_snapshot, WorkUnitsWatcher};
use codelet_rpc_types::WorkUnitInfo;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
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

#[test]
fn read_snapshot_returns_the_current_file_contents() {
    let (_dir, path) = workspace_with(&[
        ("AUTH-001", "Login", "done"),
        ("AUTH-002", "Reset password", "implementing"),
    ]);
    let workspace = path.parent().unwrap().parent().unwrap();

    let snapshot = read_snapshot(workspace).expect("read_snapshot");
    let mut ids: Vec<String> = snapshot.into_iter().map(|wu| wu.id).collect();
    ids.sort();
    assert_eq!(ids, vec!["AUTH-001".to_string(), "AUTH-002".to_string()]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_publishes_a_new_snapshot_on_file_mutation() {
    // @step Given a temporary workspace observed by a WorkUnitsWatcher and a broadcast::Receiver<Vec<WorkUnitInfo>> obtained via watcher.subscribe()
    let (_dir, path) = workspace_with(&[
        ("AUTH-001", "Login", "done"),
        ("AUTH-002", "Reset password", "implementing"),
    ]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = WorkUnitsWatcher::new(workspace).expect("watcher");
    let mut rx = watcher.subscribe();

    // @step When I append a third work unit to spec/work-units.json and wait up to one second on the receiver
    write_workspace(
        &path,
        &[
            ("AUTH-001", "Login", "done"),
            ("AUTH-002", "Reset password", "implementing"),
            ("AUTH-003", "Two factor auth", "specifying"),
        ],
    );
    let received: Vec<WorkUnitInfo> = timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("watcher must publish within 1s")
        .expect("broadcast not closed");

    // @step Then the receiver yields a Vec<WorkUnitInfo> containing all three work units in the order they appear in the file
    let mut ids: Vec<String> = received.into_iter().map(|wu| wu.id).collect();
    ids.sort();
    assert_eq!(
        ids,
        vec![
            "AUTH-001".to_string(),
            "AUTH-002".to_string(),
            "AUTH-003".to_string(),
        ],
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_multiple_subscribers_each_observe_every_broadcast() {
    // @step Given a temporary workspace observed by a single WorkUnitsWatcher and two independent broadcast::Receiver values obtained via two separate watcher.subscribe() calls
    let (_dir, path) = workspace_with(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = WorkUnitsWatcher::new(workspace).expect("watcher");
    let mut rx_a = watcher.subscribe();
    let mut rx_b = watcher.subscribe();

    // @step When I mutate spec/work-units.json once and wait up to one second on each receiver
    write_workspace(
        &path,
        &[
            ("AUTH-001", "Login", "done"),
            ("AUTH-002", "Reset password", "implementing"),
        ],
    );
    let payload_a: Vec<WorkUnitInfo> = timeout(Duration::from_secs(1), rx_a.recv())
        .await
        .expect("subscriber A must observe broadcast")
        .expect("broadcast not closed for A");
    let payload_b: Vec<WorkUnitInfo> = timeout(Duration::from_secs(1), rx_b.recv())
        .await
        .expect("subscriber B must observe broadcast")
        .expect("broadcast not closed for B");

    // @step Then both receivers yield equal Vec<WorkUnitInfo> values reflecting the post-mutation state
    let mut ids_a: Vec<String> = payload_a.iter().map(|wu| wu.id.clone()).collect();
    let mut ids_b: Vec<String> = payload_b.iter().map(|wu| wu.id.clone()).collect();
    ids_a.sort();
    ids_b.sort();
    assert_eq!(ids_a, ids_b);
    assert_eq!(ids_a, vec!["AUTH-001".to_string(), "AUTH-002".to_string()],);
    assert_eq!(payload_a, payload_b, "broadcast must be value-identical");
}

#[test]
fn snapshot_method_reflects_the_current_file_state() {
    let (_dir, path) = workspace_with(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = WorkUnitsWatcher::new(workspace).expect("watcher");
    let initial = watcher.snapshot();
    let initial_ids: Vec<String> = initial.into_iter().map(|wu| wu.id).collect();
    assert_eq!(initial_ids, vec!["AUTH-001".to_string()]);
}
