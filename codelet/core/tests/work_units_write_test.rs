//! RPC-017 — Integration tests for `codelet_core::work_units_write::move_work_unit`.
//!
//! Feature: spec/features/rpc017-work-units-write-helper.feature
//!
//! These tests drive the new pure-Rust write-side helper that the
//! shared `FspecServiceImpl::move_work_unit_up/_down` RPC methods + the
//! two additive `napi::move_work_unit_up/_down` NAPI exports both
//! delegate to. The helper must mirror the column-scoped semantics of
//! `src/commands/prioritize-work-unit.ts` (no cross-column moves; done
//! column refuses reorders) and persist via the same proper-lockfile-
//! compatible mkdir-lock + atomic temp+rename pattern as
//! `codelet/napi/src/schedule_handler.rs` so concurrent TS writers
//! cooperate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

use codelet_core::work_units_write::{move_work_unit, Direction};
use serde_json::Value;
use tempfile::TempDir;

/// Write a minimal `spec/work-units.json` covering only the columns and
/// units the test cares about. Returns the workspace tempdir.
fn workspace_with(backlog: &[&str], done: &[&str], meta_last_updated: Option<&str>) -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("spec")).expect("mkdir spec/");
    let mut work_units_obj = serde_json::Map::new();
    for id in backlog.iter().chain(done.iter()) {
        let status = if backlog.iter().any(|b| b == id) {
            "backlog"
        } else {
            "done"
        };
        work_units_obj.insert(
            id.to_string(),
            serde_json::json!({
                "id": id,
                "title": id,
                "type": "story",
                "status": status,
            }),
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
    let mut root = serde_json::Map::new();
    if let Some(ts) = meta_last_updated {
        root.insert(
            "meta".to_string(),
            serde_json::json!({"version": "1.0.0", "lastUpdated": ts}),
        );
    } else {
        root.insert(
            "meta".to_string(),
            serde_json::json!({"version": "1.0.0", "lastUpdated": "2026-01-01T00:00:00.000Z"}),
        );
    }
    root.insert("workUnits".to_string(), Value::Object(work_units_obj));
    root.insert("states".to_string(), states);
    fs::write(
        dir.path().join("spec").join("work-units.json"),
        serde_json::to_string_pretty(&Value::Object(root)).unwrap(),
    )
    .expect("write work-units.json");
    dir
}

fn read_column(workspace: &Path, column: &str) -> Vec<String> {
    let body = fs::read_to_string(workspace.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    let v: Value = serde_json::from_str(&body).expect("parse JSON");
    v.get("states")
        .and_then(|s| s.get(column))
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(std::string::ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn read_meta_last_updated(workspace: &Path) -> String {
    let body = fs::read_to_string(workspace.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    let v: Value = serde_json::from_str(&body).expect("parse JSON");
    v.get("meta")
        .and_then(|m| m.get("lastUpdated"))
        .and_then(|s| s.as_str())
        .map(std::string::ToString::to_string)
        .unwrap_or_default()
}

/// Scenario: move_work_unit Up swaps the target with its predecessor inside states[<column>]
#[test]
fn move_work_unit_up_swaps_with_predecessor() {
    // @step Given a workspace whose spec/work-units.json has states.backlog == ["A-001", "B-002", "C-003"]
    let dir = workspace_with(&["A-001", "B-002", "C-003"], &[], None);

    // @step When move_work_unit(cwd, "C-003", Direction::Up) is called
    let result = move_work_unit(dir.path(), "C-003", Direction::Up);

    // @step Then the call returns Ok(())
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    // @step And spec/work-units.json on disk now has states.backlog == ["A-001", "C-003", "B-002"]
    let after = read_column(dir.path(), "backlog");
    assert_eq!(after, vec!["A-001", "C-003", "B-002"]);

    // @step And no other column array in the file is modified
    assert_eq!(read_column(dir.path(), "done"), Vec::<String>::new());
    assert_eq!(read_column(dir.path(), "specifying"), Vec::<String>::new());
}

/// Scenario: move_work_unit Down swaps the target with its successor inside states[<column>]
#[test]
fn move_work_unit_down_swaps_with_successor() {
    // @step Given a workspace whose spec/work-units.json has states.backlog == ["A-001", "B-002", "C-003"]
    let dir = workspace_with(&["A-001", "B-002", "C-003"], &[], None);

    // @step When move_work_unit(cwd, "A-001", Direction::Down) is called
    let result = move_work_unit(dir.path(), "A-001", Direction::Down);

    // @step Then the call returns Ok(())
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    // @step And spec/work-units.json on disk now has states.backlog == ["B-002", "A-001", "C-003"]
    let after = read_column(dir.path(), "backlog");
    assert_eq!(after, vec!["B-002", "A-001", "C-003"]);
}

/// Scenario: move_work_unit Up at the top boundary is a no-op
#[test]
fn move_work_unit_up_at_top_boundary_is_a_noop() {
    // @step Given a workspace whose spec/work-units.json has states.backlog == ["A-001", "B-002", "C-003"]
    let dir = workspace_with(&["A-001", "B-002", "C-003"], &[], None);

    // @step When move_work_unit(cwd, "A-001", Direction::Up) is called
    let result = move_work_unit(dir.path(), "A-001", Direction::Up);

    // @step Then the call returns Ok(())
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    // @step And spec/work-units.json on disk still has states.backlog == ["A-001", "B-002", "C-003"]
    let after = read_column(dir.path(), "backlog");
    assert_eq!(after, vec!["A-001", "B-002", "C-003"]);
}

/// Scenario: move_work_unit Down at the bottom boundary is a no-op
#[test]
fn move_work_unit_down_at_bottom_boundary_is_a_noop() {
    // @step Given a workspace whose spec/work-units.json has states.backlog == ["A-001", "B-002", "C-003"]
    let dir = workspace_with(&["A-001", "B-002", "C-003"], &[], None);

    // @step When move_work_unit(cwd, "C-003", Direction::Down) is called
    let result = move_work_unit(dir.path(), "C-003", Direction::Down);

    // @step Then the call returns Ok(())
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    // @step And spec/work-units.json on disk still has states.backlog == ["A-001", "B-002", "C-003"]
    let after = read_column(dir.path(), "backlog");
    assert_eq!(after, vec!["A-001", "B-002", "C-003"]);
}

/// Scenario: move_work_unit refuses to reorder a done-column unit
#[test]
fn move_work_unit_refuses_to_reorder_a_done_column_unit() {
    // @step Given a workspace whose spec/work-units.json has states.done == ["DONE-001", "DONE-002"]
    let dir = workspace_with(&[], &["DONE-001", "DONE-002"], None);
    let before = read_column(dir.path(), "done");

    // @step When move_work_unit(cwd, "DONE-001", Direction::Down) is called
    let result = move_work_unit(dir.path(), "DONE-001", Direction::Down);

    // @step Then the call returns Err
    assert!(result.is_err(), "expected Err, got {result:?}");

    // @step And the error message contains the substring "done column"
    let msg = format!("{:#}", result.unwrap_err());
    assert!(
        msg.contains("done column"),
        "error message must mention 'done column', got: {msg}"
    );

    // @step And spec/work-units.json on disk is unchanged
    let after = read_column(dir.path(), "done");
    assert_eq!(after, before);
}

/// Scenario: move_work_unit returns Err for an unknown work unit id
#[test]
fn move_work_unit_returns_err_for_unknown_id() {
    // @step Given a workspace whose spec/work-units.json has states.backlog == ["A-001"]
    let dir = workspace_with(&["A-001"], &[], None);

    // @step When move_work_unit(cwd, "MISSING-999", Direction::Up) is called
    let result = move_work_unit(dir.path(), "MISSING-999", Direction::Up);

    // @step Then the call returns Err
    assert!(result.is_err(), "expected Err, got {result:?}");

    // @step And the error message contains the substring "MISSING-999"
    let msg = format!("{:#}", result.unwrap_err());
    assert!(
        msg.contains("MISSING-999"),
        "error message must mention the missing id, got: {msg}"
    );
}

/// Scenario: move_work_unit updates meta.lastUpdated on every persisting write
#[test]
fn move_work_unit_updates_meta_last_updated_on_persist() {
    // @step Given a workspace whose spec/work-units.json has meta.lastUpdated == "2026-01-01T00:00:00.000Z"
    // @step And states.backlog == ["A-001", "B-002"]
    let baseline = "2026-01-01T00:00:00.000Z";
    let dir = workspace_with(&["A-001", "B-002"], &[], Some(baseline));

    // @step When move_work_unit(cwd, "B-002", Direction::Up) is called
    let result = move_work_unit(dir.path(), "B-002", Direction::Up);

    // @step Then the call returns Ok(())
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    // @step And spec/work-units.json on disk has a meta.lastUpdated strictly greater than "2026-01-01T00:00:00.000Z"
    let after = read_meta_last_updated(dir.path());
    assert!(
        after.as_str() > baseline,
        "meta.lastUpdated must be bumped: before={baseline}, after={after}"
    );
}

/// Scenario: Concurrent move_work_unit calls serialize via the inter-process lock
#[test]
fn concurrent_move_work_unit_calls_serialize_via_lock() {
    // @step Given a workspace whose spec/work-units.json has states.backlog == ["A-001", "B-002", "C-003"]
    let dir = workspace_with(&["A-001", "B-002", "C-003"], &[], None);
    let path = dir.path().to_path_buf();

    // @step When two threads call move_work_unit("C-003", Up) and move_work_unit("A-001", Down) concurrently
    let path_a = path.clone();
    let path_b = path.clone();
    let t1 = thread::spawn(move || move_work_unit(&path_a, "C-003", Direction::Up));
    // Tiny stagger so the second thread is likely to hit the lock —
    // not required for correctness, only for the test to exercise the
    // contention path more reliably across platforms.
    thread::sleep(Duration::from_millis(5));
    let t2 = thread::spawn(move || move_work_unit(&path_b, "A-001", Direction::Down));

    let r1 = t1.join().expect("t1 join");
    let r2 = t2.join().expect("t2 join");

    // @step Then both calls return Ok(())
    assert!(r1.is_ok(), "thread 1 expected Ok, got {r1:?}");
    assert!(r2.is_ok(), "thread 2 expected Ok, got {r2:?}");

    // @step And spec/work-units.json on disk is valid JSON
    let body = fs::read_to_string(path.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    let _: Value = serde_json::from_str(&body).expect("file must still parse as JSON");

    // @step And the post-state states.backlog has length 3 and is a permutation of ["A-001", "B-002", "C-003"]
    let mut after = read_column(&path, "backlog");
    after.sort();
    assert_eq!(after, vec!["A-001", "B-002", "C-003"]);
}
