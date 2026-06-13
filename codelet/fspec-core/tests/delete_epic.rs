#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/delete-epic-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `delete-epic`
// (RPC-217). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "delete-epic".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_spec_file(project_root: &Path, name: &str, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join(name), raw).expect("write spec file");
}

fn read_value(project_root: &Path, name: &str) -> Value {
    let raw = fs::read_to_string(project_root.join("spec").join(name)).expect("read spec file");
    serde_json::from_str(&raw).expect("valid JSON")
}

fn epics_with_auth() -> &'static str {
    r#"{
  "epics": {
    "auth": {
      "id": "auth",
      "title": "Authentication",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#
}

fn epics_with_dash() -> &'static str {
    r#"{
  "epics": {
    "dash": {
      "id": "dash",
      "title": "Dashboard",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#
}

fn epics_with_auth_and_dash() -> &'static str {
    r#"{
  "epics": {
    "auth": {
      "id": "auth",
      "title": "Authentication",
      "createdAt": "2026-06-01T00:00:00.000Z"
    },
    "dash": {
      "id": "dash",
      "title": "Dashboard",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#
}

// ---------- scenarios ----------

#[test]
fn dispatcher_deletes_existing_epic_from_epics_json() {
    // Scenario: Dispatcher deletes an existing epic from spec/epics.json

    // @step Given spec/epics.json contains epic 'auth' with title='Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_spec_file(tmp.path(), "epics.json", epics_with_auth());

    // @step When I dispatch delete-epic with epicId='auth'
    let result = dispatch_command(req(tmp.path(), json!({"epicId": "auth"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And spec/epics.json no longer contains an 'auth' epic
    let data = read_value(tmp.path(), "epics.json");
    assert!(
        data["epics"].get("auth").is_none(),
        "auth should be removed; got {data}"
    );
}

#[test]
fn dispatcher_clears_epic_id_references_on_matching_prefixes() {
    // Scenario: Dispatcher clears epicId references on matching prefixes when deleting an epic

    // @step Given spec/epics.json contains epic 'auth' with title='Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_spec_file(tmp.path(), "epics.json", epics_with_auth());

    // @step And spec/prefixes.json contains prefix 'AUTH' with epicId='auth'
    write_spec_file(
        tmp.path(),
        "prefixes.json",
        r#"{
  "prefixes": {
    "AUTH": {
      "prefix": "AUTH",
      "description": "Auth features",
      "epicId": "auth",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#,
    );

    // @step When I dispatch delete-epic with epicId='auth'
    let result = dispatch_command(req(tmp.path(), json!({"epicId": "auth"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the AUTH prefix in spec/prefixes.json no longer has an epicId field
    let data = read_value(tmp.path(), "prefixes.json");
    assert!(
        data["prefixes"]["AUTH"].get("epicId").is_none(),
        "epicId must be stripped; got {data}"
    );
    // AUTH prefix still exists (only the epicId field was removed).
    assert_eq!(data["prefixes"]["AUTH"]["prefix"].as_str(), Some("AUTH"));
}

#[test]
fn dispatcher_clears_epic_references_on_matching_work_units() {
    // Scenario: Dispatcher clears epic references on matching work units when deleting an epic

    // @step Given spec/epics.json contains epic 'auth' with title='Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_spec_file(tmp.path(), "epics.json", epics_with_auth());

    // @step And spec/work-units.json contains work unit AUTH-001 with epic='auth'
    write_spec_file(
        tmp.path(),
        "work-units.json",
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Login",
      "status": "backlog",
      "epic": "auth",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  },
  "states": {
    "backlog": ["AUTH-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I dispatch delete-epic with epicId='auth'
    let result = dispatch_command(req(tmp.path(), json!({"epicId": "auth"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the AUTH-001 work unit in spec/work-units.json no longer has an epic field
    let data = read_value(tmp.path(), "work-units.json");
    assert!(
        data["workUnits"]["AUTH-001"].get("epic").is_none(),
        "epic field must be stripped; got {data}"
    );
    // The work unit itself is preserved.
    assert_eq!(
        data["workUnits"]["AUTH-001"]["title"].as_str(),
        Some("Login")
    );
}

#[test]
fn dispatcher_rejects_deletion_of_missing_epic_with_wrapped_error() {
    // Scenario: Dispatcher rejects deletion of a missing epic with the canonical wrapped error

    // @step Given spec/epics.json contains epic 'dash' with title='Dashboard'
    let tmp = TempDir::new().expect("tempdir");
    write_spec_file(tmp.path(), "epics.json", epics_with_dash());

    // @step When I dispatch delete-epic with epicId='nonexistent'
    let result = dispatch_command(req(tmp.path(), json!({"epicId": "nonexistent"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure: {result:?}");

    // @step And the error message contains the substring 'Failed to delete epic'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Failed to delete epic"),
        "missing wrapper: {msg}"
    );

    // @step And the error message contains the substring 'Epic nonexistent not found'
    assert!(
        msg.contains("Epic nonexistent not found"),
        "missing canonical not-found text: {msg}"
    );

    // @step And spec/epics.json still contains the 'dash' epic
    let data = read_value(tmp.path(), "epics.json");
    assert!(
        data["epics"].get("dash").is_some(),
        "dash must be preserved"
    );
}

#[test]
fn dispatcher_tolerates_missing_prefixes_json_silently() {
    // Scenario: Dispatcher tolerates missing spec/prefixes.json (TS bare-catch parity)

    // @step Given spec/epics.json contains epic 'auth' with title='Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_spec_file(tmp.path(), "epics.json", epics_with_auth());

    // @step And spec/prefixes.json does NOT exist
    assert!(!tmp.path().join("spec/prefixes.json").exists());

    // @step When I dispatch delete-epic with epicId='auth'
    let result = dispatch_command(req(tmp.path(), json!({"epicId": "auth"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/epics.json no longer contains an 'auth' epic
    let data = read_value(tmp.path(), "epics.json");
    assert!(data["epics"].get("auth").is_none());
}

#[test]
fn dispatcher_tolerates_missing_work_units_json_silently() {
    // Scenario: Dispatcher tolerates missing spec/work-units.json (TS bare-catch parity)

    // @step Given spec/epics.json contains epic 'auth' with title='Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_spec_file(tmp.path(), "epics.json", epics_with_auth());

    // @step And spec/work-units.json does NOT exist
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch delete-epic with epicId='auth'
    let result = dispatch_command(req(tmp.path(), json!({"epicId": "auth"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
}

#[test]
fn dispatcher_tolerates_malformed_prefixes_json_silently() {
    // Scenario: Dispatcher tolerates malformed spec/prefixes.json silently

    // @step Given spec/epics.json contains epic 'auth' with title='Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_spec_file(tmp.path(), "epics.json", epics_with_auth());

    // @step And spec/prefixes.json exists with malformed bytes '{ not json'
    write_spec_file(tmp.path(), "prefixes.json", "{ not json");

    // @step When I dispatch delete-epic with epicId='auth'
    let result = dispatch_command(req(tmp.path(), json!({"epicId": "auth"})));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "malformed prefixes.json must be silently swallowed: {result:?}"
    );

    // @step And spec/epics.json no longer contains an 'auth' epic
    let data = read_value(tmp.path(), "epics.json");
    assert!(data["epics"].get("auth").is_none());
}

#[test]
fn dispatcher_tolerates_malformed_work_units_json_silently() {
    // Scenario: Dispatcher tolerates malformed spec/work-units.json silently

    // @step Given spec/epics.json contains epic 'auth' with title='Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_spec_file(tmp.path(), "epics.json", epics_with_auth());

    // @step And spec/work-units.json exists with malformed bytes '{ not json'
    write_spec_file(tmp.path(), "work-units.json", "{ not json");

    // @step When I dispatch delete-epic with epicId='auth'
    let result = dispatch_command(req(tmp.path(), json!({"epicId": "auth"})));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "malformed work-units.json must be silently swallowed: {result:?}"
    );

    // @step And spec/epics.json no longer contains an 'auth' epic
    let data = read_value(tmp.path(), "epics.json");
    assert!(data["epics"].get("auth").is_none());
}

#[test]
fn dispatcher_preserves_non_matching_epics_prefixes_and_work_units() {
    // Scenario: Dispatcher preserves non-matching epics, prefixes, and work units

    // @step Given spec/epics.json contains epics 'auth' and 'dash'
    let tmp = TempDir::new().expect("tempdir");
    write_spec_file(tmp.path(), "epics.json", epics_with_auth_and_dash());

    // @step And spec/prefixes.json contains prefix 'AUTH' with epicId='auth' and prefix 'OTHER' with epicId='dash'
    write_spec_file(
        tmp.path(),
        "prefixes.json",
        r#"{
  "prefixes": {
    "AUTH": {
      "prefix": "AUTH",
      "description": "Auth features",
      "epicId": "auth",
      "createdAt": "x"
    },
    "OTHER": {
      "prefix": "OTHER",
      "description": "Other features",
      "epicId": "dash",
      "createdAt": "x"
    }
  }
}"#,
    );

    // @step And spec/work-units.json contains work unit AUTH-001 with epic='auth' and DASH-001 with epic='dash'
    write_spec_file(
        tmp.path(),
        "work-units.json",
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001", "title": "Auth WU", "status": "backlog",
      "epic": "auth", "createdAt": "x", "updatedAt": "x"
    },
    "DASH-001": {
      "id": "DASH-001", "title": "Dash WU", "status": "backlog",
      "epic": "dash", "createdAt": "x", "updatedAt": "x"
    }
  },
  "states": {
    "backlog": ["AUTH-001", "DASH-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I dispatch delete-epic with epicId='auth'
    let result = dispatch_command(req(tmp.path(), json!({"epicId": "auth"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/epics.json still contains the 'dash' epic
    let epics = read_value(tmp.path(), "epics.json");
    assert!(epics["epics"].get("dash").is_some(), "dash must survive");
    assert!(epics["epics"].get("auth").is_none(), "auth must be gone");

    // @step And the OTHER prefix still has epicId='dash'
    let prefixes = read_value(tmp.path(), "prefixes.json");
    assert_eq!(
        prefixes["prefixes"]["OTHER"]["epicId"].as_str(),
        Some("dash"),
        "OTHER prefix must retain its non-matching epicId"
    );
    // AUTH prefix lost its epicId
    assert!(prefixes["prefixes"]["AUTH"].get("epicId").is_none());

    // @step And the DASH-001 work unit still has epic='dash'
    let wus = read_value(tmp.path(), "work-units.json");
    assert_eq!(
        wus["workUnits"]["DASH-001"]["epic"].as_str(),
        Some("dash"),
        "DASH-001 must retain its non-matching epic"
    );
    // AUTH-001 lost its epic
    assert!(wus["workUnits"]["AUTH-001"].get("epic").is_none());
}

#[test]
fn dispatcher_response_text_renders_canonical_success_line() {
    // Scenario: Dispatcher response text renders the canonical success line

    // @step Given spec/epics.json contains epic 'auth' with title='Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_spec_file(tmp.path(), "epics.json", epics_with_auth());

    // @step When I dispatch delete-epic with epicId='auth'
    let result = dispatch_command(req(tmp.path(), json!({"epicId": "auth"})));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line '✓ Epic auth deleted successfully'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "✓ Epic auth deleted successfully"),
        "missing canonical success line; got:\n{}",
        result.data
    );
}

#[test]
fn dispatcher_fails_fast_when_required_args_are_missing() {
    // Scenario: Dispatcher fails fast when required args are missing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch delete-epic with no epicId field in the args
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected missing-field error: {result:?}");

    // @step And the error message contains the substring 'Invalid args for fspec command delete-epic'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Invalid args for fspec command delete-epic"),
        "missing canonical InvalidArgs prefix: {msg}"
    );
}
