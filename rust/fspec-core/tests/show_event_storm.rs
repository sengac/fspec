//! Feature: spec/features/show-event-storm-rust-port.feature
//!
//! Dispatcher-level tests for `show-event-storm` (RPC-303), moved out of
//! rust/fspec/tests/cli_show_event_storm.rs so each test file maps one feature (COV-075).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn dispatch_ses(project_root: &Path, args_json: &str) -> codelet_fspec_core::DispatchResult {
    let req = codelet_fspec_core::DispatchRequest {
        command: "show-event-storm".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    };
    codelet_fspec_core::dispatch_command(req)
}

// =========================================================================
// Scenarios from show-event-storm-rust-port.feature (dispatcher path)
// =========================================================================

#[test]
fn scenario_returns_work_unit_not_found_when_empty_workspace() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 not found'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Work unit AUTH-001 not found"),
        "error must contain canonical message; got {result:?}"
    );

    // @step Then spec/work-units.json exists after the call (auto-created by ensure_work_units_file)
    assert!(
        ws.path().join("spec/work-units.json").exists(),
        "spec/work-units.json must be auto-created"
    );
}

#[test]
fn scenario_returns_work_unit_not_found_when_id_missing() {
    // @step Given spec/work-units.json contains BUG-001 but not AUTH-001
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "BUG-001": {"id":"BUG-001","title":"B","status":"backlog","createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":["BUG-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 not found'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Work unit AUTH-001 not found"),
        "error must contain canonical message; got {result:?}"
    );
}

#[test]
fn scenario_returns_no_event_storm_data_when_no_event_storm_field() {
    // @step Given spec/work-units.json contains AUTH-001 with no eventStorm field
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 has no Event Storm data'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Work unit AUTH-001 has no Event Storm data"),
        "error must contain canonical message; got {result:?}"
    );
}

#[test]
fn scenario_returns_no_event_storm_data_when_no_items_array() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm={} (no items field)
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x","eventStorm":{}}
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 has no Event Storm data'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Work unit AUTH-001 has no Event Storm data"),
        "error must contain canonical message; got {result:?}"
    );
}

#[test]
fn scenario_returns_active_items_as_pretty_printed_json_array() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0, deleted=false), command(id=1, deleted=false)]
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x",
      "eventStorm":{"items":[
        {"id":0,"type":"event","text":"x","deleted":false},
        {"id":1,"type":"command","text":"y","deleted":false}
      ]}
    }
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data parses as a JSON array of length 2
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed.as_array().map(Vec::len), Some(2));

    // @step Then the parsed array[0] has id=0 and type='event'
    assert_eq!(parsed[0]["id"].as_u64(), Some(0));
    assert_eq!(parsed[0]["type"].as_str(), Some("event"));

    // @step Then the parsed array[1] has id=1 and type='command'
    assert_eq!(parsed[1]["id"].as_u64(), Some(1));
    assert_eq!(parsed[1]["type"].as_str(), Some("command"));

    // @step Then the DispatchResult.data uses 2-space indentation
    assert!(
        result.data.contains("\n  "),
        "data must use 2-space indentation; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_filters_out_soft_deleted_items() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0, deleted=false), event(id=1, deleted=true), command(id=2, deleted=false)]
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x",
      "eventStorm":{"items":[
        {"id":0,"type":"event","text":"x","deleted":false},
        {"id":1,"type":"event","text":"y","deleted":true},
        {"id":2,"type":"command","text":"z","deleted":false}
      ]}
    }
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data parses as a JSON array of length 2
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed.as_array().map(Vec::len), Some(2));

    // @step Then the parsed array[0] has id=0
    assert_eq!(parsed[0]["id"].as_u64(), Some(0));

    // @step Then the parsed array[1] has id=2
    assert_eq!(parsed[1]["id"].as_u64(), Some(2));
}

#[test]
fn scenario_treats_missing_deleted_field_as_retained() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0) (no deleted field)]
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x",
      "eventStorm":{"items":[{"id":0,"type":"event","text":"x"}]}
    }
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data parses as a JSON array of length 1
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed.as_array().map(Vec::len), Some(1));
}

#[test]
fn scenario_returns_empty_array_when_event_storm_items_empty() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm.items=[]
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x",
      "eventStorm":{"items":[]}
    }
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data parses as the empty JSON array
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed.as_array().map(Vec::len), Some(0));
}

#[test]
fn scenario_preserves_every_field_on_every_retained_item() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm.items=[policy(id=0, deleted=false, when='UserRegistered', then='SendWelcomeEmail', color='purple', type='policy', text='Send welcome email')]
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x",
      "eventStorm":{"items":[
        {"id":0,"type":"policy","text":"Send welcome email","when":"UserRegistered","then":"SendWelcomeEmail","color":"purple","deleted":false}
      ]}
    }
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001'
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the parsed array[0] has type='policy', text='Send welcome email', when='UserRegistered', then='SendWelcomeEmail', color='purple'
    assert_eq!(parsed[0]["type"].as_str(), Some("policy"));
    assert_eq!(parsed[0]["text"].as_str(), Some("Send welcome email"));
    assert_eq!(parsed[0]["when"].as_str(), Some("UserRegistered"));
    assert_eq!(parsed[0]["then"].as_str(), Some("SendWelcomeEmail"));
    assert_eq!(parsed[0]["color"].as_str(), Some("purple"));
}

#[test]
fn scenario_escalates_malformed_work_units_dispatcher() {
    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), "{ not json");

    // @step When I dispatch show-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_ses(ws.path(), r#"{"workUnitId":"AUTH-001"}"#);

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_ref()
            .map(|e| e.contains("Failed to parse work-units.json"))
            .unwrap_or(false),
        "error must mention parse failure; got {result:?}"
    );
}

#[test]
fn scenario_missing_work_unit_id_surfaces_invalid_args_error() {
    // @step Given spec/work-units.json contains AUTH-001 with eventStorm.items=[event(id=0)]
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x",
      "eventStorm":{"items":[{"id":0,"type":"event","text":"x"}]}
    }
  },
  "states": {"backlog":["AUTH-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch show-event-storm with no workUnitId argument
    let result = dispatch_ses(ws.path(), r#"{}"#);

    // @step Then the dispatcher returns success=false with an error message containing the substring 'failed to parse args'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_ref()
            .map(|e| e.to_lowercase().contains("failed to parse args"))
            .unwrap_or(false),
        "error must mention args parse failure; got {result:?}"
    );
}
