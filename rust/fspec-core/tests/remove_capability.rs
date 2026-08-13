#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/remove-capability-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `remove-capability`
// (RPC-269). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "remove-capability".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_foundation_named(project_root: &Path, name: &str, value: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join(name),
        serde_json::to_string_pretty(value).expect("ser foundation"),
    )
    .expect("write foundation file");
}

fn write_foundation(project_root: &Path, value: &Value) {
    write_foundation_named(project_root, "foundation.json", value);
}

fn read_foundation_named(project_root: &Path, name: &str) -> Value {
    let raw =
        fs::read_to_string(project_root.join("spec").join(name)).expect("read foundation file");
    serde_json::from_str(&raw).expect("parse foundation file")
}

fn read_foundation(project_root: &Path) -> Value {
    read_foundation_named(project_root, "foundation.json")
}

fn caps(data: &Value) -> &Vec<Value> {
    data["solutionSpace"]["capabilities"]
        .as_array()
        .expect("solutionSpace.capabilities must be an array")
}

fn foundation_with_caps(capabilities: Value) -> Value {
    json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "solutionSpace": {"overview": "o", "capabilities": capabilities}
    })
}

fn result_data(result: &DispatchResult) -> Value {
    serde_json::from_str(&result.data).unwrap_or(Value::Null)
}

// ---------- scenarios ----------

#[test]
fn dispatcher_removes_a_capability_from_an_existing_foundation_json() {
    // Scenario: Dispatcher removes a capability from an existing foundation.json

    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'User Authentication'},{name:'Search'}]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_caps(json!([
            {"name": "User Authentication", "description": "a"},
            {"name": "Search", "description": "s"}
        ])),
    );

    // @step And no spec/foundation.json.draft file exists
    assert!(!tmp.path().join("spec/foundation.json.draft").exists());

    // @step When I dispatch remove-capability with name='Search'
    let result = dispatch_command(req(tmp.path(), json!({"name": "Search"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/foundation.json solutionSpace.capabilities has length 1
    let data = read_foundation(tmp.path());
    assert_eq!(caps(&data).len(), 1);

    // @step And the remaining capability has name='User Authentication'
    assert_eq!(caps(&data)[0]["name"].as_str(), Some("User Authentication"));

    // @step And the result fileName is 'foundation.json'
    assert_eq!(
        result_data(&result)["fileName"].as_str(),
        Some("foundation.json")
    );
}

#[test]
fn dispatcher_draft_takes_precedence_over_the_final_foundation_file() {
    // Scenario: Draft takes precedence over the final foundation file

    // @step Given spec/foundation.json.draft exists with solutionSpace.capabilities=[{name:'Reporting'},{name:'Data Export'}]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation_named(
        tmp.path(),
        "foundation.json.draft",
        &foundation_with_caps(json!([
            {"name": "Reporting", "description": "r"},
            {"name": "Data Export", "description": "d"}
        ])),
    );

    // @step And spec/foundation.json also exists
    write_foundation(
        tmp.path(),
        &foundation_with_caps(json!([
            {"name": "Reporting", "description": "r"},
            {"name": "Data Export", "description": "d"}
        ])),
    );

    // @step When I dispatch remove-capability with name='Reporting'
    let result = dispatch_command(req(tmp.path(), json!({"name": "Reporting"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json.draft solutionSpace.capabilities has length 1
    let draft = read_foundation_named(tmp.path(), "foundation.json.draft");
    assert_eq!(caps(&draft).len(), 1);

    // @step And spec/foundation.json is left unchanged
    let final_data = read_foundation(tmp.path());
    assert_eq!(
        caps(&final_data).len(),
        2,
        "final foundation.json must be untouched"
    );

    // @step And the result fileName is 'foundation.json.draft'
    assert_eq!(
        result_data(&result)["fileName"].as_str(),
        Some("foundation.json.draft")
    );
}

#[test]
fn dispatcher_removes_only_the_first_exact_case_sensitive_match() {
    // Scenario: Only the first exact case-sensitive match is removed

    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'login'},{name:'Login'}]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_caps(json!([
            {"name": "login", "description": "lower"},
            {"name": "Login", "description": "upper"}
        ])),
    );

    // @step When I dispatch remove-capability with name='Login'
    let result = dispatch_command(req(tmp.path(), json!({"name": "Login"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json solutionSpace.capabilities has length 1
    let data = read_foundation(tmp.path());
    assert_eq!(caps(&data).len(), 1);

    // @step And the remaining capability has name='login'
    assert_eq!(caps(&data)[0]["name"].as_str(), Some("login"));
}

#[test]
fn dispatcher_fails_when_no_capabilities_exist() {
    // Scenario: Dispatcher fails when no capabilities exist

    // @step Given spec/foundation.json exists with an empty solutionSpace.capabilities array
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &foundation_with_caps(json!([])));

    // @step When I dispatch remove-capability with name='X'
    let result = dispatch_command(req(tmp.path(), json!({"name": "X"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'Capability "X" not found'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Capability \"X\" not found"),
        "missing not-found text; got: {msg}"
    );

    // @step And the error message contains the substring 'No capabilities exist in foundation'
    assert!(
        msg.contains("No capabilities exist in foundation"),
        "missing detail line; got: {msg}"
    );
}

#[test]
fn dispatcher_fails_and_lists_available_capabilities_when_name_not_found() {
    // Scenario: Dispatcher fails and lists available capabilities when the name is not found

    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'Reporting'},{name:'Search'}]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_caps(json!([
            {"name": "Reporting", "description": "r"},
            {"name": "Search", "description": "s"}
        ])),
    );

    // @step When I dispatch remove-capability with name='Login'
    let result = dispatch_command(req(tmp.path(), json!({"name": "Login"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'Capability "Login" not found'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Capability \"Login\" not found"),
        "missing not-found text; got: {msg}"
    );

    // @step And the error message contains the substring 'Available capabilities: Reporting, Search'
    assert!(
        msg.contains("Available capabilities: Reporting, Search"),
        "missing available list; got: {msg}"
    );
}

#[test]
fn dispatcher_fails_when_neither_foundation_json_nor_its_draft_exists() {
    // Scenario: Dispatcher fails when neither foundation.json nor its draft exists

    // @step Given a project root directory with no spec/foundation.json and no spec/foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/foundation.json").exists());
    assert!(!tmp.path().join("spec/foundation.json.draft").exists());

    // @step When I dispatch remove-capability with name='X'
    let result = dispatch_command(req(tmp.path(), json!({"name": "X"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'foundation.json not found'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("foundation.json not found"),
        "missing canonical error text; got: {msg}"
    );

    // @step And no spec/foundation.json file is created
    assert!(!tmp.path().join("spec/foundation.json").exists());
}

#[test]
fn dispatcher_preserves_unknown_fields_and_untouched_capabilities_on_write() {
    // Scenario: Unknown top-level fields and untouched capabilities are preserved on write

    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'Reporting'},{name:'Search'}] and a custom top-level 'experiments' key
    let tmp = TempDir::new().expect("tempdir");
    let mut f = foundation_with_caps(json!([
        {"name": "Reporting", "description": "r"},
        {"name": "Search", "description": "s"}
    ]));
    f["experiments"] = json!({"alpha": true, "beta": [1, 2, 3]});
    write_foundation(tmp.path(), &f);

    // @step When I dispatch remove-capability with name='Reporting'
    let result = dispatch_command(req(tmp.path(), json!({"name": "Reporting"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json still contains the 'experiments' key with its original value
    let data = read_foundation(tmp.path());
    assert_eq!(data["experiments"]["alpha"].as_bool(), Some(true));
    assert_eq!(data["experiments"]["beta"][2].as_u64(), Some(3));

    // @step And the remaining capability has name='Search'
    assert_eq!(caps(&data).len(), 1);
    assert_eq!(caps(&data)[0]["name"].as_str(), Some("Search"));
}
