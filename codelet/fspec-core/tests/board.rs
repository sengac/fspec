#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/board-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `board`
// (RPC-199). Each scenario maps to exactly one #[test] fn with @step
// comments mirroring the Gherkin steps verbatim.
//
// PHASE B (TESTING): the core impl is still a stub, so every dispatch
// returns FspecCoreError::NotYetPorted. These tests are RED until PHASE C.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, format: &str) -> DispatchRequest {
    DispatchRequest {
        command: "board".to_string(),
        args_json: json!({ "format": format }).to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_file(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

fn write_foundation(root: &Path) {
    write_file(root, "spec/foundation.json", "{}\n");
}

fn parse_data(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{}", result.data))
}

// ---------- scenarios ----------

#[test]
fn sums_story_points_and_builds_done_column_entry_with_estimate() {
    // Scenario: Sums story points and builds done column entry with estimate

    // @step Given spec/foundation.json exists
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());

    // @step Given spec/work-units.json contains AUTH-001 (done, estimate 5) and AUTH-002 (implementing, estimate 3)
    write_file(
        tmp.path(),
        "spec/work-units.json",
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "Login", "status": "done", "estimate": 5, "createdAt": "x", "updatedAt": "x" },
    "AUTH-002": { "id": "AUTH-002", "title": "Logout", "status": "implementing", "estimate": 3, "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": [], "specifying": [], "testing": [],
    "implementing": ["AUTH-002"], "validating": [],
    "done": ["AUTH-001"], "blocked": []
  }
}"#,
    );

    // @step When I dispatch the board command against that project root with format='json'
    let result = dispatch_command(req(tmp.path(), "json"));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result);

    // @step Then the summary field reads exactly '3 points in progress, 5 points completed'
    assert_eq!(
        data["summary"].as_str(),
        Some("3 points in progress, 5 points completed")
    );

    // @step Then the columns.done array first entry has id 'AUTH-001' and estimate 5
    let done0 = &data["columns"]["done"][0];
    assert_eq!(done0["id"].as_str(), Some("AUTH-001"));
    assert_eq!(done0["estimate"].as_i64(), Some(5));
}

#[test]
fn omits_estimate_key_and_contributes_zero_points_for_an_unestimated_unit() {
    // Scenario: Omits the estimate key and contributes zero points for an unestimated unit

    // @step Given spec/foundation.json exists
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());

    // @step Given spec/work-units.json contains AUTH-001 (backlog) with no estimate field
    write_file(
        tmp.path(),
        "spec/work-units.json",
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "Login", "status": "backlog", "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": ["AUTH-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I dispatch the board command against that project root with format='json'
    let result = dispatch_command(req(tmp.path(), "json"));
    let data = parse_data(&result);

    // @step Then the columns.backlog array first entry has no estimate key
    let backlog0 = &data["columns"]["backlog"][0];
    assert_eq!(backlog0["id"].as_str(), Some("AUTH-001"));
    assert!(
        backlog0.get("estimate").is_none(),
        "estimate key must be omitted; got {backlog0}"
    );

    // @step Then the summary field reads exactly '0 points in progress, 0 points completed'
    assert_eq!(
        data["summary"].as_str(),
        Some("0 points in progress, 0 points completed")
    );
}

#[test]
fn fails_with_foundation_missing_error_when_foundation_is_absent() {
    // Scenario: Fails with the foundation-missing error when foundation.json is absent

    // @step Given a project root with no spec/foundation.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the board command against that project root with format='json'
    let result = dispatch_command(req(tmp.path(), "json"));

    // @step Then the dispatcher returns success=false with an error message describing the missing foundation
    assert!(!result.success, "expected success=false, got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("foundation") || err.contains("Foundation"),
        "error must describe the missing foundation; got: {err}"
    );
}

#[test]
fn auto_creates_work_units_json_and_renders_all_seven_empty_columns() {
    // Scenario: Auto-creates work-units.json and renders all seven empty columns

    // @step Given spec/foundation.json exists
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());

    // @step Given spec/work-units.json does NOT exist
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch the board command against that project root with format='json'
    let result = dispatch_command(req(tmp.path(), "json"));
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result);

    // @step Then spec/work-units.json exists after the call
    assert!(
        tmp.path().join("spec/work-units.json").exists(),
        "work-units.json must be auto-created"
    );

    // @step Then the columns object contains keys backlog, specifying, testing, implementing, validating, done, blocked
    let columns = &data["columns"];
    for key in [
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        assert!(
            columns.get(key).is_some(),
            "columns must contain key '{key}'; got {columns}"
        );
    }

    // @step Then the summary field reads exactly '0 points in progress, 0 points completed'
    assert_eq!(
        data["summary"].as_str(),
        Some("0 points in progress, 0 points completed")
    );
}
