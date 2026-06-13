// Feature: spec/features/list-scenario-tags-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `list-scenario-tags` (RPC-249). Each scenario maps to exactly one
// #[test] function with @step comments mirroring the Gherkin steps
// verbatim.
//
// TS-parity envelope shape (RPC-249 architecture note [8]): the Rust
// dispatcher returns the {success, tags, message?, error?,
// categorizedTags?} payload INSIDE DispatchResult.data; the OUTER
// DispatchResult.success is ALWAYS true unless arg parsing fails.
// This matches `list_feature_tags.rs` (RPC-244) and the TS
// `listScenarioTags()` programmatic return shape.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "list-scenario-tags".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_feature(project_root: &Path, rel: &str, body: &str) {
    let abs = project_root.join(rel);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(abs, body).expect("write feature file");
}

fn write_tags_json(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("tags.json"), raw).expect("write tags.json");
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

// ---------- scenarios ----------

#[test]
fn scenario_returns_file_not_found_when_feature_file_missing() {
    // Scenario: Returns 'File not found' error when feature file does not exist

    // @step Given an empty project root directory with no spec/features/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features").exists());

    // @step When I dispatch list-scenario-tags with file='spec/features/nope.feature' and scenario='Anything' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/nope.feature",
            "scenario": "Anything",
            "format": "json"
        }),
    ));

    // @step Then the dispatcher result has DispatchResult.success=true (no FspecCoreError envelope) and DispatchResult.data parses to a JSON object with success=false, tags=[], and error='File not found: spec/features/nope.feature'
    assert!(
        result.success,
        "OUTER dispatcher must succeed (no FspecCoreError envelope); got {result:?}"
    );
    assert!(
        result.error.is_none(),
        "outer error must be None; got {:?}",
        result.error
    );
    let data = parse_data(&result.data);
    assert_eq!(
        data["success"].as_bool(),
        Some(false),
        "inner success=false; got {data}"
    );
    assert_eq!(
        data["tags"].as_array().map(Vec::len),
        Some(0),
        "tags array empty; got {data}"
    );
    assert_eq!(
        data["error"].as_str(),
        Some("File not found: spec/features/nope.feature"),
        "inner error must equal canonical TS message; got {data}"
    );
}

#[test]
fn scenario_returns_invalid_gherkin_syntax_error_for_malformed_feature() {
    // Scenario: Returns 'Invalid Gherkin syntax' error when feature file is malformed

    // @step Given the project root contains spec/features/broken.feature whose content is a Scenario keyword line with no preceding Feature header
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/broken.feature",
        "Scenario: Orphan\n  Given x\n",
    );

    // @step When I dispatch list-scenario-tags with file='spec/features/broken.feature' and scenario='Anything' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/broken.feature",
            "scenario": "Anything",
            "format": "json"
        }),
    ));

    // @step Then DispatchResult.data parses to JSON with success=false, tags=[], and error starting with 'Invalid Gherkin syntax:'
    assert!(
        result.success,
        "OUTER dispatcher must succeed; got {result:?}"
    );
    assert!(result.error.is_none(), "outer error must be None");
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(false));
    assert_eq!(data["tags"].as_array().map(Vec::len), Some(0));
    let inner_err = data["error"].as_str().expect("inner error field present");
    assert!(
        inner_err.starts_with("Invalid Gherkin syntax:"),
        "inner error must start with 'Invalid Gherkin syntax:'; got: {inner_err}"
    );
}

#[test]
fn scenario_returns_scenario_not_found_error_for_absent_name() {
    // Scenario: Returns 'Scenario not found' error when scenario name is absent

    // @step Given the project root contains spec/features/login.feature with a Feature header 'User Login' and a single Scenario 'Login with valid credentials'
    let tmp = TempDir::new().expect("tempdir");
    let body = "Feature: User Login\n\n  Scenario: Login with valid credentials\n    Given I am on the login page\n";
    write_feature(tmp.path(), "spec/features/login.feature", body);

    // @step When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Nope' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/login.feature",
            "scenario": "Nope",
            "format": "json"
        }),
    ));

    // @step Then DispatchResult.data parses to JSON with success=false, tags=[], and error exactly equal to "Scenario 'Nope' not found in spec/features/login.feature"
    assert!(
        result.success,
        "OUTER dispatcher must succeed; got {result:?}"
    );
    assert!(result.error.is_none(), "outer error must be None");
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(false));
    assert_eq!(data["tags"].as_array().map(Vec::len), Some(0));
    assert_eq!(
        data["error"].as_str(),
        Some("Scenario 'Nope' not found in spec/features/login.feature"),
        "inner error must equal canonical TS message verbatim; got {data}"
    );
}

#[test]
fn scenario_returns_empty_tags_with_sentinel_message_when_scenario_has_no_tags() {
    // Scenario: Returns success with empty tags and sentinel message when scenario has no tags

    // @step Given the project root contains spec/features/login.feature with a Feature header 'User Login' and a Scenario 'Untagged Scenario' that has NO @-tag lines preceding it
    let tmp = TempDir::new().expect("tempdir");
    let body = "Feature: User Login\n\n  Scenario: Untagged Scenario\n    Given x\n";
    write_feature(tmp.path(), "spec/features/login.feature", body);

    // @step When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Untagged Scenario' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/login.feature",
            "scenario": "Untagged Scenario",
            "format": "json"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected outer success=true, got {result:?}"
    );
    let data = parse_data(&result.data);
    assert_eq!(
        data["success"].as_bool(),
        Some(true),
        "inner success=true; got {data}"
    );

    // @step Then the parsed JSON has tags array of length 0
    assert_eq!(data["tags"].as_array().map(Vec::len), Some(0));

    // @step Then the parsed JSON has message field equal to 'No tags found on this scenario'
    assert_eq!(
        data["message"].as_str(),
        Some("No tags found on this scenario")
    );
}

#[test]
fn scenario_returns_tags_array_with_at_prefix_preserved() {
    // Scenario: Returns tags array with leading '@' preserved when scenario has tags

    // @step Given the project root contains spec/features/login.feature with a Scenario 'Login with valid credentials' immediately preceded by tag line '@smoke @critical'
    let tmp = TempDir::new().expect("tempdir");
    let body = "Feature: User Login\n\n  @smoke @critical\n  Scenario: Login with valid credentials\n    Given x\n";
    write_feature(tmp.path(), "spec/features/login.feature", body);

    // @step When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Login with valid credentials' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/login.feature",
            "scenario": "Login with valid credentials",
            "format": "json"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected outer success=true, got {result:?}"
    );
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step Then the parsed JSON tags array equals ['@smoke','@critical']
    let tags = data["tags"].as_array().expect("tags array");
    assert_eq!(tags.len(), 2, "expected 2 tags, got: {tags:?}");
    assert_eq!(tags[0].as_str(), Some("@smoke"));
    assert_eq!(tags[1].as_str(), Some("@critical"));

    // @step Then the parsed JSON does NOT contain a top-level 'message' field
    assert!(
        data.get("message").is_none(),
        "expected NO top-level message field on tagged-success path; got: {}",
        result.data
    );
}

#[test]
fn scenario_show_categories_enriches_tags_with_categories() {
    // Scenario: showCategories enriches tags with category labels from spec/tags.json

    // @step Given the project root contains spec/features/login.feature with a Scenario 'Login with valid credentials' tagged '@smoke'
    let tmp = TempDir::new().expect("tempdir");
    let body =
        "Feature: User Login\n\n  @smoke\n  Scenario: Login with valid credentials\n    Given x\n";
    write_feature(tmp.path(), "spec/features/login.feature", body);

    // @step Given the project root contains spec/tags.json with a category 'Testing Tags' whose tags include {name:'@smoke'}
    let registry = r#"{
  "categories": [
    {
      "name": "Testing Tags",
      "tags": [ { "name": "@smoke" } ]
    }
  ]
}"#;
    write_tags_json(tmp.path(), registry);

    // @step When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Login with valid credentials' and showCategories=true and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/login.feature",
            "scenario": "Login with valid credentials",
            "showCategories": true,
            "format": "json"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected outer success=true, got {result:?}"
    );
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step Then the parsed JSON categorizedTags array contains exactly one entry with tag='@smoke' and category='Testing Tags'
    let cats = data["categorizedTags"]
        .as_array()
        .expect("categorizedTags array");
    assert_eq!(cats.len(), 1);
    assert_eq!(cats[0]["tag"].as_str(), Some("@smoke"));
    assert_eq!(cats[0]["category"].as_str(), Some("Testing Tags"));
}

#[test]
fn scenario_show_categories_labels_unknown_tags_as_unknown() {
    // Scenario: showCategories labels tags absent from the registry as 'Unknown'

    // @step Given the project root contains spec/features/login.feature with a Scenario 'Login with valid credentials' tagged '@custom'
    let tmp = TempDir::new().expect("tempdir");
    let body =
        "Feature: User Login\n\n  @custom\n  Scenario: Login with valid credentials\n    Given x\n";
    write_feature(tmp.path(), "spec/features/login.feature", body);

    // @step Given the project root contains spec/tags.json with a category 'Testing Tags' whose tags include {name:'@smoke'} (no '@custom')
    let registry = r#"{
  "categories": [
    {
      "name": "Testing Tags",
      "tags": [ { "name": "@smoke" } ]
    }
  ]
}"#;
    write_tags_json(tmp.path(), registry);

    // @step When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Login with valid credentials' and showCategories=true and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/login.feature",
            "scenario": "Login with valid credentials",
            "showCategories": true,
            "format": "json"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected outer success=true, got {result:?}"
    );
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step Then the parsed JSON categorizedTags array contains exactly one entry with tag='@custom' and category='Unknown'
    let cats = data["categorizedTags"]
        .as_array()
        .expect("categorizedTags array");
    assert_eq!(cats.len(), 1);
    assert_eq!(cats[0]["tag"].as_str(), Some("@custom"));
    assert_eq!(cats[0]["category"].as_str(), Some("Unknown"));
}

#[test]
fn scenario_show_categories_degrades_when_registry_missing() {
    // Scenario: showCategories degrades gracefully when spec/tags.json is missing

    // @step Given the project root contains spec/features/login.feature with a Scenario 'Login with valid credentials' tagged '@smoke'
    let tmp = TempDir::new().expect("tempdir");
    let body =
        "Feature: User Login\n\n  @smoke\n  Scenario: Login with valid credentials\n    Given x\n";
    write_feature(tmp.path(), "spec/features/login.feature", body);

    // @step Given the project root has NO spec/tags.json file
    assert!(!tmp.path().join("spec/tags.json").exists());

    // @step When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Login with valid credentials' and showCategories=true and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/login.feature",
            "scenario": "Login with valid credentials",
            "showCategories": true,
            "format": "json"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected outer success=true, got {result:?}"
    );
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step Then the parsed JSON tags array equals ['@smoke']
    let tags = data["tags"].as_array().expect("tags array");
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].as_str(), Some("@smoke"));

    // @step Then the parsed JSON does NOT contain a top-level 'categorizedTags' field
    assert!(
        data.get("categorizedTags").is_none(),
        "missing-registry must NOT include categorizedTags; got: {}",
        result.data
    );
}

#[test]
fn scenario_show_categories_degrades_when_registry_invalid() {
    // Scenario: showCategories degrades gracefully when spec/tags.json is invalid JSON

    // @step Given the project root contains spec/features/login.feature with a Scenario 'Login with valid credentials' tagged '@smoke'
    let tmp = TempDir::new().expect("tempdir");
    let body =
        "Feature: User Login\n\n  @smoke\n  Scenario: Login with valid credentials\n    Given x\n";
    write_feature(tmp.path(), "spec/features/login.feature", body);

    // @step Given the project root contains spec/tags.json with the malformed bytes '{ not json'
    write_tags_json(tmp.path(), "{ not json");

    // @step When I dispatch list-scenario-tags with file='spec/features/login.feature' and scenario='Login with valid credentials' and showCategories=true and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/login.feature",
            "scenario": "Login with valid credentials",
            "showCategories": true,
            "format": "json"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected outer success=true, got {result:?}"
    );
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step Then the parsed JSON tags array equals ['@smoke']
    let tags = data["tags"].as_array().expect("tags array");
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].as_str(), Some("@smoke"));

    // @step Then the parsed JSON does NOT contain a top-level 'categorizedTags' field
    assert!(
        data.get("categorizedTags").is_none(),
        "invalid-registry must NOT include categorizedTags; got: {}",
        result.data
    );
}

#[test]
fn scenario_excludes_scenarios_nested_inside_rule_blocks() {
    // Scenario: Excludes Scenarios nested inside Rule: blocks

    // @step Given the project root contains spec/features/rules.feature with a Feature header and a Rule block named 'AuthRule' whose nested Scenario is 'Login with valid credentials'
    let tmp = TempDir::new().expect("tempdir");
    let body = "Feature: F\n\n  Rule: AuthRule\n\n    Scenario: Login with valid credentials\n      Given x\n";
    write_feature(tmp.path(), "spec/features/rules.feature", body);

    // @step When I dispatch list-scenario-tags with file='spec/features/rules.feature' and scenario='Login with valid credentials' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/rules.feature",
            "scenario": "Login with valid credentials",
            "format": "json"
        }),
    ));

    // @step Then DispatchResult.data parses to JSON with success=false and error containing the substring "Scenario 'Login with valid credentials' not found"
    assert!(
        result.success,
        "OUTER dispatcher must succeed; got {result:?}"
    );
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(false));
    let inner_err = data["error"].as_str().unwrap_or_default();
    assert!(
        inner_err.contains("Scenario 'Login with valid credentials' not found"),
        "expected scenario-not-found (Rule-nested excluded); got: {inner_err}"
    );
}

#[test]
fn scenario_excludes_scenario_outline_keyword() {
    // Scenario: Excludes Scenario Outline keyword (only matches plain Scenario)

    // @step Given the project root contains spec/features/outline.feature with a Feature header and a single 'Scenario Outline: Login with valid credentials' header (no plain Scenario by that name)
    let tmp = TempDir::new().expect("tempdir");
    let body = "Feature: F\n\n  Scenario Outline: Login with valid credentials\n    Given <user>\n\n    Examples:\n      | user |\n      | a    |\n";
    write_feature(tmp.path(), "spec/features/outline.feature", body);

    // @step When I dispatch list-scenario-tags with file='spec/features/outline.feature' and scenario='Login with valid credentials' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/outline.feature",
            "scenario": "Login with valid credentials",
            "format": "json"
        }),
    ));

    // @step Then DispatchResult.data parses to JSON with success=false and error containing the substring "Scenario 'Login with valid credentials' not found"
    assert!(
        result.success,
        "OUTER dispatcher must succeed; got {result:?}"
    );
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(false));
    let inner_err = data["error"].as_str().unwrap_or_default();
    assert!(
        inner_err.contains("Scenario 'Login with valid credentials' not found"),
        "expected scenario-not-found (Scenario Outline excluded); got: {inner_err}"
    );
}
