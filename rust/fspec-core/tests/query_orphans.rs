//! Feature: spec/features/query-orphans-rust-port.feature
//!
//! Dispatcher-level tests for `query-orphans` (RPC-262), moved out of
//! rust/fspec/tests/cli_query_orphans.rs so each test file maps one feature (COV-075).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "query-orphans".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

/// Build a work-units.json. `entries` is a slice of (id, status, optional_epic, extra_fields).
fn work_units_with(entries: &[(&str, &str, Option<&str>, Value)]) -> String {
    let mut wus = serde_json::Map::new();
    for (id, status, epic, extra) in entries {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        obj.insert("status".into(), Value::String((*status).to_string()));
        obj.insert("createdAt".into(), Value::String("x".to_string()));
        obj.insert("updatedAt".into(), Value::String("x".to_string()));
        if let Some(e) = epic {
            obj.insert("epic".into(), Value::String((*e).to_string()));
        }
        if let Value::Object(map) = extra {
            for (k, v) in map {
                obj.insert(k.clone(), v.clone());
            }
        }
        wus.insert((*id).to_string(), Value::Object(obj));
    }
    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": {
            "backlog": [], "specifying": [], "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    }))
    .unwrap()
}

// ─────────────────────────────────────────────────────────────────────────
// rust-port scenarios (dispatcher contract)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_empty_workspace_returns_empty_orphans() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the returned JSON has orphans=[]
    let data = parse_data(&result.data);
    assert_eq!(data["orphans"].as_array().map(Vec::len), Some(0));

    // @step And spec/work-units.json exists after the call (auto-created by ensure_work_units_file)
    assert!(tmp.path().join("spec/work-units.json").exists());
}

#[test]
fn scenario_dispatcher_non_blank_epic_not_orphaned() {
    // @step Given spec/work-units.json contains A with epic='auth' and no relationship arrays
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "backlog", Some("auth"), json!({}))]),
    );

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has orphans=[]
    assert_eq!(data["orphans"].as_array().map(Vec::len), Some(0));
}

#[test]
fn scenario_dispatcher_non_empty_blocks_not_orphaned() {
    // @step Given spec/work-units.json contains A with no epic and blocks=['X']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "backlog", None, json!({ "blocks": ["X"] }))]),
    );

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has orphans=[]
    assert_eq!(data["orphans"].as_array().map(Vec::len), Some(0));
}

#[test]
fn scenario_dispatcher_no_epic_no_relations_is_orphaned() {
    // @step Given spec/work-units.json contains A with no epic and no relationship arrays
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "backlog", None, json!({}))]),
    );

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has orphans containing exactly one entry whose id='A'
    let arr = data["orphans"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"].as_str(), Some("A"));

    // @step And that entry has status equal to A's status and suggestedActions=['Assign epic','Add relationship','Delete']
    assert_eq!(arr[0]["status"].as_str(), Some("backlog"));
    let actions: Vec<&str> = arr[0]["suggestedActions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(actions, vec!["Assign epic", "Add relationship", "Delete"]);
}

#[test]
fn scenario_dispatcher_whitespace_epic_is_treated_as_no_epic() {
    // @step Given spec/work-units.json contains A with epic='   ' and no relationship arrays
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "backlog", Some("   "), json!({}))]),
    );

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has orphans containing exactly one entry whose id='A'
    let arr = data["orphans"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"].as_str(), Some("A"));
}

#[test]
fn scenario_dispatcher_empty_relationship_arrays_treated_as_no_relationships() {
    // @step Given spec/work-units.json contains A with no epic and blocks=[] blockedBy=[] dependsOn=[] relatesTo=[]
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[(
            "A",
            "backlog",
            None,
            json!({ "blocks": [], "blockedBy": [], "dependsOn": [], "relatesTo": [] }),
        )]),
    );

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has orphans containing exactly one entry whose id='A'
    let arr = data["orphans"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"].as_str(), Some("A"));
}

#[test]
fn scenario_dispatcher_exclude_done_filters_done_orphans() {
    // @step Given spec/work-units.json contains DONE-1 with status='done' and no epic and no relationships, and OPEN-1 with status='backlog' and no epic and no relationships
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("DONE-1", "done", None, json!({})),
            ("OPEN-1", "backlog", None, json!({})),
        ]),
    );

    // @step When I dispatch query-orphans with output='json' and excludeDone=false
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "output": "json", "excludeDone": false }),
    ));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has orphans containing both DONE-1 and OPEN-1
    let ids: Vec<&str> = data["orphans"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|o| o["id"].as_str())
        .collect();
    assert!(ids.contains(&"DONE-1"));
    assert!(ids.contains(&"OPEN-1"));

    // @step When I dispatch query-orphans with output='json' and excludeDone=true
    let result2 = dispatch_command(req(
        tmp.path(),
        json!({ "output": "json", "excludeDone": true }),
    ));
    let data2 = parse_data(&result2.data);

    // @step Then the returned JSON has orphans containing only OPEN-1
    let ids2: Vec<&str> = data2["orphans"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|o| o["id"].as_str())
        .collect();
    assert_eq!(ids2, vec!["OPEN-1"]);
}

#[test]
fn scenario_dispatcher_orphans_in_insertion_order() {
    // @step Given spec/work-units.json contains FIRST-1 then SECOND-1 then THIRD-1 all with no epic and no relationships
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("FIRST-1", "backlog", None, json!({})),
            ("SECOND-1", "backlog", None, json!({})),
            ("THIRD-1", "backlog", None, json!({})),
        ]),
    );

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));
    let data = parse_data(&result.data);
    let arr = data["orphans"].as_array().expect("array");

    // @step Then orphans[0].id='FIRST-1' and orphans[1].id='SECOND-1' and orphans[2].id='THIRD-1'
    assert_eq!(arr[0]["id"].as_str(), Some("FIRST-1"));
    assert_eq!(arr[1]["id"].as_str(), Some("SECOND-1"));
    assert_eq!(arr[2]["id"].as_str(), Some("THIRD-1"));
}

#[test]
fn scenario_dispatcher_orphan_field_declaration_order() {
    // @step Given spec/work-units.json contains a single orphaned work unit ORPH-1
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("ORPH-1", "backlog", None, json!({}))]),
    );

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the orphan object's field declaration order is id, title, status, suggestedActions
    let raw = &result.data;
    let expected = ["\"id\"", "\"title\"", "\"status\"", "\"suggestedActions\""];
    let mut positions = Vec::new();
    for f in &expected {
        positions.push(raw.find(f).unwrap_or_else(|| panic!("missing {f}\n{raw}")));
    }
    for w in positions.windows(2) {
        assert!(
            w[0] < w[1],
            "field declaration order violated: {positions:?}\nraw:\n{raw}"
        );
    }
}

#[test]
fn scenario_dispatcher_malformed_work_units_json_yields_parse_error() {
    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), "{ not json");

    // @step When I dispatch query-orphans against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
    assert!(!result.success);
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Failed to parse work-units.json"),
        "expected error containing 'Failed to parse work-units.json'; got: {err}"
    );
}
