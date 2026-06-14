#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/retag-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `retag` (RPC-293).
// Each scenario maps to one #[test] fn with @step comments mirroring the
// Gherkin steps verbatim. These are dispatcher-contract tests driven through
// codelet_fspec_core::dispatch_command.
//
// PHASE B (TESTING): the core impl is still a stub, so every dispatch returns
// FspecCoreError::NotYetPorted (dispatch envelope success=false). These tests
// are RED until PHASE C.
//
// Envelope parity (matches delete_features.rs RPC-218): retag.ts RETURNS a
// RetagResult object (never throws), so the dispatch envelope succeeds and the
// real outcome is carried by the inner `success` / `error` JSON fields.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "retag".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_feature(project_root: &Path, rel: &str, body: &str) {
    let abs = project_root.join(rel);
    fs::create_dir_all(abs.parent().unwrap()).expect("mkdir feature parent");
    fs::write(&abs, body).expect("write feature file");
}

/// A valid Gherkin feature file whose feature-level tag is `tag`.
fn tagged(tag: &str, name: &str) -> String {
    format!("{tag}\nFeature: {name}\n\n  Scenario: A\n    Given x\n")
}

fn read(project_root: &Path, rel: &str) -> String {
    fs::read_to_string(project_root.join(rel)).expect("read feature file")
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

// ---------- scenarios ----------

#[test]
fn renaming_a_tag_across_two_feature_files_rewrites_every_occurrence() {
    // @step Given a project root tempdir with two spec/features feature files that each tag a scenario with @wip
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/a.feature", &tagged("@wip", "A"));
    write_feature(tmp.path(), "spec/features/b.feature", &tagged("@wip", "B"));

    // @step When I dispatch retag with from='@wip' and to='@in-progress'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"from": "@wip", "to": "@in-progress"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected dispatch envelope success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(true), "got data: {data}");

    // @step And the result reports fileCount=2
    assert_eq!(data["fileCount"].as_u64(), Some(2), "got data: {data}");

    // @step And neither feature file on disk contains the token '@wip' anymore
    assert!(!read(tmp.path(), "spec/features/a.feature").contains("@wip"));
    assert!(!read(tmp.path(), "spec/features/b.feature").contains("@wip"));

    // @step And both feature files on disk now contain the token '@in-progress'
    assert!(read(tmp.path(), "spec/features/a.feature").contains("@in-progress"));
    assert!(read(tmp.path(), "spec/features/b.feature").contains("@in-progress"));
}

#[test]
fn a_dry_run_reports_matches_but_leaves_every_file_byte_equal() {
    // @step Given a project root tempdir with two spec/features feature files that each tag a scenario with @wip
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/a.feature", &tagged("@wip", "A"));
    write_feature(tmp.path(), "spec/features/b.feature", &tagged("@wip", "B"));
    let pre_a = fs::read(tmp.path().join("spec/features/a.feature")).unwrap();
    let pre_b = fs::read(tmp.path().join("spec/features/b.feature")).unwrap();

    // @step When I dispatch retag with from='@wip', to='@in-progress' and dryRun=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"from": "@wip", "to": "@in-progress", "dryRun": true}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected dispatch envelope success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(true), "got data: {data}");

    // @step And the result reports fileCount=2 and a non-zero occurrenceCount
    assert_eq!(data["fileCount"].as_u64(), Some(2), "got data: {data}");
    assert!(
        data["occurrenceCount"].as_u64().unwrap_or(0) > 0,
        "occurrenceCount must be > 0; got data: {data}"
    );

    // @step And the result files array lists both matching feature files
    let files = data["files"].as_array().expect("files array present");
    assert_eq!(files.len(), 2, "expected 2 listed files; got {data}");

    // @step And both feature files on disk are byte-equal to their pre-call contents
    assert_eq!(
        fs::read(tmp.path().join("spec/features/a.feature")).unwrap(),
        pre_a
    );
    assert_eq!(
        fs::read(tmp.path().join("spec/features/b.feature")).unwrap(),
        pre_b
    );
}

#[test]
fn a_missing_to_is_rejected() {
    // @step Given a project root tempdir with one spec/features feature file tagged @wip
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/a.feature", &tagged("@wip", "A"));
    let pre = fs::read(tmp.path().join("spec/features/a.feature")).unwrap();

    // @step When I dispatch retag with from='@wip' and an empty to
    let result = dispatch_command(req(tmp.path(), json!({"from": "@wip", "to": ""})));

    // @step Then the dispatcher returns success=false
    assert!(result.success, "expected dispatch envelope success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(false), "got data: {data}");

    // @step And the error message is 'Both --from and --to are required'
    let err = dispatcher_error(&result);
    assert_eq!(err, "Both --from and --to are required", "got: {err}");

    // @step And the feature file on disk is byte-equal to its pre-call contents
    assert_eq!(
        fs::read(tmp.path().join("spec/features/a.feature")).unwrap(),
        pre
    );
}

#[test]
fn an_invalid_target_tag_format_is_rejected() {
    // @step Given a project root tempdir with one spec/features feature file tagged @wip
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/a.feature", &tagged("@wip", "A"));
    let pre = fs::read(tmp.path().join("spec/features/a.feature")).unwrap();

    // @step When I dispatch retag with from='@wip' and to='WIP'
    let result = dispatch_command(req(tmp.path(), json!({"from": "@wip", "to": "WIP"})));

    // @step Then the dispatcher returns success=false
    assert!(result.success, "expected dispatch envelope success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(false), "got data: {data}");

    // @step And the error message contains the substring 'Invalid tag format: "WIP". Valid format is @lowercase-with-hyphens'
    let err = dispatcher_error(&result);
    assert!(
        err.contains("Invalid tag format: \"WIP\". Valid format is @lowercase-with-hyphens"),
        "got: {err}"
    );

    // @step And the feature file on disk is byte-equal to its pre-call contents
    assert_eq!(
        fs::read(tmp.path().join("spec/features/a.feature")).unwrap(),
        pre
    );
}

#[test]
fn a_from_tag_present_in_no_feature_file_reports_the_not_found_error() {
    // @step Given a project root tempdir with one spec/features feature file tagged @wip
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/a.feature", &tagged("@wip", "A"));
    let pre = fs::read(tmp.path().join("spec/features/a.feature")).unwrap();

    // @step When I dispatch retag with from='@missing' and to='@found'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"from": "@missing", "to": "@found"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(result.success, "expected dispatch envelope success; got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(false), "got data: {data}");

    // @step And the error message is 'Tag @missing not found in any feature files'
    let err = dispatcher_error(&result);
    assert_eq!(err, "Tag @missing not found in any feature files", "got: {err}");

    // @step And the feature file on disk is byte-equal to its pre-call contents
    assert_eq!(
        fs::read(tmp.path().join("spec/features/a.feature")).unwrap(),
        pre
    );
}
