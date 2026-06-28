#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/validate-tags-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `validate-tags`
// (RPC-324). Each scenario maps to exactly one #[test] fn with @step
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

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "validate-tags".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_tags(root: &Path, component: &[&str], feature_group: &[&str], other: &[&str]) {
    let spec = root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
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
    fs::write(
        spec.join("tags.json"),
        serde_json::to_string_pretty(&data).unwrap(),
    )
    .expect("write tags.json");
}

fn write_feature(root: &Path, rel: &str, body: &str) {
    let abs = root.join(rel);
    fs::create_dir_all(abs.parent().unwrap()).expect("mkdir feature parent");
    fs::write(&abs, body).expect("write feature file");
}

fn write_work_units(root: &Path, ids: &[&str]) {
    let spec = root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let mut wus = serde_json::Map::new();
    for id in ids {
        wus.insert(
            (*id).to_string(),
            json!({ "id": id, "title": "t", "status": "backlog", "createdAt": "x", "updatedAt": "x" }),
        );
    }
    let data = json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": {
            "backlog": [], "specifying": [], "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    });
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&data).unwrap(),
    )
    .expect("write work-units.json");
}

fn parse_data(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{}", result.data))
}

/// Collect every error message across every per-file result.
fn all_messages(data: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(results) = data["results"].as_array() {
        for r in results {
            if let Some(errs) = r["errors"].as_array() {
                for e in errs {
                    if let Some(m) = e["message"].as_str() {
                        out.push(m.to_string());
                    }
                }
            }
        }
    }
    out
}

fn has_message(data: &Value, needle: &str) -> bool {
    all_messages(data).iter().any(|m| m.contains(needle))
}

/// Is there at least one per-file result with valid == expected?
fn any_file_valid(data: &Value, expected: bool) -> bool {
    data["results"]
        .as_array()
        .map(|rs| rs.iter().any(|r| r["valid"].as_bool() == Some(expected)))
        .unwrap_or(false)
}

// A minimal valid feature carrying a registered component + feature-group tag.
fn valid_feature(name: &str) -> String {
    format!("@comp @grp\nFeature: {name}\n\n  Scenario: A\n    Given x\n")
}

// ---------- scenarios ----------

#[test]
fn dispatcher_reports_all_feature_files_valid_when_every_tag_is_registered() {
    // Scenario: Dispatcher reports all feature files valid when every tag is registered

    // @step Given spec/tags.json registers a component tag and a feature-group tag plus the tags used by two feature files
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &["@comp"], &["@grp"], &[]);

    // @step Given two feature files each carry only registered tags including a component and feature-group tag
    write_feature(tmp.path(), "spec/features/a.feature", &valid_feature("A"));
    write_feature(tmp.path(), "spec/features/b.feature", &valid_feature("B"));

    // @step When I dispatch the validate-tags command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result);

    // @step Then the result reports validCount=2 and invalidCount=0
    assert_eq!(data["validCount"].as_u64(), Some(2), "got {data}");
    assert_eq!(data["invalidCount"].as_u64(), Some(0), "got {data}");
}

#[test]
fn dispatcher_flags_an_unregistered_feature_tag() {
    // Scenario: Dispatcher flags an unregistered feature tag

    // @step Given spec/tags.json does not register the tag '@made-up'
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &["@comp"], &["@grp"], &[]);

    // @step Given a feature file carries the feature-level tag '@made-up'
    write_feature(
        tmp.path(),
        "spec/features/example.feature",
        "@comp @grp @made-up\nFeature: Example\n\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch the validate-tags command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result marks that file invalid
    assert!(any_file_valid(&data, false), "got {data}");

    // @step Then the file errors include the message 'Unregistered tag: @made-up in spec/features/example.feature'
    assert!(
        has_message(
            &data,
            "Unregistered tag: @made-up in spec/features/example.feature"
        ),
        "got {data}"
    );

    // @step Then the result reports invalidCount=1
    assert_eq!(data["invalidCount"].as_u64(), Some(1), "got {data}");
}

#[test]
fn dispatcher_rejects_a_scenario_level_work_unit_tag() {
    // Scenario: Dispatcher rejects a scenario-level work-unit tag

    // @step Given a feature file carries the work-unit tag '@AUTH-001' on a scenario
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &["@comp"], &["@grp"], &[]);
    write_feature(
        tmp.path(),
        "spec/features/example.feature",
        "@comp @grp\nFeature: Example\n\n  @AUTH-001\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch the validate-tags command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result marks that file invalid
    assert!(any_file_valid(&data, false), "got {data}");

    // @step Then the file errors include the message 'Work unit ID tag @AUTH-001 must be at feature level, not scenario level'
    assert!(
        has_message(
            &data,
            "Work unit ID tag @AUTH-001 must be at feature level, not scenario level"
        ),
        "got {data}"
    );
}

#[test]
fn dispatcher_reports_a_feature_level_work_unit_tag_not_in_work_units_json() {
    // Scenario: Dispatcher reports a feature-level work-unit tag that is not in work-units.json

    // @step Given spec/work-units.json exists and does NOT contain a work unit AUTH-999
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &["@comp"], &["@grp"], &[]);
    write_work_units(tmp.path(), &["AUTH-001"]);

    // @step Given a feature file carries the feature-level work-unit tag '@AUTH-999'
    write_feature(
        tmp.path(),
        "spec/features/example.feature",
        "@comp @grp @AUTH-999\nFeature: Example\n\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch the validate-tags command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result marks that file invalid
    assert!(any_file_valid(&data, false), "got {data}");

    // @step Then the file errors include the message 'Work unit @AUTH-999 not found in spec/work-units.json'
    assert!(
        has_message(
            &data,
            "Work unit @AUTH-999 not found in spec/work-units.json"
        ),
        "got {data}"
    );
}

#[test]
fn dispatcher_reports_a_feature_level_work_unit_tag_when_work_units_json_is_missing() {
    // Scenario: Dispatcher reports a feature-level work-unit tag when work-units.json is missing

    // @step Given spec/work-units.json does NOT exist in the project root
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &["@comp"], &["@grp"], &[]);
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step Given a feature file carries the feature-level work-unit tag '@AUTH-001'
    write_feature(
        tmp.path(),
        "spec/features/example.feature",
        "@comp @grp @AUTH-001\nFeature: Example\n\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch the validate-tags command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result marks that file invalid
    assert!(any_file_valid(&data, false), "got {data}");

    // @step Then the file errors include the message 'Work unit @AUTH-001 found but spec/work-units.json does not exist'
    assert!(
        has_message(
            &data,
            "Work unit @AUTH-001 found but spec/work-units.json does not exist"
        ),
        "got {data}"
    );
}

#[test]
fn dispatcher_reports_a_missing_required_component_tag() {
    // Scenario: Dispatcher reports a missing required component tag

    // @step Given a feature file carries a feature-group tag but no component-category tag and no '@component' placeholder
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &["@comp"], &["@grp"], &[]);
    write_feature(
        tmp.path(),
        "spec/features/example.feature",
        "@grp\nFeature: Example\n\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch the validate-tags command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result marks that file invalid
    assert!(any_file_valid(&data, false), "got {data}");

    // @step Then the file errors include the message 'Missing required component tag'
    assert!(
        has_message(&data, "Missing required component tag"),
        "got {data}"
    );
}

#[test]
fn dispatcher_reports_a_missing_required_feature_group_tag() {
    // Scenario: Dispatcher reports a missing required feature-group tag

    // @step Given a feature file carries a component tag but no feature-group-category tag and no '@feature-group' placeholder
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &["@comp"], &["@grp"], &[]);
    write_feature(
        tmp.path(),
        "spec/features/example.feature",
        "@comp\nFeature: Example\n\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch the validate-tags command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result marks that file invalid
    assert!(any_file_valid(&data, false), "got {data}");

    // @step Then the file errors include the message 'Missing required feature-group tag'
    assert!(
        has_message(&data, "Missing required feature-group tag"),
        "got {data}"
    );
}

#[test]
fn dispatcher_flags_a_lowercase_work_unit_like_tag_as_malformed() {
    // Scenario: Dispatcher flags a lowercase work-unit-like tag as malformed

    // @step Given a feature file carries the unregistered feature-level tag '@auth-001'
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &["@comp"], &["@grp"], &[]);
    write_feature(
        tmp.path(),
        "spec/features/example.feature",
        "@comp @grp @auth-001\nFeature: Example\n\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch the validate-tags command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result marks that file invalid
    assert!(any_file_valid(&data, false), "got {data}");

    // @step Then the file errors include the message 'Invalid work unit tag format: @auth-001'
    assert!(
        has_message(&data, "Invalid work unit tag format: @auth-001"),
        "got {data}"
    );
}

#[test]
fn dispatcher_flags_a_placeholder_component_tag() {
    // Scenario: Dispatcher flags a placeholder @component tag

    // @step Given a feature file carries the unregistered feature-level tag '@component'
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &["@comp"], &["@grp"], &[]);
    write_feature(
        tmp.path(),
        "spec/features/example.feature",
        "@component @grp\nFeature: Example\n\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch the validate-tags command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the file errors include the message 'Placeholder tag: @component'
    assert!(
        has_message(&data, "Placeholder tag: @component"),
        "got {data}"
    );
}

#[test]
fn dispatcher_returns_zero_counts_when_no_feature_files_exist() {
    // Scenario: Dispatcher returns zero counts when no feature files exist

    // @step Given an empty project root with no spec/features directory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features").exists());

    // @step When I dispatch the validate-tags command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result);

    // @step Then the result reports validCount=0 and invalidCount=0
    assert_eq!(data["validCount"].as_u64(), Some(0), "got {data}");
    assert_eq!(data["invalidCount"].as_u64(), Some(0), "got {data}");
}

#[test]
fn dispatcher_validates_only_the_single_file_named_by_the_file_argument() {
    // Scenario: Dispatcher validates only the single file named by the file argument

    // @step Given two feature files exist but only one carries an unregistered tag
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &["@comp"], &["@grp"], &[]);
    write_feature(
        tmp.path(),
        "spec/features/good.feature",
        &valid_feature("Good"),
    );
    write_feature(
        tmp.path(),
        "spec/features/bad.feature",
        "@comp @grp @made-up\nFeature: Bad\n\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch the validate-tags command with file set to the valid feature file
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "file": "spec/features/good.feature" }),
    ));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports validCount=1 and invalidCount=0
    assert_eq!(data["validCount"].as_u64(), Some(1), "got {data}");
    assert_eq!(data["invalidCount"].as_u64(), Some(0), "got {data}");
}

#[test]
fn dispatcher_skips_a_file_that_does_not_parse_as_gherkin() {
    // Scenario: Dispatcher skips a file that does not parse as Gherkin

    // @step Given a feature file that does not contain a valid Feature header
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), &["@comp"], &["@grp"], &[]);
    write_feature(
        tmp.path(),
        "spec/features/broken.feature",
        "this is not gherkin at all\njust some random text\n",
    );

    // @step When I dispatch the validate-tags command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result marks that file valid
    assert!(any_file_valid(&data, true), "got {data}");

    // @step Then the result reports invalidCount=0
    assert_eq!(data["invalidCount"].as_u64(), Some(0), "got {data}");
}
