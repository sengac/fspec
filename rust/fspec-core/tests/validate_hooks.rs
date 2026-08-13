#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/validate-hooks-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `validate-hooks`
// (RPC-322). Each scenario maps to exactly one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim.
//
// FRAMING A: the core `run` returns a JSON envelope {valid, exitCode, message}
// (RPC-247 list-hooks precedent). The CLI bridge prints `message` and exits
// with `exitCode`.
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
        command: "validate-hooks".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_hooks(root: &Path, raw: &str) {
    let spec = root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("fspec-hooks.json"), raw).expect("write fspec-hooks.json");
}

/// Create a hook script file on disk so existence checks pass.
fn write_script(root: &Path, rel: &str) {
    let abs = root.join(rel);
    fs::create_dir_all(abs.parent().unwrap()).expect("mkdir script parent");
    fs::write(&abs, "#!/bin/sh\necho hi\n").expect("write hook script");
}

fn parse_data(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{}", result.data))
}

fn message(data: &Value) -> String {
    data["message"].as_str().unwrap_or("").to_string()
}

// ---------- scenarios ----------

#[test]
fn dispatcher_reports_all_hooks_valid_when_every_script_exists() {
    // Scenario: Dispatcher reports all hooks valid when every script exists

    // @step Given spec/fspec-hooks.json configures one hook whose command script exists on disk
    let tmp = TempDir::new().expect("tempdir");
    write_script(tmp.path(), "spec/hooks/lint.sh");
    write_hooks(
        tmp.path(),
        r#"{ "hooks": { "pre-implementing": [ { "name": "lint", "command": "spec/hooks/lint.sh" } ] } }"#,
    );

    // @step When I dispatch the validate-hooks command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result);

    // @step Then the result is valid with message '✓ All hooks are valid' and exitCode 0
    assert_eq!(data["valid"].as_bool(), Some(true), "got {data}");
    assert_eq!(message(&data), "✓ All hooks are valid", "got {data}");
    assert_eq!(data["exitCode"].as_i64(), Some(0), "got {data}");
}

#[test]
fn dispatcher_reports_a_missing_hook_script() {
    // Scenario: Dispatcher reports a missing hook script

    // @step Given spec/fspec-hooks.json configures a hook with command 'spec/hooks/lint.sh' that does not exist on disk
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{ "hooks": { "pre-implementing": [ { "name": "lint", "command": "spec/hooks/lint.sh" } ] } }"#,
    );
    assert!(!tmp.path().join("spec/hooks/lint.sh").exists());

    // @step When I dispatch the validate-hooks command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result is invalid with exitCode 1
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");
    assert_eq!(data["exitCode"].as_i64(), Some(1), "got {data}");

    // @step Then the message contains '✗ Hook validation failed'
    assert!(
        message(&data).contains("✗ Hook validation failed"),
        "got {data}"
    );

    // @step Then the message contains 'Hook command not found: spec/hooks/lint.sh'
    assert!(
        message(&data).contains("Hook command not found: spec/hooks/lint.sh"),
        "got {data}"
    );
}

#[test]
fn dispatcher_reports_no_hooks_configured_for_empty_hooks_object() {
    // Scenario: Dispatcher reports no hooks configured for an empty hooks object

    // @step Given spec/fspec-hooks.json exists with an empty hooks object
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(tmp.path(), r#"{ "hooks": {} }"#);

    // @step When I dispatch the validate-hooks command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result);

    // @step Then the message is 'No hooks configured (nothing to validate)' with exitCode 0
    assert_eq!(
        message(&data),
        "No hooks configured (nothing to validate)",
        "got {data}"
    );
    assert_eq!(data["exitCode"].as_i64(), Some(0), "got {data}");
}

#[test]
fn dispatcher_reports_a_load_failure_when_the_config_is_missing() {
    // Scenario: Dispatcher reports a load failure when the config is missing

    // @step Given an empty project root with no spec/fspec-hooks.json
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/fspec-hooks.json").exists());

    // @step When I dispatch the validate-hooks command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result is invalid with exitCode 1
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");
    assert_eq!(data["exitCode"].as_i64(), Some(1), "got {data}");

    // @step Then the message is 'Failed to load hook configuration'
    assert_eq!(
        message(&data),
        "Failed to load hook configuration",
        "got {data}"
    );
}

#[test]
fn dispatcher_reports_a_load_failure_when_the_config_is_malformed_json() {
    // Scenario: Dispatcher reports a load failure when the config is malformed JSON

    // @step Given spec/fspec-hooks.json exists but contains the malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(tmp.path(), "{ not json");

    // @step When I dispatch the validate-hooks command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result is invalid with exitCode 1
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");
    assert_eq!(data["exitCode"].as_i64(), Some(1), "got {data}");

    // @step Then the message is 'Failed to load hook configuration'
    assert_eq!(
        message(&data),
        "Failed to load hook configuration",
        "got {data}"
    );
}

#[test]
fn dispatcher_lists_every_missing_script_across_multiple_events() {
    // Scenario: Dispatcher lists every missing script across multiple events

    // @step Given spec/fspec-hooks.json configures two hooks under different events whose command scripts are both missing
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{ "hooks": {
            "pre-implementing": [ { "name": "lint", "command": "spec/hooks/lint.sh" } ],
            "post-implementing": [ { "name": "test", "command": "spec/hooks/test.sh" } ]
        } }"#,
    );

    // @step When I dispatch the validate-hooks command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result is invalid with exitCode 1
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");
    assert_eq!(data["exitCode"].as_i64(), Some(1), "got {data}");

    // @step Then the message contains a 'Hook command not found' line for each missing script
    let msg = message(&data);
    assert!(
        msg.contains("Hook command not found: spec/hooks/lint.sh"),
        "got {data}"
    );
    assert!(
        msg.contains("Hook command not found: spec/hooks/test.sh"),
        "got {data}"
    );
}
