#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/create-epic-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `create-epic`
// (RPC-211). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "create-epic".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_epics(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("epics.json"), raw).expect("write epics.json");
}

fn read_epics_value(project_root: &Path) -> Value {
    let raw =
        fs::read_to_string(project_root.join("spec/epics.json")).expect("read spec/epics.json");
    serde_json::from_str(&raw).expect("epics.json is valid JSON")
}

fn read_epics_raw(project_root: &Path) -> String {
    fs::read_to_string(project_root.join("spec/epics.json")).expect("read spec/epics.json")
}

// ---------- scenarios ----------

#[test]
fn dispatcher_creates_a_minimal_epic_and_writes_epics_json() {
    // Scenario: Dispatcher creates a minimal epic and writes spec/epics.json

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch create-epic with epicId='auth' and title='Authentication'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"epicId": "auth", "title": "Authentication"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/epics.json exists and contains an epic 'auth' with id='auth', title='Authentication', and a non-empty createdAt string
    assert!(tmp.path().join("spec/epics.json").exists());
    let data = read_epics_value(tmp.path());
    let auth = &data["epics"]["auth"];
    assert_eq!(auth["id"].as_str(), Some("auth"));
    assert_eq!(auth["title"].as_str(), Some("Authentication"));
    let created_at = auth["createdAt"]
        .as_str()
        .expect("createdAt should be present");
    assert!(
        !created_at.is_empty(),
        "createdAt must be non-empty, got {created_at:?}"
    );

    // @step And the epic record does NOT contain a 'description' key
    assert!(
        auth.get("description").is_none(),
        "description key must be omitted when not provided, got {auth}"
    );
}

#[test]
fn dispatcher_creates_an_epic_with_a_description() {
    // Scenario: Dispatcher creates an epic with a description

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch create-epic with epicId='auth', title='Authentication', and description='Login features'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "epicId": "auth",
            "title": "Authentication",
            "description": "Login features"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/epics.json contains epic 'auth' with description='Login features'
    let data = read_epics_value(tmp.path());
    assert_eq!(
        data["epics"]["auth"]["description"].as_str(),
        Some("Login features")
    );

    // @step And in the on-disk JSON the 'createdAt' key appears before the 'description' key
    // Parity with TS `src/commands/create-epic.ts:56-64`: the literal
    // object is built `{ id, title, createdAt }` first and `description`
    // is appended ONLY when supplied, so `JSON.stringify`'s insertion
    // order puts description AFTER createdAt.
    let raw = read_epics_raw(tmp.path());
    let desc_idx = raw
        .find("\"description\"")
        .expect("'description' key must exist in raw JSON");
    let created_idx = raw
        .find("\"createdAt\"")
        .expect("'createdAt' key must exist in raw JSON");
    assert!(
        created_idx < desc_idx,
        "createdAt ({created_idx}) must appear before description ({desc_idx}) in:\n{raw}"
    );
}

#[test]
fn dispatcher_preserves_pre_existing_epics_when_adding_a_new_one() {
    // Scenario: Dispatcher preserves pre-existing epics when adding a new one

    // @step Given spec/epics.json contains epic 'dash' with title='Dashboard'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(
        tmp.path(),
        r#"{
  "epics": {
    "dash": {
      "id": "dash",
      "title": "Dashboard",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#,
    );

    // @step When I dispatch create-epic with epicId='auth' and title='Authentication'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"epicId": "auth", "title": "Authentication"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/epics.json contains both 'dash' and 'auth' epics
    let data = read_epics_value(tmp.path());
    assert!(data["epics"].get("dash").is_some(), "dash must survive");
    assert!(data["epics"].get("auth").is_some(), "auth must be added");

    // @step And the existing 'dash' epic still has title='Dashboard'
    assert_eq!(data["epics"]["dash"]["title"].as_str(), Some("Dashboard"));
}

#[test]
fn dispatcher_rejects_invalid_epic_id_with_canonical_regex_error() {
    // Scenario: Dispatcher rejects an invalid epicId with the canonical regex error

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch create-epic with epicId='AUTH' and title='Authentication'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"epicId": "AUTH", "title": "Authentication"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step And the error message does NOT contain the substring 'Failed to create epic'
    // Per TS parity (`src/commands/create-epic.ts:31-35`), the
    // id-format validation throws OUTSIDE the outer try/catch, so the
    // wrap `"Failed to create epic: "` is NOT applied — the raw
    // validator string is what the dispatcher sees.
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        !msg.contains("Failed to create epic"),
        "id-format error must NOT include the outer-catch wrap; got: {msg}"
    );

    // @step And the error message contains the substring 'lowercase-with-hyphens format'
    assert!(
        msg.contains("lowercase-with-hyphens format"),
        "error missing regex hint: {msg}"
    );

    // @step And spec/epics.json does NOT exist
    assert!(
        !tmp.path().join("spec/epics.json").exists(),
        "epics.json must not be written when validation fails"
    );
}

#[test]
fn dispatcher_rejects_creating_an_epic_that_already_exists() {
    // Scenario: Dispatcher rejects creating an epic that already exists

    // @step Given spec/epics.json contains epic 'auth' with title='Old Title'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(
        tmp.path(),
        r#"{
  "epics": {
    "auth": {
      "id": "auth",
      "title": "Old Title",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#,
    );

    // @step When I dispatch create-epic with epicId='auth' and title='New Title'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"epicId": "auth", "title": "New Title"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected duplicate to be rejected: {result:?}"
    );

    // @step And the error message contains the substring 'Failed to create epic'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Failed to create epic"),
        "missing wrapper prefix: {msg}"
    );

    // @step And the error message contains the substring 'Epic auth already exists'
    assert!(
        msg.contains("Epic auth already exists"),
        "missing duplicate-detection text: {msg}"
    );

    // @step And the existing 'auth' epic in spec/epics.json still has title='Old Title'
    let data = read_epics_value(tmp.path());
    assert_eq!(
        data["epics"]["auth"]["title"].as_str(),
        Some("Old Title"),
        "existing epic must not be clobbered"
    );
}

#[test]
fn dispatcher_auto_creates_spec_directory_when_missing() {
    // Scenario: Dispatcher auto-creates the spec directory when missing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch create-epic with epicId='auth' and title='Authentication'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"epicId": "auth", "title": "Authentication"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the directory spec/ exists
    assert!(tmp.path().join("spec").is_dir());

    // @step And the file spec/epics.json exists
    assert!(tmp.path().join("spec/epics.json").is_file());
}

#[test]
fn dispatcher_tolerates_malformed_epics_json_by_treating_it_as_empty() {
    // Scenario: Dispatcher tolerates malformed spec/epics.json by treating it as empty (TS bare-catch parity)

    // @step Given spec/epics.json exists with malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), "{ not json");

    // @step When I dispatch create-epic with epicId='auth' and title='Authentication'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"epicId": "auth", "title": "Authentication"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "malformed epics.json must be silently swallowed: {result:?}"
    );

    // @step And spec/epics.json is overwritten and contains exactly one epic 'auth'
    let data = read_epics_value(tmp.path());
    let epics = data["epics"].as_object().expect("epics object");
    assert_eq!(epics.len(), 1, "expected exactly one epic, got {epics:?}");
    assert!(epics.contains_key("auth"));
}

#[test]
fn dispatcher_fails_fast_when_required_args_are_missing() {
    // Scenario: Dispatcher fails fast when required args are missing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch create-epic with no epicId field in the args
    let result = dispatch_command(req(tmp.path(), json!({"title": "Authentication"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected missing-field error: {result:?}");

    // @step And the error message contains the substring 'Invalid args for fspec command create-epic'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Invalid args for fspec command create-epic"),
        "missing canonical InvalidArgs prefix: {msg}"
    );
}

#[test]
fn dispatcher_response_text_renders_canonical_success_block_without_description() {
    // Scenario: Dispatcher response text renders the canonical success block without description

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch create-epic with epicId='auth' and title='Authentication'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"epicId": "auth", "title": "Authentication"}),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line '✓ Created epic auth'
    assert!(
        result.data.lines().any(|l| l == "✓ Created epic auth"),
        "missing checkmark line; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the line '  Title: Authentication'
    assert!(
        result.data.lines().any(|l| l == "  Title: Authentication"),
        "missing title line; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data does NOT contain the substring 'Description:'
    assert!(
        !result.data.contains("Description:"),
        "must omit Description when none provided; got:\n{}",
        result.data
    );
}

#[test]
fn dispatcher_response_text_includes_description_line_when_provided() {
    // Scenario: Dispatcher response text includes the Description line when provided

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch create-epic with epicId='auth', title='Authentication', and description='Login features'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "epicId": "auth",
            "title": "Authentication",
            "description": "Login features"
        }),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line '✓ Created epic auth'
    assert!(
        result.data.lines().any(|l| l == "✓ Created epic auth"),
        "missing checkmark line; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the line '  Title: Authentication'
    assert!(
        result.data.lines().any(|l| l == "  Title: Authentication"),
        "missing title line; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the line '  Description: Login features'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  Description: Login features"),
        "missing description line; got:\n{}",
        result.data
    );
}
