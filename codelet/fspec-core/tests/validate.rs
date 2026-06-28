#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/validate-gherkin-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `validate`
// (RPC-320). Each scenario maps to exactly one #[test] fn with @step
// comments mirroring the Gherkin steps verbatim.
//
// PHASE B (TESTING): the core impl is still a 1-arg NotYetPorted stub, so
// every dispatch returns FspecCoreError::NotYetPorted. These tests are RED
// until PHASE C lands the real impl + the supervisor re-points the dispatcher.
//
// RPC-329 KNOWN DIVERGENCE: the embedded raw gherkin parser-error TEXT differs
// from @cucumber/gherkin. Per supervisor decision these tests assert ONLY
// structural facts (valid/invalid markers, exit code, 'Line ' presence,
// summary lines) and parser-independent content-heuristic messages — never the
// exact raw parser message.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────── helpers ─────────

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

/// Parse the dispatcher `data` field as the structured `{success, output,
/// exitCode, ...}` JSON envelope the command emits.
fn data_json(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("data not JSON: {e}; got:\n{}", result.data))
}

const VALID_A: &str = "Feature: A\n\n  Scenario: A1\n    Given x\n    When y\n    Then z\n";
const VALID_B: &str = "Feature: B\n\n  Scenario: B1\n    Given p\n    When q\n    Then r\n";
const VALID_LOGIN: &str =
    "Feature: Login\n\n  Scenario: Valid login\n    Given I am on the login page\n    When I submit credentials\n    Then I see the dashboard\n";
// Definitively non-parseable gherkin (no Feature keyword, raw prose).
const BROKEN: &str = "this is not gherkin";
// Otherwise-shaped feature carrying four consecutive blank lines (>2 → heuristic).
const FOUR_BLANKS: &str = "Feature: Blanks\n\n  Scenario: A\n    Given x\n\n\n\n\n    Then y\n";

// ───────── scenarios ─────────

#[test]
fn validates_all_feature_files_and_reports_an_all_valid_summary() {
    // Scenario: Validates all feature files and reports an all-valid summary

    // @step Given spec/features/ contains two syntactically valid feature files
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "spec/features/a.feature", VALID_A);
    write_file(tmp.path(), "spec/features/b.feature", VALID_B);

    // @step When I dispatch the validate command against that project root with no file argument
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected dispatch envelope success; got {result:?}"
    );
    let data = data_json(&result);
    let output = data["output"].as_str().expect("output string");

    // @step Then the rendered output contains a '✓ <file> is valid' line for each file
    assert!(
        output.contains("✓ spec/features/a.feature is valid"),
        "missing valid line for a.feature; got:\n{output}"
    );
    assert!(
        output.contains("✓ spec/features/b.feature is valid"),
        "missing valid line for b.feature; got:\n{output}"
    );

    // @step Then the rendered output contains the summary line '✓ All 2 feature files are valid'
    assert!(
        output.contains("✓ All 2 feature files are valid"),
        "missing all-valid summary; got:\n{output}"
    );
}

#[test]
fn validates_a_single_valid_file_with_no_summary_line() {
    // Scenario: Validates a single valid file with no summary line

    // @step Given spec/features/login.feature is a syntactically valid feature file
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "spec/features/login.feature", VALID_LOGIN);

    // @step When I dispatch the validate command against that project root with the file argument 'spec/features/login.feature'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected dispatch envelope success; got {result:?}"
    );
    let data = data_json(&result);
    let output = data["output"].as_str().expect("output string");

    // @step Then the rendered output is exactly '✓ spec/features/login.feature is valid'
    assert_eq!(
        output, "✓ spec/features/login.feature is valid",
        "single-file output must be exact; got:\n{output}"
    );

    // @step Then the rendered output does NOT contain the substring 'feature files are valid'
    assert!(
        !output.contains("feature files are valid"),
        "single-file path must skip the >1 summary; got:\n{output}"
    );
}

#[test]
fn marks_a_syntactically_broken_file_and_reports_a_mixed_summary() {
    // Scenario: Marks a syntactically broken file and reports a mixed summary

    // @step Given spec/features/ contains one valid file and one file with broken Gherkin syntax
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "spec/features/good.feature", VALID_A);
    write_file(tmp.path(), "spec/features/broken.feature", BROKEN);

    // @step When I dispatch the validate command against that project root with no file argument
    let result = dispatch_command(req(tmp.path(), json!({})));
    let data = data_json(&result);
    let output = data["output"].as_str().expect("output string");

    // @step Then the rendered output contains the substring '✗' followed by ' has syntax errors:'
    assert!(
        output.contains('✗') && output.contains(" has syntax errors:"),
        "missing invalid marker; got:\n{output}"
    );

    // @step Then the rendered output contains a 'Line ' detail for the broken file
    assert!(
        output.contains("Line "),
        "missing 'Line ' detail; got:\n{output}"
    );

    // @step Then the rendered output contains the summary line 'Validated 2 files: 1 valid, 1 invalid'
    assert!(
        output.contains("Validated 2 files: 1 valid, 1 invalid"),
        "missing mixed summary; got:\n{output}"
    );

    // @step Then the dispatcher result reports an exit code of 1
    assert_eq!(data["exitCode"].as_i64(), Some(1), "got data: {data}");
}

#[test]
fn reports_no_feature_files_found_when_spec_features_is_empty() {
    // Scenario: Reports no feature files found when spec/features is empty

    // @step Given spec/features/ exists but contains zero .feature files
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("spec/features")).expect("mkdir spec/features");

    // @step When I dispatch the validate command against that project root with no file argument
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns an error containing the substring 'No feature files found in spec/features/'
    // (zero-feature-files → Err(FspecCoreError) → bridge maps to exit 2.)
    assert!(!result.success, "expected dispatch failure; got {result:?}");
    let err = result.error.as_deref().unwrap_or_default();
    assert!(
        err.contains("No feature files found in spec/features/"),
        "expected no-files message; got: {err}"
    );

    // @step Then the dispatcher result reports an exit code of 2
    // The zero-feature-files path returns Err so the shell bridge maps it to
    // process exit code 2 (mirrors list-features DirectoryNotFound→2). The
    // Err path is the structural proxy for the exit-2 mapping asserted here.
    assert!(
        result.data.is_empty(),
        "Err path must carry no JSON data payload; got:\n{}",
        result.data
    );
}

#[test]
fn flags_excessive_consecutive_blank_lines_via_the_content_heuristic() {
    // Scenario: Flags excessive consecutive blank lines via the content heuristic

    // @step Given a feature file containing four consecutive blank lines
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "spec/features/blanks.feature", FOUR_BLANKS);

    // @step When I dispatch the validate command against that single file
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/blanks.feature"}),
    ));
    let data = data_json(&result);
    let output = data["output"].as_str().expect("output string");

    // @step Then the rendered output marks the file invalid
    assert!(
        output.contains('✗') && output.contains(" has syntax errors:"),
        "blank-line file must be marked invalid; got:\n{output}"
    );

    // @step Then the rendered output contains the substring 'Excessive blank lines detected'
    assert!(
        output.contains("Excessive blank lines detected"),
        "missing content-heuristic message; got:\n{output}"
    );

    // @step Then the dispatcher result reports an exit code of 1
    assert_eq!(data["exitCode"].as_i64(), Some(1), "got data: {data}");
}
