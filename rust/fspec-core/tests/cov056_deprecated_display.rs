#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/deprecated-scenario-report-marking.feature
//
// Validates the show-coverage rendering acceptance criteria for the
// @deprecated coverage-exemption mechanism (COV-056). Each scenario maps to
// exactly one #[test] function with @step comments mirroring the Gherkin
// steps verbatim.
//
// RED PHASE: show-coverage does not yet mark deprecated scenarios, so all
// scenarios below fail now — that is the correct red state.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "show-coverage".to_string(),
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

/// Feature + coverage fixture: `feature_body` is the .feature content after the
/// `@COV-056` tag line (Feature: header + scenarios included).
fn fixture(root: &Path, feature_body: &str, coverage_scenarios: &str) {
    write_file(root, "spec/features/dep-feature.feature", &format!("@COV-056\n{feature_body}\n"));
    write_file(
        root,
        "spec/features/dep-feature.feature.coverage",
        &format!(r#"{{ "scenarios": [{coverage_scenarios}] }}"#),
    );
    // Referenced files so no file-not-found warnings pollute the output.
    write_file(root, "tests/t.rs", "// @step step one\n");
    write_file(root, "src/s.rs", "fn live() {}\n");
}

const TWO_SCENARIOS: &str = r#"{ "name": "Live behavior", "testMappings": [{ "file": "tests/t.rs", "lines": "1-2", "implMappings": [{ "file": "src/s.rs", "lines": "1-3" }] }] },
   { "name": "Stale behavior", "testMappings": [] }"#;

const FEATURE_TWO_SCENARIOS: &str = "Feature: dep-feature\n\n  Scenario: Live behavior\n    Given a live precondition\n    When the live action happens\n    Then the live outcome is observed\n\n  @deprecated\n  Scenario: Stale behavior\n    Given a stale precondition\n    When the stale action happens\n    Then the stale outcome is observed\n";

#[test]
fn deprecated_marker_appears_in_markdown_report() {
    // Scenario: Deprecated marker appears in the show-coverage markdown report

    // @step Given a coverage file with one live covered scenario and one scenario tagged @deprecated in its feature file
    let tmp = TempDir::new().expect("tempdir");
    fixture(tmp.path(), FEATURE_TWO_SCENARIOS, TWO_SCENARIOS);

    // @step When the show-coverage command renders the feature in markdown
    let result = dispatch_command(req(tmp.path(), json!({"featureName": "dep-feature", "format": "markdown"})));
    assert!(result.success, "expected success; got {result:?}");
    let out = result.data.as_str();

    // @step Then the deprecated scenario heading carries a DEPRECATED marker
    let stale_heading = out
        .lines()
        .find(|l| l.contains("Stale behavior"))
        .expect("Stale behavior scenario must be rendered");
    assert!(
        stale_heading.contains("DEPRECATED"),
        "deprecated scenario heading must carry a DEPRECATED marker; got: {stale_heading}"
    );

    // @step And the live scenario heading has no DEPRECATED marker
    let live_heading = out
        .lines()
        .find(|l| l.contains("Live behavior"))
        .expect("Live behavior scenario must be rendered");
    assert!(
        !live_heading.contains("DEPRECATED"),
        "live scenario heading must not carry a DEPRECATED marker; got: {live_heading}"
    );

    // @step And the coverage percentage is computed from all scenarios including the deprecated one
    assert!(
        out.contains("**Coverage**: 50% (1/2 scenarios)"),
        "stats must count the deprecated scenario; got:\n{out}"
    );
}

#[test]
fn deprecated_marker_appears_in_json_report() {
    // Scenario: Deprecated marker appears in the show-coverage JSON report

    // @step Given a coverage file with one scenario tagged @deprecated in its feature file
    let tmp = TempDir::new().expect("tempdir");
    fixture(tmp.path(), FEATURE_TWO_SCENARIOS, TWO_SCENARIOS);

    // @step When the show-coverage command renders the feature in JSON
    let result = dispatch_command(req(tmp.path(), json!({"featureName": "dep-feature", "format": "json"})));
    assert!(result.success, "expected success; got {result:?}");
    let payload: Value =
        serde_json::from_str(result.data.as_str()).expect("output must be valid JSON");

    let scenarios = payload["scenarios"]
        .as_array()
        .expect("scenarios array");
    let stale = scenarios
        .iter()
        .find(|s| s["name"].as_str() == Some("Stale behavior"))
        .expect("Stale behavior entry");

    // @step Then the deprecated scenario entry carries a deprecated flag
    assert!(
        stale["deprecated"].as_bool() == Some(true),
        "deprecated scenario entry must carry a deprecated flag; got: {stale}"
    );

    // @step And its coverageStatus keeps its normal value
    assert_eq!(
        stale["coverageStatus"].as_str(),
        Some("uncovered"),
        "deprecated marker must not replace the coverageStatus; got: {stale}"
    );

    // @step And the stats block counts the deprecated scenario in totalScenarios
    assert_eq!(payload["stats"]["totalScenarios"], 2);
}

#[test]
fn untagged_feature_files_render_byte_identical_output() {
    // Scenario: Untagged feature files render byte-identical output

    // @step Given a coverage file whose feature file carries no @deprecated tag anywhere
    let tmp = TempDir::new().expect("tempdir");
    fixture(
        tmp.path(),
        "Feature: dep-feature\n\n  Scenario: Live behavior\n    Given a live precondition\n    When the live action happens\n    Then the live outcome is observed\n\n  Scenario: Stale behavior\n    Given a stale precondition\n    When the stale action happens\n    Then the stale outcome is observed\n",
        TWO_SCENARIOS,
    );

    // @step When the show-coverage command renders the feature in markdown
    let result = dispatch_command(req(tmp.path(), json!({"featureName": "dep-feature", "format": "markdown"})));
    assert!(result.success, "expected success; got {result:?}");
    let out = result.data.as_str();

    // @step Then the output is byte-identical to the pre-deprecation rendering
    // Pre-deprecation rendering: per-scenario heading is "### <symbol> <name> (<label>)"
    // with no DEPRECATED marker anywhere in the document.
    assert!(
        !out.contains("DEPRECATED"),
        "no DEPRECATED marker may appear in output for untagged features; got:\n{out}"
    );
    let stale_heading = out
        .lines()
        .find(|l| l.contains("Stale behavior"))
        .expect("Stale behavior scenario must be rendered");
    assert!(
        stale_heading.starts_with("### ") && stale_heading.contains("(UNCOVERED)"),
        "heading shape must be unchanged: '### ❌ Stale behavior (UNCOVERED)'; got: {stale_heading}"
    );
}
