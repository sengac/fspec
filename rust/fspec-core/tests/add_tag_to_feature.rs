#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-tag-to-feature-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-tag-to-feature` (RPC-193).
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
        command: "add-tag-to-feature".to_string(),
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

fn write_canonical_tags_json(project_root: &Path) {
    // 9-category default, all empty — sufficient for the "not registered" scenarios.
    let body = json!({
        "categories": [
            {"name": "Phase Tags", "description": "", "required": true, "tags": []},
            {"name": "Component Tags", "description": "", "required": true, "tags": []},
            {"name": "Feature Group Tags", "description": "", "required": true, "tags": []},
            {"name": "Technical Tags", "description": "", "required": false, "tags": []},
            {"name": "Platform Tags", "description": "", "required": false, "tags": []},
            {"name": "Priority Tags", "description": "", "required": false, "tags": []},
            {"name": "Status Tags", "description": "", "required": false, "tags": []},
            {"name": "Testing Tags", "description": "", "required": false, "tags": []},
            {"name": "Automation Tags", "description": "", "required": false, "tags": []}
        ]
    });
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("tags.json"),
        serde_json::to_string_pretty(&body).unwrap(),
    )
    .expect("write tags.json");
}

const FEATURE_LOGIN_PLAIN: &str = "Feature: Login\n  Scenario: A\n    Given x\n";

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_adds_single_tag_to_feature_with_no_existing_tags() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@critical']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@critical"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher message contains 'Added @critical to spec/features/login.feature'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("Added @critical to spec/features/login.feature"),
        "unexpected message: {msg}"
    );

    // @step And the file on disk starts with the line '@critical' followed by the 'Feature: Login' line
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    let mut lines = after.lines();
    assert_eq!(lines.next(), Some("@critical"));
    assert_eq!(lines.next(), Some("Feature: Login"));
}

#[test]
fn scenario_adds_multiple_tags_in_a_single_call_preserving_order() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@critical','@auth']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@critical", "@auth"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher message contains 'Added @critical, @auth to spec/features/login.feature'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("Added @critical, @auth to spec/features/login.feature"),
        "unexpected message: {msg}"
    );

    // @step And the file on disk contains the line '@critical' immediately followed by the line '@auth' above 'Feature: Login'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    let lines: Vec<&str> = after.lines().collect();
    let crit = lines
        .iter()
        .position(|l| *l == "@critical")
        .expect("@critical line present");
    assert_eq!(lines.get(crit + 1).copied(), Some("@auth"));
    let feat = lines
        .iter()
        .position(|l| *l == "Feature: Login")
        .expect("Feature line present");
    assert!(feat > crit + 1, "Feature line must come after both tags");
}

#[test]
fn scenario_missing_feature_file_surfaces_canonical_not_found_error() {
    // @step Given a project root tempdir with NO spec/features/missing.feature file
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features/missing.feature").exists());

    // @step When I dispatch add-tag-to-feature with file='spec/features/missing.feature' and tags=['@critical']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/missing.feature", "tags": ["@critical"]}),
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
fn scenario_rejects_input_tag_missing_at_sign() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );
    let pre_bytes = fs::read(tmp.path().join("spec/features/login.feature")).unwrap();

    // @step When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['critical']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["critical"]}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message equals 'Invalid tag format. Tags must start with @'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Invalid tag format. Tags must start with @"),
        "expected canonical invalid-format message; got: {err}"
    );

    // file must remain untouched
    let post = fs::read(tmp.path().join("spec/features/login.feature")).unwrap();
    assert_eq!(pre_bytes, post);
}

#[test]
fn scenario_rejects_mixed_case_tag_that_fails_both_regexes() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@MIXEDcase']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@MIXEDcase"]}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains 'Regular tags must use lowercase-with-hyphens, work unit tags must match @[A-Z]{2,6}-\d+'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Regular tags must use lowercase-with-hyphens")
            && err.contains("work unit tags must match @[A-Z]{2,6}-\\d+"),
        "expected canonical mixed-case message; got: {err}"
    );
}

#[test]
fn scenario_rejects_duplicate_tag_already_present_on_the_feature() {
    // @step Given a project root tempdir with spec/features/login.feature containing '@critical\nFeature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        "@critical\nFeature: Login\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@critical']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@critical"]}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message equals 'Tag @critical already exists on this feature'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Tag @critical already exists on this feature"),
        "expected duplicate message; got: {err}"
    );
}

#[test]
fn scenario_validate_registry_rejects_tag_not_in_tags_json() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step And spec/tags.json contains the canonical 9-category default with NO '@unregistered' tag
    write_canonical_tags_json(tmp.path());

    // @step When I dispatch add-tag-to-feature with file='spec/features/login.feature', tags=['@unregistered'], validateRegistry=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "file": "spec/features/login.feature",
            "tags": ["@unregistered"],
            "validateRegistry": true
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message equals 'Tag @unregistered is not registered in spec/tags.json'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Tag @unregistered is not registered in spec/tags.json"),
        "expected canonical registry-miss message; got: {err}"
    );
}

#[test]
fn scenario_appends_new_tag_after_existing_tag_block() {
    // @step Given a project root tempdir with spec/features/login.feature containing '@auth\n@security\nFeature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        "@auth\n@security\nFeature: Login\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@critical']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@critical"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk contains the lines '@critical', '@auth', '@security' in that order immediately above 'Feature: Login'
    //
    // Parity note: TS walks backwards from the Feature line; when it hits
    // index 0 while still on a tag line it clamps `insertIndex = 0` and
    // skips the "reposition after last tag" block, so the new tag lands
    // ABOVE the existing tag block (see src/commands/add-tag-to-feature.ts:174-177).
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    let lines: Vec<&str> = after.lines().collect();
    let auth = lines
        .iter()
        .position(|l| *l == "@auth")
        .expect("@auth line");
    let sec = lines
        .iter()
        .position(|l| *l == "@security")
        .expect("@security line");
    let crit = lines
        .iter()
        .position(|l| *l == "@critical")
        .expect("@critical line");
    let feat = lines
        .iter()
        .position(|l| *l == "Feature: Login")
        .expect("Feature line");
    assert!(crit < auth && auth < sec && sec < feat, "expected @critical < @auth < @security < Feature; got order indices {crit}/{auth}/{sec}/{feat}");
    assert_eq!(
        sec + 1,
        feat,
        "@security must remain immediately above Feature: Login"
    );
}

#[test]
fn scenario_inserts_new_tag_at_top_when_no_existing_tags() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@critical']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@critical"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the first line of the file on disk is '@critical'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    let mut lines = after.lines();
    assert_eq!(lines.next(), Some("@critical"));

    // @step And the second line of the file on disk is 'Feature: Login'
    assert_eq!(lines.next(), Some("Feature: Login"));
}

#[test]
fn scenario_without_registry_emits_unregistered_system_reminder() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step And spec/tags.json contains the canonical 9-category default with NO '@unknown' tag
    write_canonical_tags_json(tmp.path());

    // @step When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@unknown']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@unknown"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher response includes a systemReminder containing 'is not registered in spec/tags.json'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let reminder = data["systemReminder"].as_str().unwrap_or("");
    assert!(
        reminder.contains("is not registered in spec/tags.json"),
        "expected unregistered-tag reminder; got data.systemReminder: {reminder}"
    );
}

#[test]
fn scenario_work_unit_tag_without_registry_emits_no_unregistered_reminder() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step And spec/tags.json contains the canonical 9-category default with NO '@AUTH-001' tag
    write_canonical_tags_json(tmp.path());

    // @step When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@AUTH-001']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@AUTH-001"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher response does NOT include a systemReminder containing '@AUTH-001 is not registered'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let reminder = data["systemReminder"].as_str().unwrap_or("");
    assert!(
        !reminder.contains("@AUTH-001 is not registered"),
        "work-unit tags must NOT emit unregistered reminders; got: {reminder}"
    );
}

#[test]
fn scenario_emits_missing_required_tags_reminder() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@critical']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@critical"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher response includes a systemReminder mentioning 'component' and 'feature-group'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let reminder = data["systemReminder"].as_str().unwrap_or("");
    assert!(
        reminder.contains("component") && reminder.contains("feature-group"),
        "expected missing-required-tags reminder mentioning component and feature-group; got: {reminder}"
    );
}

#[test]
fn scenario_no_missing_required_tags_reminder_when_satisfied() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-tag-to-feature with file='spec/features/login.feature' and tags=['@cli','@feature-management']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "tags": ["@cli", "@feature-management"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher response does NOT include a systemReminder mentioning missing required tags
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let reminder = data["systemReminder"].as_str().unwrap_or("");
    assert!(
        !reminder.contains("missing required tags"),
        "must not emit missing-required-tags reminder when both component and feature-group are present; got: {reminder}"
    );
}
