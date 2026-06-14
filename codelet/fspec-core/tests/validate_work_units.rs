#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/validate-work-units-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `validate-work-units`
// (RPC-325). Each scenario maps to exactly one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim.
//
// PHASE B (TESTING): the core impl is still a stub, so every dispatch returns
// FspecCoreError::NotYetPorted. These tests are RED until PHASE C.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "validate-work-units".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_raw_work_units(root: &Path, raw: &str) {
    let spec = root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn write_work_units(root: &Path, value: &Value) {
    write_raw_work_units(root, &serde_json::to_string_pretty(value).unwrap());
}

fn parse_data(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{}", result.data))
}

fn errors(data: &Value) -> Vec<String> {
    data["errors"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn has_error(data: &Value, needle: &str) -> bool {
    errors(data).iter().any(|e| e.contains(needle))
}

fn has_exact_error(data: &Value, exact: &str) -> bool {
    errors(data).iter().any(|e| e == exact)
}

// ---------- scenarios ----------

#[test]
fn dispatcher_reports_a_clean_store_as_valid() {
    // Scenario: Dispatcher reports a clean store as valid

    // @step Given spec/work-units.json contains consistent work units with matching parent/child links, valid statuses, and correct state arrays
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "Parent", "status": "done", "children": ["AUTH-002"] },
                "AUTH-002": { "id": "AUTH-002", "title": "Child", "status": "backlog", "parent": "AUTH-001" }
            },
            "states": {
                "backlog": ["AUTH-002"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": ["AUTH-001"], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=true with no errors
    assert_eq!(data["valid"].as_bool(), Some(true), "got {data}");
    assert!(errors(&data).is_empty(), "expected no errors; got {data}");

    // @step Then the result checks list includes schema, uniqueIds, parentChild, exampleMapping, and dependencies
    let checks: Vec<String> = data["checks"]
        .as_array()
        .expect("checks array")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    for expected in ["schema", "uniqueIds", "parentChild", "exampleMapping", "dependencies"] {
        assert!(checks.contains(&expected.to_string()), "checks missing {expected}: {data}");
    }
}

#[test]
fn dispatcher_flags_a_work_unit_whose_parent_is_missing() {
    // Scenario: Dispatcher reproduces the TS crash for a work unit with a
    // missing parent.
    //
    // The TypeScript reference pushes a "references non-existent parent" error
    // but does NOT `continue`; it then dereferences
    // `workUnitsData.workUnits[parent].children` on the `undefined` parent,
    // throwing a TypeError. The `.action` catch renders this as
    // `✗ Failed to validate work units: Cannot read properties of undefined
    // (reading 'children')` (exit 1). Parity requires the dispatcher to surface
    // that exact message as a failure rather than the structured error.

    // @step Given spec/work-units.json contains AUTH-002 with parent AUTH-001 but no AUTH-001 entry
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-002": { "id": "AUTH-002", "title": "Child", "status": "backlog", "parent": "AUTH-001" }
            },
            "states": {
                "backlog": ["AUTH-002"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns failure carrying the V8 TypeError message
    assert!(!result.success, "{result:?}");
    assert_eq!(
        result.error.as_deref(),
        Some("Cannot read properties of undefined (reading 'children')"),
        "{result:?}"
    );
}

#[test]
fn dispatcher_flags_a_parent_that_does_not_list_its_child() {
    // Scenario: Dispatcher flags a parent that does not list its child

    // @step Given spec/work-units.json contains AUTH-001 with no children and AUTH-002 whose parent is AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "Parent", "status": "done" },
                "AUTH-002": { "id": "AUTH-002", "title": "Child", "status": "backlog", "parent": "AUTH-001" }
            },
            "states": {
                "backlog": ["AUTH-002"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": ["AUTH-001"], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=false
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");

    // @step Then the errors include "Work unit AUTH-002 has parent AUTH-001, but parent doesn't list it as a child"
    assert!(
        has_exact_error(
            &data,
            "Work unit AUTH-002 has parent AUTH-001, but parent doesn't list it as a child"
        ),
        "got {data}"
    );
}

#[test]
fn dispatcher_flags_a_missing_child_reference() {
    // Scenario: Dispatcher flags a missing child reference

    // @step Given spec/work-units.json contains AUTH-001 whose children list AUTH-099 which does not exist
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "Parent", "status": "done", "children": ["AUTH-099"] }
            },
            "states": {
                "backlog": [], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": ["AUTH-001"], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=false
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");

    // @step Then the errors include 'Work unit AUTH-001 references non-existent child: AUTH-099'
    assert!(
        has_exact_error(&data, "Work unit AUTH-001 references non-existent child: AUTH-099"),
        "got {data}"
    );
}

#[test]
fn dispatcher_flags_an_invalid_status_value() {
    // Scenario: Dispatcher flags an invalid status value

    // @step Given spec/work-units.json contains AUTH-001 with status 'review'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "X", "status": "review" }
            },
            "states": {
                "backlog": [], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=false
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");

    // @step Then the errors include a message starting with 'Invalid status value for AUTH-001: review'
    assert!(
        errors(&data)
            .iter()
            .any(|e| e.starts_with("Invalid status value for AUTH-001: review")),
        "got {data}"
    );
}

#[test]
fn dispatcher_flags_a_work_unit_absent_from_its_state_array() {
    // Scenario: Dispatcher flags a work unit absent from its state array

    // @step Given spec/work-units.json contains AUTH-001 with status 'done' but states.done does not include AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "X", "status": "done" }
            },
            "states": {
                "backlog": [], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=false
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");

    // @step Then the errors include a message containing 'has status' and 'is not in states.done array'
    assert!(
        errors(&data)
            .iter()
            .any(|e| e.contains("has status") && e.contains("is not in states.done array")),
        "got {data}"
    );
}

#[test]
fn dispatcher_flags_a_work_unit_listed_in_the_wrong_state_array() {
    // Scenario: Dispatcher flags a work unit listed in the wrong state array

    // @step Given spec/work-units.json contains AUTH-001 with status 'done' that also appears in states.backlog
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "X", "status": "done" }
            },
            "states": {
                "backlog": ["AUTH-001"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": ["AUTH-001"], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=false
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");

    // @step Then the errors include a message containing "is in 'backlog' array"
    assert!(has_error(&data, "is in 'backlog' array"), "got {data}");
}

#[test]
fn dispatcher_flags_an_empty_string_in_the_rules_array() {
    // Scenario: Dispatcher flags an empty string in the rules array

    // @step Given spec/work-units.json contains AUTH-001 whose rules array contains an empty string at index 0
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "X", "status": "backlog", "rules": [""] }
            },
            "states": {
                "backlog": ["AUTH-001"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=false
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");

    // @step Then the errors include 'AUTH-001: rules array contains empty strings or non-strings at index 0'
    assert!(
        has_exact_error(
            &data,
            "Work unit AUTH-001: rules array contains empty strings or non-strings at index 0"
        ),
        "got {data}"
    );
}

#[test]
fn dispatcher_flags_a_malformed_questions_item() {
    // Scenario: Dispatcher flags a malformed questions item

    // @step Given spec/work-units.json contains AUTH-001 whose questions array contains a string instead of an object
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "X", "status": "backlog", "questions": ["not an object"] }
            },
            "states": {
                "backlog": ["AUTH-001"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=false
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");

    // @step Then the errors include a message containing 'questions[0] must be a QuestionItem object'
    assert!(
        has_error(&data, "questions[0] must be a QuestionItem object"),
        "got {data}"
    );
}

#[test]
fn dispatcher_flags_a_non_array_dependency_field() {
    // Scenario: Dispatcher flags a non-array dependency field

    // @step Given spec/work-units.json contains AUTH-001 whose blockedBy field is a string instead of an array
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "X", "status": "backlog", "blockedBy": "AUTH-002" }
            },
            "states": {
                "backlog": ["AUTH-001"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=false
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");

    // @step Then the errors include 'AUTH-001: blockedBy must be an array'
    assert!(
        has_exact_error(&data, "Work unit AUTH-001: blockedBy must be an array"),
        "got {data}"
    );
}

#[test]
fn dispatcher_reports_schema_errors_for_a_missing_work_units_field() {
    // Scenario: Dispatcher reproduces the TS crash for a missing workUnits field
    //
    // The TypeScript reference does NOT short-circuit on the schema check; its
    // Check 2 calls `Object.keys(workUnitsData.workUnits)`, which throws a
    // TypeError when `workUnits` is absent. The `.action` catch renders this as
    // `✗ Failed to validate work units: Cannot convert undefined or null to
    // object` (exit 1). Parity requires the dispatcher to surface that exact
    // message as a failure rather than a structured "missing workUnits" error.

    // @step Given spec/work-units.json contains a states object but no workUnits field
    let tmp = TempDir::new().expect("tempdir");
    write_raw_work_units(
        tmp.path(),
        r#"{ "version": "0.7.1", "states": { "backlog": [], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] } }"#,
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns failure carrying the V8 TypeError message
    assert!(!result.success, "{result:?}");
    assert_eq!(
        result.error.as_deref(),
        Some("Cannot convert undefined or null to object"),
        "{result:?}"
    );
}

#[test]
fn dispatcher_auto_creates_and_validates_an_empty_store_when_missing() {
    // Scenario: Dispatcher auto-creates and validates an empty store when work-units.json is missing

    // @step Given an empty project root with no spec/work-units.json
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=true with no errors
    assert_eq!(data["valid"].as_bool(), Some(true), "got {data}");
    assert!(errors(&data).is_empty(), "expected no errors; got {data}");
}
