#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/delete-features-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `delete-features`
// (RPC-218). Each scenario maps to one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim.
//
// PHASE B (TESTING): the core impl is still a stub, so every dispatch
// returns FspecCoreError::NotYetPorted. These tests are RED until PHASE C.

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
        command: "delete-features".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_feature(project_root: &Path, rel: &str, body: &str) {
    let abs = project_root.join(rel);
    fs::create_dir_all(abs.parent().unwrap()).expect("mkdir feature parent");
    fs::write(&abs, body).expect("write feature file");
}

fn exists(project_root: &Path, rel: &str) -> bool {
    project_root.join(rel).exists()
}

fn tagged(tags: &str, name: &str) -> String {
    format!("{tags}\nFeature: {name}\n\n  Scenario: A\n    Given x\n")
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
// Scenario: Dispatcher deletes features carrying a single tag
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_deletes_features_carrying_a_single_tag() {
    // @step Given a project root tempdir with three features, two tagged @deprecated
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        &tagged("@deprecated", "A"),
    );
    write_feature(
        tmp.path(),
        "spec/features/b.feature",
        &tagged("@deprecated", "B"),
    );
    write_feature(tmp.path(), "spec/features/c.feature", &tagged("@keep", "C"));

    // @step When I dispatch delete-features with tags=['@deprecated']
    let result = dispatch_command(req(tmp.path(), json!({"tags": ["@deprecated"]})));

    // @step Then the dispatcher returns success=true and deletedCount=2
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["deletedCount"].as_u64(), Some(2), "got data: {data}");

    // @step And the two @deprecated feature files no longer exist on disk
    assert!(!exists(tmp.path(), "spec/features/a.feature"));
    assert!(!exists(tmp.path(), "spec/features/b.feature"));

    // @step And the untagged feature file still exists
    assert!(exists(tmp.path(), "spec/features/c.feature"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher applies AND logic across multiple tags
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_applies_and_logic_across_multiple_tags() {
    // @step Given a project root tempdir with one feature tagged @critical and @spike and one feature tagged only @critical
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/both.feature",
        &tagged("@critical @spike", "Both"),
    );
    write_feature(
        tmp.path(),
        "spec/features/onecrit.feature",
        &tagged("@critical", "OneCrit"),
    );

    // @step When I dispatch delete-features with tags=['@critical','@spike']
    let result = dispatch_command(req(tmp.path(), json!({"tags": ["@critical", "@spike"]})));

    // @step Then the dispatcher returns success=true and deletedCount=1
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["deletedCount"].as_u64(), Some(1), "got data: {data}");

    // @step And only the feature carrying both tags is deleted
    assert!(!exists(tmp.path(), "spec/features/both.feature"));
    assert!(exists(tmp.path(), "spec/features/onecrit.feature"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher dry-run leaves all files on disk
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_dry_run_leaves_all_files_on_disk() {
    // @step Given a project root tempdir with two features tagged @deprecated
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        &tagged("@deprecated", "A"),
    );
    write_feature(
        tmp.path(),
        "spec/features/b.feature",
        &tagged("@deprecated", "B"),
    );

    // @step When I dispatch delete-features with tags=['@deprecated'] and dryRun=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tags": ["@deprecated"], "dryRun": true}),
    ));

    // @step Then the dispatcher returns success=true and deletedCount=2
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["deletedCount"].as_u64(), Some(2), "got data: {data}");

    // @step And the dispatcher result lists the matching files
    let files = data["files"].as_array().expect("files array present");
    assert_eq!(files.len(), 2, "expected 2 listed files; got {data}");

    // @step And both feature files still exist on disk
    assert!(exists(tmp.path(), "spec/features/a.feature"));
    assert!(exists(tmp.path(), "spec/features/b.feature"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher rejects an empty tag list
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_rejects_an_empty_tag_list() {
    // @step Given a project root tempdir with a feature tagged @deprecated
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        &tagged("@deprecated", "A"),
    );

    // @step When I dispatch delete-features with tags=[]
    let result = dispatch_command(req(tmp.path(), json!({"tags": []})));

    // @step Then the dispatcher returns success=false
    // Per TS parity: delete-features-by-tag.ts RETURNS a result object (it does
    // not throw) for an empty tag list, so the dispatch envelope succeeds and the
    // failure is carried by the inner `success:false` field of the JSON payload.
    assert!(
        result.success,
        "expected dispatch envelope success; got {result:?}"
    );
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["success"].as_bool(),
        Some(false),
        "expected inner success=false; got {data}"
    );

    // @step And the dispatcher error equals 'At least one --tag is required'
    let err = dispatcher_error(&result);
    assert!(
        err.contains("At least one --tag is required"),
        "expected required-tag message; got: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher reports no matches for an unused tag
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_reports_no_matches_for_an_unused_tag() {
    // @step Given a project root tempdir with a feature tagged @deprecated
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        &tagged("@deprecated", "A"),
    );

    // @step When I dispatch delete-features with tags=['@notpresent']
    let result = dispatch_command(req(tmp.path(), json!({"tags": ["@notpresent"]})));

    // @step Then the dispatcher returns success=true and deletedCount=0
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["deletedCount"].as_u64(), Some(0), "got data: {data}");

    // @step And the dispatcher message equals 'No feature files found matching tags'
    let msg = data["message"].as_str().unwrap_or("");
    assert_eq!(msg, "No feature files found matching tags", "got: {msg}");
}

// ---------- scenarios moved from delete-features-by-tag (COV-072) ----------

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher deletes features whose tag contains special characters
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_deletes_features_whose_tag_contains_special_characters() {
    // @step Given a project root tempdir with three features tagged @bug-#123 and one feature tagged @bug-123
    let tmp = TempDir::new().expect("tempdir");
    for name in ["a", "b", "c"] {
        write_feature(
            tmp.path(),
            &format!("spec/features/{name}.feature"),
            &tagged("@bug-#123", name),
        );
    }
    write_feature(
        tmp.path(),
        "spec/features/d.feature",
        &tagged("@bug-123", "d"),
    );

    // @step When I dispatch delete-features with tags=['@bug-#123']
    let result = dispatch_command(req(tmp.path(), json!({"tags": ["@bug-#123"]})));

    // @step Then the dispatcher returns success=true and deletedCount=3
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["deletedCount"].as_u64(), Some(3), "got data: {data}");

    // @step And the dispatcher message equals 'Deleted 3 feature file(s)'
    assert_eq!(
        data["message"].as_str(),
        Some("Deleted 3 feature file(s)"),
        "got data: {data}"
    );

    // @step And the three @bug-#123 feature files no longer exist on disk
    for name in ["a", "b", "c"] {
        assert!(!exists(
            tmp.path(),
            &format!("spec/features/{name}.feature")
        ));
    }

    // @step And the feature tagged @bug-123 still exists
    assert!(exists(tmp.path(), "spec/features/d.feature"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher deletes tagged features in nested directories
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_deletes_tagged_features_in_nested_directories() {
    // @step Given a project root tempdir with features tagged @old at spec/features/top.feature, spec/features/area/mid.feature and spec/features/area/deep/low.feature, and an untagged spec/features/area/keep.feature
    let tmp = TempDir::new().expect("tempdir");
    let old = [
        "spec/features/top.feature",
        "spec/features/area/mid.feature",
        "spec/features/area/deep/low.feature",
    ];
    for rel in old {
        write_feature(tmp.path(), rel, &tagged("@old", rel));
    }
    write_feature(
        tmp.path(),
        "spec/features/area/keep.feature",
        &tagged("@keep", "Keep"),
    );

    // @step When I dispatch delete-features with tags=['@old']
    let result = dispatch_command(req(tmp.path(), json!({"tags": ["@old"]})));

    // @step Then the dispatcher returns success=true and deletedCount=3
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["deletedCount"].as_u64(), Some(3), "got data: {data}");

    // @step And the dispatcher result lists spec/features/top.feature, spec/features/area/mid.feature and spec/features/area/deep/low.feature
    assert_eq!(
        data["files"],
        json!(old),
        "files are listed files-before-subdirectories; got data: {data}"
    );

    // @step And the three @old feature files no longer exist on disk
    for rel in old {
        assert!(!exists(tmp.path(), rel), "{rel} must be deleted");
    }

    // @step And spec/features/area/keep.feature still exists
    assert!(exists(tmp.path(), "spec/features/area/keep.feature"));
}
