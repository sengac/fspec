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
    assert!(
        result.success,
        "expected dispatch envelope success; got {result:?}"
    );
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
    assert!(
        result.success,
        "expected dispatch envelope success; got {result:?}"
    );
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
    assert!(
        result.success,
        "expected dispatch envelope success; got {result:?}"
    );
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
    assert!(
        result.success,
        "expected dispatch envelope success; got {result:?}"
    );
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
    let result = dispatch_command(req(tmp.path(), json!({"from": "@missing", "to": "@found"})));

    // @step Then the dispatcher returns success=false
    assert!(
        result.success,
        "expected dispatch envelope success; got {result:?}"
    );
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(false), "got data: {data}");

    // @step And the error message is 'Tag @missing not found in any feature files'
    let err = dispatcher_error(&result);
    assert_eq!(
        err, "Tag @missing not found in any feature files",
        "got: {err}"
    );

    // @step And the feature file on disk is byte-equal to its pre-call contents
    assert_eq!(
        fs::read(tmp.path().join("spec/features/a.feature")).unwrap(),
        pre
    );
}

// ---------- scenarios moved from retag.feature (COV-071) ----------

fn retag_ok(project_root: &Path, from: &str, to: &str) -> Value {
    let result = dispatch_command(req(project_root, json!({"from": from, "to": to})));
    assert!(
        result.success,
        "expected dispatch envelope success; got {result:?}"
    );
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(true), "got data: {data}");
    data
}

fn assert_valid_gherkin(project_root: &Path, rel: &str) {
    let parsed = codelet_fspec_core::io::gherkin::parse_feature_lenient(&read(project_root, rel));
    assert!(parsed.is_ok(), "{rel} must re-parse as Gherkin: {parsed:?}");
}

#[test]
fn renaming_a_feature_level_tag_rewrites_only_the_tag_and_keeps_the_file_structure() {
    // @step Given a project root tempdir with a feature file tagged @legacy-api at feature level
    let tmp = TempDir::new().expect("tempdir");
    let before = "@legacy-api\n@cli\nFeature: A\n  \"\"\"\n  Notes\n  \"\"\"\n\n  Scenario: One\n    Given x\n    Then y\n";
    write_feature(tmp.path(), "spec/features/a.feature", before);

    // @step When I dispatch retag with from='@legacy-api' and to='@legacy'
    // @step Then the dispatcher returns success=true
    let data = retag_ok(tmp.path(), "@legacy-api", "@legacy");
    assert_eq!(
        data["occurrenceCount"].as_u64(),
        Some(1),
        "got data: {data}"
    );

    // @step And the feature-level tag should change from @legacy-api to @legacy
    let after = read(tmp.path(), "spec/features/a.feature");
    assert!(
        after.starts_with("@legacy\n@cli\nFeature: A\n"),
        "got: {after}"
    );
    assert!(!after.contains("@legacy-api"), "got: {after}");

    // @step And the file structure should be preserved
    assert_eq!(after, before.replacen("@legacy-api", "@legacy", 1));
}

#[test]
fn renaming_a_scenario_level_tag_leaves_feature_level_tags_unchanged() {
    // @step Given a project root tempdir with a feature file whose scenarios are tagged with @wip under a feature-level @api tag
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        "@api\nFeature: A\n\n  @wip\n  Scenario: One\n    Given x\n\n  @wip\n  Scenario: Two\n    Given y\n",
    );

    // @step When I dispatch retag with from='@wip' and to='@in-progress'
    // @step Then the dispatcher returns success=true
    let data = retag_ok(tmp.path(), "@wip", "@in-progress");
    assert_eq!(
        data["occurrenceCount"].as_u64(),
        Some(2),
        "got data: {data}"
    );

    // @step And all scenario-level @wip tags should change to @in-progress
    let after = read(tmp.path(), "spec/features/a.feature");
    assert_eq!(
        after.matches("  @in-progress\n  Scenario:").count(),
        2,
        "got: {after}"
    );
    assert!(!after.contains("@wip"), "got: {after}");

    // @step And feature-level tags should remain unchanged
    assert!(after.starts_with("@api\nFeature: A\n"), "got: {after}");
}

#[test]
fn renaming_a_tag_used_at_both_levels_rewrites_both_and_counts_each_occurrence() {
    // @step Given a project root tempdir with a feature file tagged @temp at feature level
    // @step And a scenario in that file tagged @temp at scenario level
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        "@temp\nFeature: A\n\n  @temp\n  Scenario: One\n    Given x\n",
    );

    // @step When I dispatch retag with from='@temp' and to='@temporary'
    // @step Then the dispatcher returns success=true
    let data = retag_ok(tmp.path(), "@temp", "@temporary");

    // @step And all @temp tags should change to @temporary at both levels
    assert_eq!(
        read(tmp.path(), "spec/features/a.feature"),
        "@temporary\nFeature: A\n\n  @temporary\n  Scenario: One\n    Given x\n"
    );

    // @step And the result reports occurrenceCount=2
    assert_eq!(
        data["occurrenceCount"].as_u64(),
        Some(2),
        "got data: {data}"
    );
}

#[test]
fn renaming_a_tag_with_special_characters_keeps_the_file_valid_gherkin() {
    // @step Given a project root tempdir with a feature file tagged @bug-#123
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        &tagged("@bug-#123", "A"),
    );

    // @step When I dispatch retag with from='@bug-#123' and to='@issue-123'
    // @step Then the dispatcher returns success=true
    retag_ok(tmp.path(), "@bug-#123", "@issue-123");

    // @step And all @bug-#123 tags should change to @issue-123
    let after = read(tmp.path(), "spec/features/a.feature");
    assert_eq!(after, tagged("@issue-123", "A"));

    // @step And the file should remain valid Gherkin
    assert_valid_gherkin(tmp.path(), "spec/features/a.feature");
}

#[test]
fn renaming_one_tag_keeps_the_other_tags_on_the_line_in_order() {
    // @step Given a project root tempdir with a feature tagged @release-one @critical @api
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        &tagged("@release-one @critical @api", "A"),
    );

    // @step When I dispatch retag with from='@release-one' and to='@v1'
    // @step Then the dispatcher returns success=true
    retag_ok(tmp.path(), "@release-one", "@v1");

    // @step And the feature should have tags @v1 @critical @api
    let after = read(tmp.path(), "spec/features/a.feature");
    let tag_line = after.lines().next().unwrap_or_default();
    assert_eq!(tag_line, "@v1 @critical @api", "got: {after}");

    // @step And tag order should be preserved
    // @step And other tags should remain unchanged
    assert_eq!(after, tagged("@v1 @critical @api", "A"));
}

#[test]
fn renaming_across_twenty_files_validates_every_modified_file() {
    // @step Given a project root tempdir with 20 feature files using @legacy-api
    let tmp = TempDir::new().expect("tempdir");
    let rels: Vec<String> = (0..20)
        .map(|i| format!("spec/features/f{i:02}.feature"))
        .collect();
    for (i, rel) in rels.iter().enumerate() {
        write_feature(tmp.path(), rel, &tagged("@legacy-api", &format!("F{i}")));
    }

    // @step When I dispatch retag with from='@legacy-api' and to='@legacy'
    // @step Then the dispatcher returns success=true
    let data = retag_ok(tmp.path(), "@legacy-api", "@legacy");
    assert_eq!(data["fileCount"].as_u64(), Some(20), "got data: {data}");

    // @step And all 20 files should have valid Gherkin syntax
    for rel in &rels {
        assert!(
            read(tmp.path(), rel).starts_with("@legacy\n"),
            "{rel} not renamed"
        );
        assert_valid_gherkin(tmp.path(), rel);
    }

    // @step And the result message contains "All modified files validated successfully"
    let message = data["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("All modified files validated successfully"),
        "got: {message}"
    );
}

#[test]
fn renaming_four_occurrences_in_one_file_reports_four_occurrences() {
    // @step Given a project root tempdir with a feature file tagged @temp at feature level
    // @step And 3 scenarios in that file tagged @temp at scenario level
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        "@temp\nFeature: A\n\n  @temp\n  Scenario: One\n    Given x\n\n  @temp\n  Scenario: Two\n    Given y\n\n  @temp\n  Scenario: Three\n    Given z\n",
    );

    // @step When I dispatch retag with from='@temp' and to='@temporary'
    // @step Then the dispatcher returns success=true
    let data = retag_ok(tmp.path(), "@temp", "@temporary");

    // @step And all 4 occurrences should be renamed
    let after = read(tmp.path(), "spec/features/a.feature");
    assert_eq!(after.matches("@temporary").count(), 4, "got: {after}");
    assert!(!after.contains("@temp\n"), "got: {after}");
    assert_eq!(
        data["occurrenceCount"].as_u64(),
        Some(4),
        "got data: {data}"
    );

    // @step And the result message contains "4 occurrence(s)"
    let message = data["message"].as_str().unwrap_or_default();
    assert!(message.contains("4 occurrence(s)"), "got: {message}");
}
