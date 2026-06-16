#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/format-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `format`
// (RPC-230). Each scenario maps to one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim.
//
// PHASE B (TESTING): the core impl is still a stub, so every dispatch
// returns FspecCoreError::NotYetPorted. The behavioural tests are RED
// until PHASE C.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "format".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_feature(project_root: &Path, rel: &str, body: &str) {
    let abs = project_root.join(rel);
    fs::create_dir_all(abs.parent().unwrap()).expect("mkdir feature parent");
    fs::write(&abs, body).expect("write feature file");
}

fn read_feature(project_root: &Path, rel: &str) -> String {
    fs::read_to_string(project_root.join(rel)).expect("read feature file")
}

/// A well-formed but non-canonically-indented feature (steps under-indented
/// to 2 spaces; the formatter must renormalise them to 4 spaces).
fn messy_feature(name: &str) -> String {
    format!("Feature: {name}\n\n  Scenario: A\n  Given x\n  When y\n  Then z\n")
}

fn data_of(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data).expect("parse data json")
}

fn dispatcher_error(result: &codelet_fspec_core::DispatchResult) -> String {
    let data: Value = serde_json::from_str(&result.data).unwrap_or(Value::Null);
    result
        .error
        .as_deref()
        .map(str::to_string)
        .or_else(|| data["error"].as_str().map(str::to_string))
        .unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher formats all feature files in a workspace
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_formats_all_feature_files_in_a_workspace() {
    // @step Given a project root tempdir with two well-formed feature files under spec/features
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/one.feature", &messy_feature("One"));
    write_feature(tmp.path(), "spec/features/two.feature", &messy_feature("Two"));

    // @step When I dispatch format with no file argument
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "expected success=true, got {result:?}");
    let data = data_of(&result);

    // @step Then the dispatcher returns formattedCount=2
    assert_eq!(data["formattedCount"].as_u64(), Some(2), "got data: {data}");

    // @step And both feature files are rewritten in the canonical formatter layout
    let one = read_feature(tmp.path(), "spec/features/one.feature");
    let two = read_feature(tmp.path(), "spec/features/two.feature");
    assert!(one.contains("    Given x"), "one not canonicalised:\n{one}");
    assert!(two.contains("    Given x"), "two not canonicalised:\n{two}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher formats a single supplied file
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_formats_a_single_supplied_file() {
    // @step Given a project root tempdir with two feature files under spec/features
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/one.feature", &messy_feature("One"));
    write_feature(tmp.path(), "spec/features/two.feature", &messy_feature("Two"));
    let two_before = read_feature(tmp.path(), "spec/features/two.feature");

    // @step When I dispatch format with file=spec/features/one.feature
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/one.feature"}),
    ));
    assert!(result.success, "expected success=true, got {result:?}");
    let data = data_of(&result);

    // @step Then the dispatcher returns formattedCount=1
    assert_eq!(data["formattedCount"].as_u64(), Some(1), "got data: {data}");

    // @step And only that file is rewritten
    let one = read_feature(tmp.path(), "spec/features/one.feature");
    let two_after = read_feature(tmp.path(), "spec/features/two.feature");
    assert!(one.contains("    Given x"), "one not canonicalised:\n{one}");
    assert_eq!(two_before, two_after, "the other file must be untouched");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher returns zero when no feature files exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_returns_zero_when_no_feature_files_exist() {
    // @step Given a project root tempdir with no feature files under spec/features
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch format with no file argument
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "expected success=true, got {result:?}");
    let data = data_of(&result);

    // @step Then the dispatcher returns formattedCount=0
    assert_eq!(data["formattedCount"].as_u64(), Some(0), "got data: {data}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher errors when the supplied file does not exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_errors_when_the_supplied_file_does_not_exist() {
    // @step Given a project root tempdir with no spec/features/missing.feature file
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch format with file=spec/features/missing.feature
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/missing.feature"}),
    ));

    // @step Then the dispatcher returns an error mentioning 'File not found'
    assert!(!result.success, "expected dispatch failure; got {result:?}");
    let err = dispatcher_error(&result);
    assert!(
        err.contains("File not found"),
        "expected 'File not found' in error; got: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher output is idempotent
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_output_is_idempotent() {
    // @step Given a project root tempdir with one feature file that is already canonically formatted
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/one.feature", &messy_feature("One"));
    // Canonicalise once so the file is in formatter layout before the assertion run.
    let _ = dispatch_command(req(tmp.path(), json!({})));
    let after_first = read_feature(tmp.path(), "spec/features/one.feature");
    assert!(
        after_first.contains("    Given x"),
        "precondition: file must be canonicalised after the first format; got:\n{after_first}"
    );

    // @step When I dispatch format twice over that file
    let _ = dispatch_command(req(tmp.path(), json!({})));
    let after_second = read_feature(tmp.path(), "spec/features/one.feature");

    // @step Then the file content is identical after both runs
    assert_eq!(
        after_first, after_second,
        "format must be idempotent: second run changed the file"
    );
}
