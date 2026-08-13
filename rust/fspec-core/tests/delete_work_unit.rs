#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/delete-work-unit-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `delete-work-unit`
// (RPC-223). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "delete-work-unit".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("valid JSON")
}

/// A single leaf work unit AUTH-001 in backlog with no dependencies.
fn wu_leaf_auth_001() -> &'static str {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001", "title": "Login", "status": "backlog",
      "createdAt": "x", "updatedAt": "x"
    }
  },
  "states": {
    "backlog": ["AUTH-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#
}

// ---------- scenarios ----------

#[test]
fn dispatcher_deletes_existing_leaf_work_unit() {
    // Scenario: Dispatcher deletes an existing leaf work unit with no dependencies

    // @step Given spec/work-units.json contains work unit AUTH-001 with status='backlog' and no dependencies
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), wu_leaf_auth_001());

    // @step When I dispatch delete-work-unit with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the DispatchResult.data contains the line '✓ Work unit AUTH-001 deleted successfully'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "✓ Work unit AUTH-001 deleted successfully"),
        "missing canonical success line; got:\n{}",
        result.data
    );

    // @step And spec/work-units.json no longer contains the AUTH-001 work unit
    let data = read_work_units(tmp.path());
    assert!(
        data["workUnits"].get("AUTH-001").is_none(),
        "AUTH-001 should be removed; got {data}"
    );
}

#[test]
fn dispatcher_rejects_deletion_of_missing_work_unit() {
    // Scenario: Dispatcher rejects deletion of a missing work unit

    // @step Given spec/work-units.json contains work unit AUTH-001 with status='backlog' and no dependencies
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), wu_leaf_auth_001());

    // @step When I dispatch delete-work-unit with workUnitId='MISSING-999'
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "MISSING-999"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Work unit 'MISSING-999' does not exist"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Work unit 'MISSING-999' does not exist"),
        "missing canonical not-found text: {msg}"
    );

    // @step And spec/work-units.json still contains the AUTH-001 work unit
    let data = read_work_units(tmp.path());
    assert!(
        data["workUnits"].get("AUTH-001").is_some(),
        "AUTH-001 must be preserved"
    );
}

#[test]
fn dispatcher_refuses_to_delete_work_unit_with_children() {
    // Scenario: Dispatcher refuses to delete a work unit that has children

    // @step Given spec/work-units.json contains work unit AUTH-999 with children AUTH-002 and AUTH-003
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-999": {
      "id": "AUTH-999", "title": "Parent", "status": "backlog",
      "children": ["AUTH-002", "AUTH-003"],
      "createdAt": "x", "updatedAt": "x"
    }
  },
  "states": {
    "backlog": ["AUTH-999"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I dispatch delete-work-unit with workUnitId='AUTH-999'
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-999"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring 'Cannot delete work unit with children: AUTH-002, AUTH-003. Delete children first or remove parent relationship.'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Cannot delete work unit with children: AUTH-002, AUTH-003. Delete children first or remove parent relationship."),
        "missing canonical children-block text: {msg}"
    );

    // @step And spec/work-units.json still contains the AUTH-999 work unit
    let data = read_work_units(tmp.path());
    assert!(
        data["workUnits"].get("AUTH-999").is_some(),
        "AUTH-999 must be preserved"
    );
}

#[test]
fn dispatcher_refuses_to_delete_with_dependencies_without_cascade() {
    // Scenario: Dispatcher refuses to delete a work unit with dependencies without cascade

    // @step Given spec/work-units.json contains work unit AUTH-001 with dependsOn AUTH-000
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001", "title": "Login", "status": "backlog",
      "dependsOn": ["AUTH-000"],
      "createdAt": "x", "updatedAt": "x"
    }
  },
  "states": {
    "backlog": ["AUTH-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I dispatch delete-work-unit with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Work unit 'AUTH-001' has dependencies. Use --cascade-dependencies flag to remove dependencies and delete."
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Work unit 'AUTH-001' has dependencies. Use --cascade-dependencies flag to remove dependencies and delete."),
        "missing canonical dependency-block text: {msg}"
    );

    // @step And spec/work-units.json still contains the AUTH-001 work unit
    let data = read_work_units(tmp.path());
    assert!(
        data["workUnits"].get("AUTH-001").is_some(),
        "AUTH-001 must be preserved"
    );
}

#[test]
fn dispatcher_cascades_blocks_references_and_emits_warning() {
    // Scenario: Dispatcher cascades blocks references and emits a blocks warning

    // @step Given spec/work-units.json contains work unit AUTH-001 with blocks API-001 and work unit API-001 with blockedBy AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001", "title": "Login", "status": "backlog",
      "blocks": ["API-001"],
      "createdAt": "x", "updatedAt": "x"
    },
    "API-001": {
      "id": "API-001", "title": "API", "status": "backlog",
      "blockedBy": ["AUTH-001"],
      "createdAt": "x", "updatedAt": "x"
    }
  },
  "states": {
    "backlog": ["AUTH-001", "API-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I dispatch delete-work-unit with workUnitId='AUTH-001' and cascadeDependencies=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "cascadeDependencies": true}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the API-001 work unit in spec/work-units.json no longer lists AUTH-001 in its blockedBy
    let data = read_work_units(tmp.path());
    let api_blocked_by = data["workUnits"]["API-001"].get("blockedBy");
    let still_listed = api_blocked_by
        .and_then(Value::as_array)
        .map(|a| a.iter().any(|v| v.as_str() == Some("AUTH-001")))
        .unwrap_or(false);
    assert!(
        !still_listed,
        "API-001.blockedBy must no longer list AUTH-001; got {data}"
    );

    // @step And the DispatchResult.data contains the substring '⚠ This work unit blocks 1 work unit(s): API-001'
    assert!(
        result
            .data
            .contains("⚠ This work unit blocks 1 work unit(s): API-001"),
        "missing blocks warning; got:\n{}",
        result.data
    );

    // @step And spec/work-units.json no longer contains the AUTH-001 work unit
    assert!(
        data["workUnits"].get("AUTH-001").is_none(),
        "AUTH-001 should be removed; got {data}"
    );
}

#[test]
fn dispatcher_does_not_cascade_depends_on_references() {
    // Scenario: Dispatcher does NOT cascade dependsOn references

    // @step Given spec/work-units.json contains work unit AUTH-001 with dependsOn AUTH-000 and work unit AUTH-000 with no references
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001", "title": "Login", "status": "backlog",
      "dependsOn": ["AUTH-000"],
      "createdAt": "x", "updatedAt": "x"
    },
    "AUTH-000": {
      "id": "AUTH-000", "title": "Base", "status": "backlog",
      "createdAt": "x", "updatedAt": "x"
    }
  },
  "states": {
    "backlog": ["AUTH-001", "AUTH-000"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );
    let before = read_work_units(tmp.path());
    let auth_000_before = before["workUnits"]["AUTH-000"].clone();

    // @step When I dispatch delete-work-unit with workUnitId='AUTH-001' and cascadeDependencies=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "cascadeDependencies": true}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the AUTH-000 work unit in spec/work-units.json is unchanged
    let after = read_work_units(tmp.path());
    assert_eq!(
        after["workUnits"]["AUTH-000"], auth_000_before,
        "AUTH-000 must be untouched (dependsOn is not cascaded)"
    );

    // @step And spec/work-units.json no longer contains the AUTH-001 work unit
    assert!(
        after["workUnits"].get("AUTH-001").is_none(),
        "AUTH-001 should be removed; got {after}"
    );
}

#[test]
fn dispatcher_removes_unit_from_parent_children() {
    // Scenario: Dispatcher removes the unit from its parent's children array

    // @step Given spec/work-units.json contains work unit AUTH-PARENT with children AUTH-CHILD and work unit AUTH-CHILD with parent AUTH-PARENT
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-PARENT": {
      "id": "AUTH-PARENT", "title": "Parent", "status": "backlog",
      "children": ["AUTH-CHILD"],
      "createdAt": "x", "updatedAt": "x"
    },
    "AUTH-CHILD": {
      "id": "AUTH-CHILD", "title": "Child", "status": "backlog",
      "parent": "AUTH-PARENT",
      "createdAt": "x", "updatedAt": "x"
    }
  },
  "states": {
    "backlog": ["AUTH-PARENT", "AUTH-CHILD"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I dispatch delete-work-unit with workUnitId='AUTH-CHILD'
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-CHILD"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the AUTH-PARENT work unit in spec/work-units.json no longer lists AUTH-CHILD in its children
    let data = read_work_units(tmp.path());
    let still_child = data["workUnits"]["AUTH-PARENT"]
        .get("children")
        .and_then(Value::as_array)
        .map(|a| a.iter().any(|v| v.as_str() == Some("AUTH-CHILD")))
        .unwrap_or(false);
    assert!(
        !still_child,
        "AUTH-PARENT.children must no longer list AUTH-CHILD; got {data}"
    );

    // @step And spec/work-units.json no longer contains the AUTH-CHILD work unit
    assert!(
        data["workUnits"].get("AUTH-CHILD").is_none(),
        "AUTH-CHILD should be removed; got {data}"
    );
}

#[test]
fn dispatcher_removes_unit_from_state_index() {
    // Scenario: Dispatcher removes the unit from its state index array

    // @step Given spec/work-units.json contains work unit AUTH-001 with status='specifying' listed in states.specifying
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001", "title": "Login", "status": "specifying",
      "createdAt": "x", "updatedAt": "x"
    }
  },
  "states": {
    "backlog": [], "specifying": ["AUTH-001"], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I dispatch delete-work-unit with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And states.specifying in spec/work-units.json no longer contains AUTH-001
    let data = read_work_units(tmp.path());
    let still_listed = data["states"]["specifying"]
        .as_array()
        .map(|a| a.iter().any(|v| v.as_str() == Some("AUTH-001")))
        .unwrap_or(false);
    assert!(
        !still_listed,
        "states.specifying must no longer list AUTH-001; got {data}"
    );
}

#[test]
fn dispatcher_fails_fast_when_required_args_are_missing() {
    // Scenario: Dispatcher fails fast when required args are missing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch delete-work-unit with no workUnitId field in the args
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected missing-field error: {result:?}");

    // @step And the error message contains the substring 'Invalid args for fspec command delete-work-unit'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Invalid args for fspec command delete-work-unit"),
        "missing canonical InvalidArgs prefix: {msg}"
    );
}
