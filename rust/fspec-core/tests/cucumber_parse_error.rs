#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/cucumber-compatible-gherkin-parse-error-formatting.feature
//
// RPC-329 PHASE B (TESTING): these tests assert cucumber-compatible parse-error
// formatting for the no-Feature-keyword malformed-file class. The shared
// formatter format_parse_error_cucumber + the validate.rs delegation do NOT yet
// exist, so every assertion below is RED. Each scenario maps to exactly one
// #[test] fn with @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────── helpers (copied from tests/validate.rs) ─────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "validate".to_string(),
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

fn data_json(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("data not JSON: {e}; got:\n{}", result.data))
}

// ───────── scenarios ─────────

#[test]
fn no_feature_keyword_file_reports_cucumber_vocabulary_on_line_0() {
    // Scenario: Validate a no-Feature-keyword file reports cucumber vocabulary on Line 0

    // @step Given a feature file whose content is 'Scenario: orphaned' then '  Given x' then '  Then y' with no Feature keyword
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/orphaned.feature",
        "Scenario: orphaned\n  Given x\n  Then y\n",
    );

    // @step When I dispatch the validate command against that single file
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/orphaned.feature"}),
    ));
    let data = data_json(&result);
    let output = data["output"].as_str().expect("output string");

    // @step Then the rendered output marks the file invalid with a 'Line 0:' detail
    assert!(
        output.contains('✗') && output.contains(" has syntax errors:"),
        "missing invalid marker; got:\n{output}"
    );
    assert!(
        output.contains("Line 0:"),
        "missing 'Line 0:' detail; got:\n{output}"
    );

    // @step Then the rendered output contains 'Parser errors:'
    assert!(
        output.contains("Parser errors:"),
        "missing 'Parser errors:'; got:\n{output}"
    );

    // @step Then the rendered output contains "(1:1): expected: #EOF, #Language, #TagLine, #FeatureLine, #Comment, #Empty, got 'Scenario: orphaned'"
    assert!(
        output.contains(
            "(1:1): expected: #EOF, #Language, #TagLine, #FeatureLine, #Comment, #Empty, got 'Scenario: orphaned'"
        ),
        "missing (1:1) entry; got:\n{output}"
    );

    // @step Then the rendered output contains "(2:3): expected: #EOF, #Language, #TagLine, #FeatureLine, #Comment, #Empty, got 'Given x'"
    assert!(
        output.contains(
            "(2:3): expected: #EOF, #Language, #TagLine, #FeatureLine, #Comment, #Empty, got 'Given x'"
        ),
        "missing (2:3) entry; got:\n{output}"
    );

    // @step Then the rendered output contains the line 'Suggestion: Add Feature keyword at the beginning of the file'
    assert!(
        output.contains("Suggestion: Add Feature keyword at the beginning of the file"),
        "missing Suggestion line; got:\n{output}"
    );

    // @step Then the dispatcher result reports an exit code of 1
    assert_eq!(data["exitCode"].as_i64(), Some(1), "got data: {data}");
}

#[test]
fn no_feature_keyword_file_with_unindented_step_keywords() {
    // Scenario: Validate a no-Feature-keyword file with unindented step keywords

    // @step Given a feature file whose content is 'When something' then 'Given nothing' with no leading whitespace and no Feature keyword
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/unindented.feature",
        "When something\nGiven nothing\n",
    );

    // @step When I dispatch the validate command against that single file
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/unindented.feature"}),
    ));
    let data = data_json(&result);
    let output = data["output"].as_str().expect("output string");

    // @step Then the rendered output marks the file invalid with a 'Line 0:' detail
    assert!(
        output.contains('✗') && output.contains(" has syntax errors:"),
        "missing invalid marker; got:\n{output}"
    );
    assert!(
        output.contains("Line 0:"),
        "missing 'Line 0:' detail; got:\n{output}"
    );

    // @step Then the rendered output contains "(1:1): expected: #EOF, #Language, #TagLine, #FeatureLine, #Comment, #Empty, got 'When something'"
    assert!(
        output.contains(
            "(1:1): expected: #EOF, #Language, #TagLine, #FeatureLine, #Comment, #Empty, got 'When something'"
        ),
        "missing (1:1) entry; got:\n{output}"
    );

    // @step Then the rendered output contains "(2:1): expected: #EOF, #Language, #TagLine, #FeatureLine, #Comment, #Empty, got 'Given nothing'"
    assert!(
        output.contains(
            "(2:1): expected: #EOF, #Language, #TagLine, #FeatureLine, #Comment, #Empty, got 'Given nothing'"
        ),
        "missing (2:1) entry; got:\n{output}"
    );

    // @step Then the rendered output contains the line 'Suggestion: Add Feature keyword at the beginning of the file'
    assert!(
        output.contains("Suggestion: Add Feature keyword at the beginning of the file"),
        "missing Suggestion line; got:\n{output}"
    );
}

#[test]
fn file_with_feature_keyword_that_fails_later_stays_out_of_scope() {
    // Scenario: A file with a Feature keyword that fails later stays out of scope

    // @step Given a feature file that begins with a valid 'Feature:' line but then contains a malformed construct the parser rejects
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/later-fail.feature",
        "Feature: X\n  Scenario: S\n    Given a\n  |bad table\n",
    );

    // @step When I dispatch the validate command against that single file
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/later-fail.feature"}),
    ));
    let data = data_json(&result);
    let output = data["output"].as_str().expect("output string");

    // @step Then the rendered output marks the file invalid
    assert!(
        output.contains('✗') && output.contains(" has syntax errors:"),
        "malformed feature must be marked invalid; got:\n{output}"
    );

    // @step Then the rendered output does NOT contain 'Parser errors:'
    assert!(
        !output.contains("Parser errors:"),
        "in-scope-only: file WITH Feature keyword must keep gherkin-0.16 text; got:\n{output}"
    );

    // @step Then the dispatcher result reports an exit code of 1
    assert_eq!(data["exitCode"].as_i64(), Some(1), "got data: {data}");
}
