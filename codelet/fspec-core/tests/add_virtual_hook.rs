// Feature: spec/features/add-virtual-hook-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-virtual-hook`
// (RPC-195). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// RED phase: add-virtual-hook is still a NotYetPorted stub (it is NOT in
// `PORTED_COMMANDS`), so every assertion below should fail today —
// dispatch_command returns `success=false` with the canonical "not yet
// ported" error string instead of the expected payload.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-virtual-hook".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn seed_work_units(project_root: &Path, value: Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("create spec dir");
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&value).expect("serialize seed"),
    )
    .expect("write work-units.json");
}

fn read_work_units(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

/// Single work-unit AUTH-001 with NO virtualHooks field.
fn auth001_no_hooks() -> Value {
    json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": {
            "AUTH-001": {
                "id": "AUTH-001",
                "title": "Login feature",
                "status": "backlog",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": {
            "backlog": ["AUTH-001"],
            "specifying": [], "testing": [], "implementing": [],
            "validating": [], "done": [], "blocked": []
        }
    })
}

/// Single work-unit AUTH-001 with an empty `virtualHooks: []` field.
fn auth001_empty_hooks() -> Value {
    let mut v = auth001_no_hooks();
    v["workUnits"]["AUTH-001"]["virtualHooks"] = json!([]);
    v
}

/// AUTH-001 with one pre-existing 'lint' hook at post-implementing.
fn auth001_with_lint_hook() -> Value {
    let mut v = auth001_no_hooks();
    v["workUnits"]["AUTH-001"]["virtualHooks"] = json!([
        {
            "name": "lint",
            "event": "post-implementing",
            "command": "npm run lint",
            "blocking": true
        }
    ]);
    v
}

// ---------- scenarios ----------

#[test]
fn scenario_adds_blocking_hook_to_work_unit_with_no_prior_hooks() {
    // Scenario: Adds a blocking hook to a work unit with no prior virtualHooks

    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks field
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_no_hooks());

    // @step When I dispatch add-virtual-hook with workUnitId='AUTH-001' event='post-implementing' command='eslint src/' blocking=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "event": "post-implementing",
            "command": "eslint src/",
            "blocking": true
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the parsed JSON has hookCount=1
    let data = parse_data(&result.data);
    assert_eq!(
        data["hookCount"].as_u64(),
        Some(1),
        "expected hookCount=1; got: {}",
        result.data
    );

    // @step And the on-disk virtualHooks array has length 1
    let v = read_work_units(tmp.path());
    let hooks = v["workUnits"]["AUTH-001"]["virtualHooks"]
        .as_array()
        .expect("virtualHooks array");
    assert_eq!(hooks.len(), 1);

    // @step And the first stored hook has name='eslint' event='post-implementing' command='eslint src/' blocking=true
    assert_eq!(hooks[0]["name"].as_str(), Some("eslint"));
    assert_eq!(hooks[0]["event"].as_str(), Some("post-implementing"));
    assert_eq!(hooks[0]["command"].as_str(), Some("eslint src/"));
    assert_eq!(hooks[0]["blocking"].as_bool(), Some(true));

    // @step And the stored hook does NOT contain a gitContext key
    let obj = hooks[0].as_object().expect("hook entry should be an object");
    assert!(
        !obj.contains_key("gitContext"),
        "stored hook must not include gitContext when --git-context is not passed; got: {hooks:?}"
    );
}

#[test]
fn scenario_appends_second_hook_preserving_insertion_order() {
    // Scenario: Appends a second hook preserving insertion order

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'lint',event:'post-implementing',command:'npm run lint',blocking:true}]
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_with_lint_hook());

    // @step When I dispatch add-virtual-hook with workUnitId='AUTH-001' event='post-implementing' command='npm test' blocking=false
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "event": "post-implementing",
            "command": "npm test",
            "blocking": false
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the parsed JSON has hookCount=2
    let data = parse_data(&result.data);
    assert_eq!(data["hookCount"].as_u64(), Some(2));

    // @step And the stored virtualHooks names in order are ['lint','npm']
    let v = read_work_units(tmp.path());
    let hooks = v["workUnits"]["AUTH-001"]["virtualHooks"]
        .as_array()
        .expect("virtualHooks array");
    let names: Vec<&str> = hooks
        .iter()
        .map(|h| h["name"].as_str().expect("hook name"))
        .collect();
    assert_eq!(names, vec!["lint", "npm"]);
}

#[test]
fn scenario_git_context_generates_script_and_stores_relative_path() {
    // Scenario: gitContext=true generates a shell script and stores the relative script path

    // @step Given an empty project root directory with an AUTH-001 work unit
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_no_hooks());

    // @step When I dispatch add-virtual-hook with workUnitId='AUTH-001' event='post-implementing' command='eslint src/' blocking=true gitContext=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "event": "post-implementing",
            "command": "eslint src/",
            "blocking": true,
            "gitContext": true
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file spec/hooks/.virtual/AUTH-001-eslint.sh exists
    let script_path = tmp
        .path()
        .join("spec/hooks/.virtual/AUTH-001-eslint.sh");
    assert!(
        script_path.exists(),
        "expected generated script at {}",
        script_path.display()
    );

    // @step And the file spec/hooks/.virtual/AUTH-001-eslint.sh has Unix permission bits 0o755
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&script_path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o755, "expected mode 0o755, got 0o{mode:o}");
    }

    // @step And the stored hook command is 'spec/hooks/.virtual/AUTH-001-eslint.sh'
    let v = read_work_units(tmp.path());
    let hook = &v["workUnits"]["AUTH-001"]["virtualHooks"][0];
    assert_eq!(
        hook["command"].as_str(),
        Some("spec/hooks/.virtual/AUTH-001-eslint.sh")
    );

    // @step And the stored hook has gitContext=true
    assert_eq!(hook["gitContext"].as_bool(), Some(true));
}

#[test]
fn scenario_unknown_work_unit_returns_invalid_args_with_canonical_message() {
    // Scenario: Unknown work unit id returns InvalidArgs with the canonical message

    // @step Given spec/work-units.json contains AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_no_hooks());

    // @step When I dispatch add-virtual-hook with workUnitId='AUTH-999' event='post-implementing' command='npm test'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-999",
            "event": "post-implementing",
            "command": "npm test"
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step And the error message contains the exact substring "Work unit 'AUTH-999' does not exist"
    let msg = result.error.as_ref().expect("error message expected");
    assert!(
        msg.contains("Work unit 'AUTH-999' does not exist"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn scenario_empty_args_object_rejected_mentioning_work_unit_id() {
    // Scenario: Empty args object is rejected as InvalidArgs mentioning the missing workUnitId

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-virtual-hook with an empty args object {}
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step And the error message indicates that workUnitId is required
    let msg = result.error.as_ref().expect("error message expected");
    let lower = msg.to_lowercase();
    assert!(
        lower.contains("workunitid") || lower.contains("work unit id") || lower.contains("workunit"),
        "error message should mention workUnitId; got: {msg}"
    );
}

#[test]
fn scenario_hook_name_derivation_strips_path_and_trailing_args() {
    // Scenario: Hook name derivation strips path prefix and trailing arguments

    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_no_hooks());

    // @step When I dispatch add-virtual-hook with command='npm run lint'
    let r1 = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "event": "post-implementing",
            "command": "npm run lint"
        }),
    ));
    assert!(r1.success, "first dispatch should succeed: {r1:?}");

    // @step Then the stored hook has name='npm'
    let v = read_work_units(tmp.path());
    let h = &v["workUnits"]["AUTH-001"]["virtualHooks"][0];
    assert_eq!(h["name"].as_str(), Some("npm"));

    // @step When I dispatch add-virtual-hook with command='eslint src/'
    let r2 = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "event": "post-implementing",
            "command": "eslint src/"
        }),
    ));
    assert!(r2.success, "second dispatch should succeed: {r2:?}");

    // @step Then the stored hook has name='eslint'
    let v = read_work_units(tmp.path());
    let h = &v["workUnits"]["AUTH-001"]["virtualHooks"][1];
    assert_eq!(h["name"].as_str(), Some("eslint"));

    // @step When I dispatch add-virtual-hook with command='/usr/bin/node script.js'
    let r3 = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "event": "post-implementing",
            "command": "/usr/bin/node script.js"
        }),
    ));
    assert!(r3.success, "third dispatch should succeed: {r3:?}");

    // @step Then the stored hook has name='node'
    let v = read_work_units(tmp.path());
    let h = &v["workUnits"]["AUTH-001"]["virtualHooks"][2];
    assert_eq!(h["name"].as_str(), Some("node"));
}

#[test]
fn scenario_blocking_and_git_context_default_to_false() {
    // Scenario: blocking and gitContext default to false when omitted

    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_no_hooks());

    // @step When I dispatch add-virtual-hook with workUnitId='AUTH-001' event='post-validating' command='npm audit'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "event": "post-validating",
            "command": "npm audit"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the stored hook has blocking=false
    let v = read_work_units(tmp.path());
    let hook = &v["workUnits"]["AUTH-001"]["virtualHooks"][0];
    assert_eq!(hook["blocking"].as_bool(), Some(false));

    // @step And the stored hook JSON does NOT include the key 'gitContext'
    let obj = hook.as_object().expect("hook should be object");
    assert!(
        !obj.contains_key("gitContext"),
        "gitContext must be omitted when --git-context not passed; got: {obj:?}"
    );
}

#[test]
fn scenario_adding_to_existing_empty_virtual_hooks_array_appends() {
    // Scenario: Adding to an existing empty virtualHooks array appends and returns hookCount=1

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[]
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_empty_hooks());

    // @step When I dispatch add-virtual-hook with workUnitId='AUTH-001' event='post-implementing' command='npm test'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "event": "post-implementing",
            "command": "npm test"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the parsed JSON has hookCount=1
    let data = parse_data(&result.data);
    assert_eq!(data["hookCount"].as_u64(), Some(1));

    // @step And the on-disk virtualHooks array has length 1
    let v = read_work_units(tmp.path());
    let hooks = v["workUnits"]["AUTH-001"]["virtualHooks"]
        .as_array()
        .expect("virtualHooks array");
    assert_eq!(hooks.len(), 1);
}

#[test]
fn scenario_work_units_file_auto_created_but_lookup_still_fails() {
    // Scenario: spec/work-units.json is auto-created when missing but lookup still fails

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-virtual-hook with workUnitId='AUTH-001' event='post-implementing' command='npm test'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "event": "post-implementing",
            "command": "npm test"
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step And the error message contains the exact substring "Work unit 'AUTH-001' does not exist"
    let msg = result.error.as_ref().expect("error message expected");
    assert!(
        msg.contains("Work unit 'AUTH-001' does not exist"),
        "error message missing canonical substring: {msg}"
    );

    // @step And spec/work-units.json exists after the call
    assert!(
        tmp.path().join("spec/work-units.json").exists(),
        "ensure_work_units_file must auto-create spec/work-units.json"
    );
}

#[test]
fn scenario_result_json_uses_camel_case_hook_count_key() {
    // Scenario: Result JSON uses camelCase hookCount key

    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_no_hooks());

    // @step When I dispatch add-virtual-hook with workUnitId='AUTH-001' event='post-implementing' command='npm test'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "event": "post-implementing",
            "command": "npm test"
        }),
    ));

    // @step Then the DispatchResult.data parses to a JSON object containing the key 'hookCount'
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);
    let obj = data.as_object().expect("data should be an object");
    assert!(
        obj.contains_key("hookCount"),
        "data must contain key 'hookCount'; got: {result:?}"
    );

    // @step And the DispatchResult.data does NOT contain the key 'hook_count'
    assert!(
        !obj.contains_key("hook_count"),
        "data must NOT contain snake_case 'hook_count'; got: {result:?}"
    );

    // @step And the DispatchResult.data contains 'success' equal to true
    assert_eq!(obj.get("success"), Some(&Value::Bool(true)));
}
