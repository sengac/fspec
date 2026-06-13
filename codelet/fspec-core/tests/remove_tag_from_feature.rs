#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/remove-tag-from-feature-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `remove-tag-from-feature` (RPC-281).
// Each scenario maps to one #[test] fn with @step comments mirroring the
// Gherkin steps verbatim.

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
        command: "remove-tag-from-feature".to_string(),
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
    fs::read_to_string(project_root.join(rel)).expect("read feature")
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_removes_single_tag_from_feature_file() {
    // @step Given a project root tempdir with spec/features/login.feature containing '@wip\nFeature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        "@wip\nFeature: Login\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch remove-tag-from-feature with file='spec/features/login.feature' and tags=['@wip']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@wip"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher message contains 'Removed @wip from spec/features/login.feature'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("Removed @wip from spec/features/login.feature"),
        "unexpected message: {msg}"
    );

    // @step And the file on disk does NOT contain a line whose trimmed value is '@wip'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        !after.lines().any(|l| l.trim() == "@wip"),
        "@wip line must be removed; got:\n{after}"
    );

    // @step And the file on disk still contains the line 'Feature: Login'
    assert!(
        after.lines().any(|l| l == "Feature: Login"),
        "Feature header must remain; got:\n{after}"
    );
}

#[test]
fn scenario_removes_multiple_tags_in_a_single_call() {
    // @step Given a project root tempdir with spec/features/login.feature containing '@wip\n@draft\nFeature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        "@wip\n@draft\nFeature: Login\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch remove-tag-from-feature with file='spec/features/login.feature' and tags=['@wip','@draft']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@wip", "@draft"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher message contains 'Removed @wip, @draft from spec/features/login.feature'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("Removed @wip, @draft from spec/features/login.feature"),
        "unexpected message: {msg}"
    );

    // @step And the file on disk does NOT contain a line whose trimmed value is '@wip'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(!after.lines().any(|l| l.trim() == "@wip"));

    // @step And the file on disk does NOT contain a line whose trimmed value is '@draft'
    assert!(!after.lines().any(|l| l.trim() == "@draft"));
}

#[test]
fn scenario_missing_target_file_surfaces_canonical_not_found_error() {
    // @step Given a project root tempdir with NO spec/features/missing.feature file
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features/missing.feature").exists());

    // @step When I dispatch remove-tag-from-feature with file='spec/features/missing.feature' and tags=['@wip']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/missing.feature", "tags": ["@wip"]}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message equals 'File not found: spec/features/missing.feature'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("File not found: spec/features/missing.feature"),
        "expected canonical not-found message; got: {err}"
    );
}

#[test]
fn scenario_removing_absent_tag_leaves_file_untouched() {
    // @step Given a project root tempdir with spec/features/login.feature containing '@critical\nFeature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        "@critical\nFeature: Login\n  Scenario: A\n    Given x\n",
    );
    let pre_bytes = fs::read(tmp.path().join("spec/features/login.feature")).unwrap();

    // @step When I dispatch remove-tag-from-feature with file='spec/features/login.feature' and tags=['@notthere']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@notthere"]}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message equals 'Tag @notthere not found on this feature'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Tag @notthere not found on this feature"),
        "expected canonical absent-tag message; got: {err}"
    );

    // @step And the file on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/features/login.feature")).unwrap();
    assert_eq!(
        pre_bytes, post_bytes,
        "file must not be modified on failure"
    );
}

#[test]
fn scenario_source_without_feature_header_is_rejected() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'just some text\n# no feature header here\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        "just some text\n# no feature header here\n",
    );

    // @step When I dispatch remove-tag-from-feature with file='spec/features/login.feature' and tags=['@wip']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@wip"]}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains 'File does not contain a valid Feature'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("File does not contain a valid Feature"),
        "expected canonical no-feature message; got: {err}"
    );
}

#[test]
fn scenario_removed_tag_leaves_others_untouched_in_original_order() {
    // @step Given a project root tempdir with spec/features/login.feature containing '@critical\n@wip\n@auth\nFeature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        "@critical\n@wip\n@auth\nFeature: Login\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch remove-tag-from-feature with file='spec/features/login.feature' and tags=['@wip']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@wip"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk contains the line '@critical' immediately followed by the line '@auth' above 'Feature: Login'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    let lines: Vec<&str> = after.lines().collect();
    let crit = lines
        .iter()
        .position(|l| *l == "@critical")
        .expect("@critical line");
    let auth = lines
        .iter()
        .position(|l| *l == "@auth")
        .expect("@auth line");
    let feat = lines
        .iter()
        .position(|l| *l == "Feature: Login")
        .expect("Feature line");
    assert_eq!(
        crit + 1,
        auth,
        "@critical must be immediately followed by @auth"
    );
    assert!(auth < feat, "tags must come above Feature line");
    assert!(!lines.contains(&"@wip"), "@wip line removed");
}

#[test]
fn scenario_multi_tag_on_one_line_is_preserved() {
    // @step Given a project root tempdir with spec/features/login.feature containing '@a @b\nFeature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        "@a @b\nFeature: Login\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch remove-tag-from-feature with file='spec/features/login.feature' and tags=['@a']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@a"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected success=true (existence check passes via gherkin parse); got {result:?}"
    );

    // @step And the file on disk still contains a line whose trimmed value is '@a @b' (documented TS divergence — whole-line equality removal does not split multi-tag lines)
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        after.lines().any(|l| l.trim() == "@a @b"),
        "multi-tag line '@a @b' must be preserved (whole-line filter); got:\n{after}"
    );
}
