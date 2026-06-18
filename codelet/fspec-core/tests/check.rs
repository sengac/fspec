#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/check-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `check`
// (RPC-201). Each scenario maps to exactly one #[test] fn with @step
// comments mirroring the Gherkin steps verbatim.
//
// PHASE B (TESTING): the core impl is still a stub, so every dispatch
// returns FspecCoreError::NotYetPorted. These tests are RED until PHASE C.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path) -> DispatchRequest {
    DispatchRequest {
        command: "check".to_string(),
        args_json: json!({}).to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_file(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

/// Register a component tag and a feature-group tag in spec/tags.json plus
/// any extra technical tags supplied.
fn write_tags(root: &Path, component: &[&str], feature_group: &[&str], other: &[&str]) {
    let to_tags = |names: &[&str]| -> Vec<Value> {
        names
            .iter()
            .map(|n| json!({ "name": n, "description": "x" }))
            .collect()
    };
    let data = json!({
        "categories": [
            { "name": "Component Tags", "description": "", "required": true, "tags": to_tags(component) },
            { "name": "Feature Group Tags", "description": "", "required": true, "tags": to_tags(feature_group) },
            { "name": "Technical Tags", "description": "", "required": false, "tags": to_tags(other) }
        ]
    });
    write_file(
        root,
        "spec/tags.json",
        &serde_json::to_string_pretty(&data).unwrap(),
    );
}

// A minimal valid feature carrying a registered component + feature-group tag.
// Text is in CANONICAL formatted form (each tag on its own line, matching the
// AST formatter's output) so the `check` formatting sub-check reports PASS.
fn valid_feature(name: &str) -> String {
    format!("@comp\n@grp\nFeature: {name}\n\n  Scenario: A\n    Given x\n")
}

fn parse_data(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{}", result.data))
}

fn all_error_strings(data: &Value) -> Vec<String> {
    data["errors"]
        .as_array()
        .map(|a| a.iter().filter_map(|e| e.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

// ---------- scenarios ----------

#[test]
fn all_sub_checks_pass_for_valid_registered_feature_files() {
    // Scenario: All sub-checks pass for valid registered feature files

    // @step Given spec/features contains three valid feature files whose tags are all registered in spec/tags.json
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &["@comp"], &["@grp"], &[]);
    write_file(tmp.path(), "spec/features/a.feature", &valid_feature("A"));
    write_file(tmp.path(), "spec/features/b.feature", &valid_feature("B"));
    write_file(tmp.path(), "spec/features/c.feature", &valid_feature("C"));

    // @step When I dispatch the check command against that project root
    let result = dispatch_command(req(tmp.path()));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result);
    assert_eq!(data["success"].as_bool(), Some(true), "data: {data}");

    // @step Then the gherkinStatus is 'PASS'
    assert_eq!(data["gherkinStatus"].as_str(), Some("PASS"));

    // @step Then the tagStatus is 'PASS'
    assert_eq!(data["tagStatus"].as_str(), Some("PASS"));
}

#[test]
fn gherkin_syntax_failure_fails_the_check() {
    // Scenario: Gherkin syntax failure fails the check

    // @step Given spec/features contains a feature file with invalid Gherkin syntax
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &["@comp"], &["@grp"], &[]);
    write_file(tmp.path(), "spec/features/broken.feature", "this is not gherkin");

    // @step When I dispatch the check command against that project root
    let result = dispatch_command(req(tmp.path()));
    let data = parse_data(&result);

    // @step Then the gherkinStatus is 'FAIL'
    assert_eq!(data["gherkinStatus"].as_str(), Some("FAIL"), "data: {data}");

    // @step Then the result success field is false
    assert_eq!(data["success"].as_bool(), Some(false));

    // @step Then the errors list contains an entry mentioning that file
    assert!(
        all_error_strings(&data).iter().any(|e| e.contains("broken.feature")),
        "errors must mention broken.feature; got {:?}",
        all_error_strings(&data)
    );
}

#[test]
fn an_unregistered_tag_fails_the_check() {
    // Scenario: An unregistered tag fails the check

    // @step Given spec/features contains a valid feature file carrying the unregistered tag '@unknown-tag'
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &["@comp"], &["@grp"], &[]);
    write_file(
        tmp.path(),
        "spec/features/a.feature",
        "@comp @grp @unknown-tag\nFeature: A\n\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch the check command against that project root
    let result = dispatch_command(req(tmp.path()));
    let data = parse_data(&result);

    // @step Then the tagStatus is 'FAIL'
    assert_eq!(data["tagStatus"].as_str(), Some("FAIL"), "data: {data}");

    // @step Then the result success field is false
    assert_eq!(data["success"].as_bool(), Some(false));

    // @step Then the errors list mentions '@unknown-tag'
    assert!(
        all_error_strings(&data).iter().any(|e| e.contains("@unknown-tag")),
        "errors must mention @unknown-tag; got {:?}",
        all_error_strings(&data)
    );
}

#[test]
fn no_feature_files_reports_the_canonical_message_and_succeeds() {
    // Scenario: No feature files reports the canonical message and succeeds

    // @step Given a project root with no feature files under spec/features
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the check command against that project root
    let result = dispatch_command(req(tmp.path()));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result);

    // @step Then the result message is 'No feature files found'
    assert_eq!(data["message"].as_str(), Some("No feature files found"));

    // @step Then the fileCount is 0
    assert_eq!(data["fileCount"].as_i64(), Some(0));
}
