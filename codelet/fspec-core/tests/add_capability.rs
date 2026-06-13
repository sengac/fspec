#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-capability-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-capability`
// (RPC-173). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-capability".to_string(),
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

fn result_data(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data).unwrap_or(Value::Null)
}

// ---------- scenarios ----------

#[test]
fn dispatcher_appends_a_capability_to_an_existing_foundation_json() {
    // Scenario: Dispatcher appends a capability to an existing foundation.json

    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'Reporting'}]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_caps(json!([{"name": "Reporting", "description": "Reports"}])),
    );

    // @step And no spec/foundation.json.draft file exists
    assert!(!tmp.path().join("spec/foundation.json.draft").exists());

    // @step When I dispatch add-capability with name='User Authentication' description='Login and sessions'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "User Authentication", "description": "Login and sessions"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/foundation.json solutionSpace.capabilities has length 2
    let data = read_foundation(tmp.path());
    assert_eq!(caps(&data).len(), 2, "expected 2 capabilities, got {data}");

    // @step And the last capability has name='User Authentication' and description='Login and sessions'
    let last = caps(&data).last().expect("at least one capability");
    assert_eq!(last["name"].as_str(), Some("User Authentication"));
    assert_eq!(last["description"].as_str(), Some("Login and sessions"));

    // @step And the result fileName is 'foundation.json'
    assert_eq!(
        result_data(&result)["fileName"].as_str(),
        Some("foundation.json")
    );
}

#[test]
fn dispatcher_draft_takes_precedence_over_the_final_foundation_file() {
    // Scenario: Draft takes precedence over the final foundation file

    // @step Given spec/foundation.json.draft exists with solutionSpace.capabilities=[{name:'Reporting'}]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation_named(
        tmp.path(),
        "foundation.json.draft",
        &foundation_with_caps(json!([{"name": "Reporting", "description": "Reports"}])),
    );

    // @step And spec/foundation.json also exists
    write_foundation(
        tmp.path(),
        &foundation_with_caps(json!([{"name": "Reporting", "description": "Reports"}])),
    );

    // @step When I dispatch add-capability with name='Data Export' description='Export to CSV'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "Data Export", "description": "Export to CSV"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json.draft solutionSpace.capabilities has length 2
    let draft = read_foundation_named(tmp.path(), "foundation.json.draft");
    assert_eq!(caps(&draft).len(), 2);

    // @step And spec/foundation.json is left unchanged
    let final_data = read_foundation(tmp.path());
    assert_eq!(
        caps(&final_data).len(),
        1,
        "final foundation.json must be untouched"
    );

    // @step And the result fileName is 'foundation.json.draft'
    assert_eq!(
        result_data(&result)["fileName"].as_str(),
        Some("foundation.json.draft")
    );
}

#[test]
fn dispatcher_creates_capabilities_array_when_solution_space_has_no_capabilities_key() {
    // Scenario: Capabilities array is created when solutionSpace has no capabilities key

    // @step Given spec/foundation.json exists with a solutionSpace object that has no capabilities key
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &json!({
            "version": "2.0.0",
            "solutionSpace": {"overview": "o"}
        }),
    );

    // @step When I dispatch add-capability with name='Search' description='Full text search'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "Search", "description": "Full text search"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json solutionSpace.capabilities has length 1
    let data = read_foundation(tmp.path());
    assert_eq!(caps(&data).len(), 1);

    // @step And the only capability has name='Search'
    assert_eq!(caps(&data)[0]["name"].as_str(), Some("Search"));
}

#[test]
fn dispatcher_prunes_all_placeholder_capabilities_before_adding_new_entry() {
    // Scenario: All-placeholder capabilities are pruned before the new entry is added

    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'[QUESTION: What can users do?]', description:'[DETECTED: ...]'}]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_caps(json!([
            {"name": "[QUESTION: What can users do?]", "description": "[DETECTED: ...]"}
        ])),
    );

    // @step When I dispatch add-capability with name='Login' description='Authenticate users'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "Login", "description": "Authenticate users"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json solutionSpace.capabilities has length 1
    let data = read_foundation(tmp.path());
    assert_eq!(caps(&data).len(), 1);

    // @step And the only capability has name='Login'
    assert_eq!(caps(&data)[0]["name"].as_str(), Some("Login"));

    // @step And the result removedCount is 1
    assert_eq!(result_data(&result)["removedCount"].as_u64(), Some(1));
}

#[test]
fn dispatcher_keeps_every_entry_in_a_mixed_real_and_placeholder_array() {
    // Scenario: Mixed real-and-placeholder array keeps every entry

    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'Reporting'},{name:'[QUESTION: anything else?]'}]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_caps(json!([
            {"name": "Reporting", "description": "Reports"},
            {"name": "[QUESTION: anything else?]", "description": "?"}
        ])),
    );

    // @step When I dispatch add-capability with name='Login' description='Authenticate users'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "Login", "description": "Authenticate users"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json solutionSpace.capabilities has length 3
    let data = read_foundation(tmp.path());
    assert_eq!(caps(&data).len(), 3);

    // @step And the result removedCount is 0
    assert_eq!(result_data(&result)["removedCount"].as_u64(), Some(0));
}

#[test]
fn dispatcher_fails_when_neither_foundation_json_nor_its_draft_exists() {
    // Scenario: Dispatcher fails when neither foundation.json nor its draft exists

    // @step Given a project root directory with no spec/foundation.json and no spec/foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/foundation.json").exists());
    assert!(!tmp.path().join("spec/foundation.json.draft").exists());

    // @step When I dispatch add-capability with name='X' description='Y'
    let result = dispatch_command(req(tmp.path(), json!({"name": "X", "description": "Y"})));

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

    // @step And no spec/foundation.json.draft file is created
    assert!(!tmp.path().join("spec/foundation.json.draft").exists());
}

#[test]
fn dispatcher_preserves_unknown_top_level_fields_on_write() {
    // Scenario: Unknown top-level foundation fields are preserved on write

    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[] and a custom top-level 'experiments' key
    let tmp = TempDir::new().expect("tempdir");
    let mut f = foundation_with_caps(json!([]));
    f["experiments"] = json!({"alpha": true, "beta": [1, 2, 3]});
    write_foundation(tmp.path(), &f);

    // @step When I dispatch add-capability with name='Search' description='Full text search'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "Search", "description": "Full text search"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json still contains the 'experiments' key with its original value
    let data = read_foundation(tmp.path());
    assert_eq!(data["experiments"]["alpha"].as_bool(), Some(true));
    assert_eq!(data["experiments"]["beta"][0].as_u64(), Some(1));
}

#[test]
fn dispatcher_result_reports_written_file_name_and_description() {
    // Scenario: Successful result reports the written file, name and description

    // @step Given spec/foundation.json exists with solutionSpace.capabilities=[]
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &foundation_with_caps(json!([])));

    // @step When I dispatch add-capability with name='Search' description='Full text search'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"name": "Search", "description": "Full text search"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the result name is 'Search'
    assert_eq!(result_data(&result)["name"].as_str(), Some("Search"));

    // @step And the result description is 'Full text search'
    assert_eq!(
        result_data(&result)["description"].as_str(),
        Some("Full text search")
    );

    // @step And the result fileName is 'foundation.json'
    assert_eq!(
        result_data(&result)["fileName"].as_str(),
        Some("foundation.json")
    );
}
