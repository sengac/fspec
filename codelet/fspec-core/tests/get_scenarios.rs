#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/get-scenarios-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `get-scenarios`
// (RPC-237). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// Red phase: the `get-scenarios` command is still a stub returning
// FspecCoreError::NotYetPorted, so every success-path assertion below FAILS
// until Phase C wires the real implementation.
//
// Dispatcher data shape (mirrors show-acceptance-criteria RPC-299): the run
// function returns a JSON envelope { success, scenarios, totalCount, message,
// warnings? } so the LLM sees structured data. The CLI bridge renders the
// format-specific body. These dispatcher tests assert against that envelope.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────── helpers ─────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "get-scenarios".to_string(),
        args_json: args.to_string(),
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

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

const TWO_SCENARIO_FEATURE: &str = "Feature: Sample\n\n  Scenario: First\n    Given a step\n    When another step\n    Then a result\n\n  Scenario: Second\n    Given a step\n    When another step\n    Then a result\n";

// ───────── scenarios ─────────

#[test]
fn dispatch_with_format_json_and_no_tags_returns_every_scenario_across_all_feature_files() {
    // Scenario: Dispatch with format json and no tags returns every scenario across all feature files

    // @step Given a project root contains three feature files under spec/features/ each with two scenarios
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "spec/features/a.feature", TWO_SCENARIO_FEATURE);
    write_file(tmp.path(), "spec/features/b.feature", TWO_SCENARIO_FEATURE);
    write_file(tmp.path(), "spec/features/c.feature", TWO_SCENARIO_FEATURE);

    // @step When I dispatch get-scenarios with format='json' and no tags
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);

    // @step Then the returned envelope has totalCount=6
    assert_eq!(data["totalCount"].as_u64(), Some(6));

    // @step Then the returned scenarios array has six elements
    assert_eq!(
        data["scenarios"].as_array().map(Vec::len),
        Some(6),
        "expected six scenarios; got:\n{}",
        result.data
    );
}

#[test]
fn dispatch_with_multiple_tags_applies_and_logic_against_the_feature_plus_scenario_tag_union() {
    // Scenario: Dispatch with multiple tags applies AND logic against the feature plus scenario tag union

    // @step Given a project root contains a feature tagged '@auth' with one scenario tagged '@smoke' and one scenario tagged '@regression'
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/auth.feature",
        "@auth\nFeature: Auth\n\n  @smoke\n  Scenario: Smoke login\n    Given a step\n    When another step\n    Then a result\n\n  @regression\n  Scenario: Regression login\n    Given a step\n    When another step\n    Then a result\n",
    );

    // @step When I dispatch get-scenarios with tags=['@auth','@smoke']
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "tags": ["@auth", "@smoke"], "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);

    // @step Then the returned scenarios array contains only the scenario whose tag union includes both @auth and @smoke
    let scenarios = data["scenarios"].as_array().expect("scenarios array");
    assert_eq!(scenarios.len(), 1, "expected one match; got:\n{}", result.data);
    assert_eq!(scenarios[0]["name"].as_str(), Some("Smoke login"));
}

#[test]
fn dispatch_with_a_tag_no_feature_carries_returns_a_zero_count_not_found_message() {
    // Scenario: Dispatch with a tag no feature carries returns a zero-count not-found message

    // @step Given a project root contains feature files none of which carry the '@deprecated' tag
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "spec/features/a.feature", TWO_SCENARIO_FEATURE);
    write_file(tmp.path(), "spec/features/b.feature", TWO_SCENARIO_FEATURE);

    // @step When I dispatch get-scenarios with tags=['@deprecated']
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "tags": ["@deprecated"], "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);

    // @step Then the returned envelope has totalCount=0
    assert_eq!(data["totalCount"].as_u64(), Some(0));

    // @step Then the returned envelope message equals 'No scenarios found matching tags: @deprecated'
    assert_eq!(
        data["message"].as_str(),
        Some("No scenarios found matching tags: @deprecated")
    );
}

#[test]
fn scenario_level_tags_are_emitted_while_a_scenario_with_no_own_tags_omits_the_tags_field() {
    // Scenario: Scenario-level tags are emitted while a scenario with no own tags omits the tags field

    // @step Given a project root contains a feature whose first scenario is tagged '@smoke' and '@critical' and whose second scenario has no scenario-level tags
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/mix.feature",
        "Feature: Mix\n\n  @smoke\n  @critical\n  Scenario: Tagged one\n    Given a step\n    When another step\n    Then a result\n\n  Scenario: Untagged two\n    Given a step\n    When another step\n    Then a result\n",
    );

    // @step When I dispatch get-scenarios with format='json' and no tags
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);
    let scenarios = data["scenarios"].as_array().expect("scenarios array");
    assert_eq!(scenarios.len(), 2, "expected two scenarios; got:\n{}", result.data);

    // @step Then the first scenario's tags field equals ['@smoke','@critical']
    assert_eq!(
        scenarios[0]["tags"],
        json!(["@smoke", "@critical"]),
        "first scenario tags mismatch; got:\n{}",
        result.data
    );

    // @step Then the second scenario omits its tags field
    assert!(
        scenarios[1].get("tags").is_none(),
        "second scenario must omit tags; got:\n{}",
        result.data
    );
}

#[test]
fn dispatch_against_a_project_root_with_no_spec_features_directory_returns_a_structured_not_found_error(
) {
    // Scenario: Dispatch against a project root with no spec/features directory returns a structured not-found error

    // @step Given a project root with no spec/features directory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features").exists());

    // @step When I dispatch get-scenarios with no tags
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'spec/features directory not found'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("spec/features directory not found"),
        "error message missing canonical substring: {msg}"
    );
}
