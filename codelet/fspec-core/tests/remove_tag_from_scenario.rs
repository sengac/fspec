#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/remove-tag-from-scenario-rust-port.feature (RPC-282)
//!
//! Validates the dispatcher-level contract for the Rust port of
//! `remove-tag-from-scenario`. Each `#[test]` maps 1:1 to a Gherkin
//! scenario; @step comments mirror the feature steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "remove-tag-from-scenario".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_feature(project_root: &Path, rel: &str, body: &str) {
    let p = project_root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).expect("mkdir parents");
    fs::write(&p, body).expect("write feature");
}

fn read_feature(project_root: &Path, rel: &str) -> String {
    fs::read_to_string(project_root.join(rel)).expect("read feature")
}

fn data_value(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data).expect("parse dispatcher data json")
}

fn feature_scenario_tagged(tags: &[&str]) -> String {
    let mut s = String::from("Feature: Login\n\n");
    for t in tags {
        s.push_str(&format!("  {t}\n"));
    }
    s.push_str(
        "  Scenario: Login\n\
         \x20\x20\x20\x20Given a user\n\
         \x20\x20\x20\x20When the user logs in\n\
         \x20\x20\x20\x20Then the dashboard appears\n",
    );
    s
}

fn feature_only_scenario_no_tags() -> String {
    String::from(
        "Feature: Login\n\
         \n\
         \x20\x20Scenario: Login\n\
         \x20\x20\x20\x20Given a user\n\
         \x20\x20\x20\x20When the user logs in\n\
         \x20\x20\x20\x20Then the dashboard appears\n",
    )
}

fn feature_two_scenarios_with_feature_tag() -> String {
    String::from(
        "@authentication\n\
         Feature: Auth\n\
         \n\
         \x20\x20@smoke\n\
         \x20\x20@critical\n\
         \x20\x20Scenario: Login\n\
         \x20\x20\x20\x20Given a user\n\
         \x20\x20\x20\x20When the user logs in\n\
         \x20\x20\x20\x20Then the dashboard appears\n",
    )
}

fn feature_two_scenarios_shared_smoke() -> String {
    String::from(
        "Feature: Auth\n\
         \n\
         \x20\x20@smoke\n\
         \x20\x20Scenario: Login\n\
         \x20\x20\x20\x20Given a user\n\
         \x20\x20\x20\x20When the user logs in\n\
         \x20\x20\x20\x20Then the dashboard appears\n\
         \n\
         \x20\x20@smoke\n\
         \x20\x20@regression\n\
         \x20\x20Scenario: Logout\n\
         \x20\x20\x20\x20Given a user\n\
         \x20\x20\x20\x20When the user logs out\n\
         \x20\x20\x20\x20Then the login appears\n",
    )
}

// ---------- scenarios ----------

#[test]
fn remove_single_tag_from_multi_tag_scenario() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke @critical @regression
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/login.feature";
    write_feature(
        tmp.path(),
        path,
        &feature_scenario_tagged(&["@smoke", "@critical", "@regression"]),
    );

    // @step When I dispatch remove-tag-from-scenario with file='spec/features/login.feature' scenario='Login' tags=['@critical']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Login", "tags": ["@critical"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");

    // @step And the returned data contains valid=true
    let data = data_value(&result);
    assert_eq!(data["valid"].as_bool(), Some(true));

    // @step And the returned data contains message="Removed @critical from scenario 'Login'"
    assert_eq!(
        data["message"].as_str(),
        Some("Removed @critical from scenario 'Login'")
    );

    // @step And the file on disk shows the Login scenario tagged @smoke @regression
    let body = read_feature(tmp.path(), path);
    assert!(
        body.contains("\n  @smoke\n  @regression\n  Scenario: Login\n"),
        "expected @smoke @regression; got:\n{body}"
    );
    assert!(!body.contains("@critical"), "@critical must be gone");

    // @step And the file on disk still parses as valid Gherkin
    let _ = gherkin::Feature::parse(&body, gherkin::GherkinEnv::default())
        .expect("file should still parse as Gherkin");
}

#[test]
fn remove_multiple_tags_in_one_call() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke @critical @regression @wip
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/login.feature";
    write_feature(
        tmp.path(),
        path,
        &feature_scenario_tagged(&["@smoke", "@critical", "@regression", "@wip"]),
    );

    // @step When I dispatch remove-tag-from-scenario with file='spec/features/login.feature' scenario='Login' tags=['@critical','@wip']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Login", "tags": ["@critical", "@wip"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");

    // @step And the file on disk shows the Login scenario tagged @smoke @regression
    let body = read_feature(tmp.path(), path);
    assert!(
        body.contains("\n  @smoke\n  @regression\n  Scenario: Login\n"),
        "expected @smoke @regression; got:\n{body}"
    );
    assert!(!body.contains("@critical"));
    assert!(!body.contains("@wip"));

    // @step And the returned data contains message="Removed @critical, @wip from scenario 'Login'"
    let data = data_value(&result);
    assert_eq!(
        data["message"].as_str(),
        Some("Removed @critical, @wip from scenario 'Login'")
    );
}

#[test]
fn requested_tag_absent_from_scenario_is_idempotent_success() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/login.feature";
    write_feature(tmp.path(), path, &feature_scenario_tagged(&["@smoke"]));
    let pre = fs::read(tmp.path().join(path)).unwrap();

    // @step When I dispatch remove-tag-from-scenario with file='spec/features/login.feature' scenario='Login' tags=['@critical']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Login", "tags": ["@critical"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected idempotent success; got {result:?}"
    );

    // @step And the returned data contains valid=true
    let data = data_value(&result);
    assert_eq!(data["valid"].as_bool(), Some(true));

    // @step And the returned data contains message="No changes made - none of the specified tags found on scenario 'Login'"
    assert_eq!(
        data["message"].as_str(),
        Some("No changes made - none of the specified tags found on scenario 'Login'")
    );

    // @step And the file on disk is byte-equal to its pre-call contents
    let post = fs::read(tmp.path().join(path)).unwrap();
    assert_eq!(pre, post);
}

#[test]
fn remove_all_tags_from_a_scenario() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke @critical
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/login.feature";
    write_feature(
        tmp.path(),
        path,
        &feature_scenario_tagged(&["@smoke", "@critical"]),
    );

    // @step When I dispatch remove-tag-from-scenario with file='spec/features/login.feature' scenario='Login' tags=['@smoke','@critical']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Login", "tags": ["@smoke", "@critical"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");

    // @step And the file on disk shows the Login scenario with no tag lines immediately above it
    let body = read_feature(tmp.path(), path);
    assert!(
        !body.contains("@smoke"),
        "@smoke must be gone; got:\n{body}"
    );
    assert!(
        !body.contains("@critical"),
        "@critical must be gone; got:\n{body}"
    );
    // No tag line directly above the Scenario:
    assert!(
        body.contains("\n  Scenario: Login\n"),
        "expected Scenario line preserved; got:\n{body}"
    );
}

#[test]
fn missing_scenario_is_idempotent_success() {
    // @step Given a project root tempdir with spec/features/login.feature containing only Scenario 'Login'
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/login.feature";
    write_feature(tmp.path(), path, &feature_only_scenario_no_tags());
    let pre = fs::read(tmp.path().join(path)).unwrap();

    // @step When I dispatch remove-tag-from-scenario with file='spec/features/login.feature' scenario='Nonexistent' tags=['@smoke']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Nonexistent", "tags": ["@smoke"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected idempotent success; got {result:?}"
    );

    // @step And the returned data contains valid=true
    let data = data_value(&result);
    assert_eq!(data["valid"].as_bool(), Some(true));

    // @step And the returned data contains message="Scenario 'Nonexistent' not found in spec/features/login.feature - no changes made"
    assert_eq!(
        data["message"].as_str(),
        Some("Scenario 'Nonexistent' not found in spec/features/login.feature - no changes made")
    );

    // @step And the file on disk is byte-equal to its pre-call contents
    let post = fs::read(tmp.path().join(path)).unwrap();
    assert_eq!(pre, post);
}

#[test]
fn missing_feature_file_surfaces_canonical_error() {
    // @step Given an empty project root directory with no spec/features/missing.feature
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features/missing.feature").exists());

    // @step When I dispatch remove-tag-from-scenario with file='spec/features/missing.feature' scenario='Login' tags=['@smoke']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/missing.feature", "scenario": "Login", "tags": ["@smoke"]}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring 'File not found: spec/features/missing.feature'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("File not found: spec/features/missing.feature"),
        "expected file-not-found message; got: {err}"
    );
}

#[test]
fn feature_level_tags_survive_the_mutation() {
    // @step Given a project root tempdir with spec/features/auth.feature containing feature tag @authentication and Scenario 'Login' tagged @smoke @critical
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/auth.feature";
    write_feature(tmp.path(), path, &feature_two_scenarios_with_feature_tag());

    // @step When I dispatch remove-tag-from-scenario with file='spec/features/auth.feature' scenario='Login' tags=['@smoke']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Login", "tags": ["@smoke"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");

    // @step And the file on disk still contains the feature-level tag @authentication
    let body = read_feature(tmp.path(), path);
    assert!(
        body.starts_with("@authentication\n"),
        "feature tag must persist; got:\n{body}"
    );

    // @step And the file on disk shows the Login scenario tagged @critical
    assert!(
        body.contains("\n  @critical\n  Scenario: Login\n"),
        "Login must be tagged @critical only; got:\n{body}"
    );
    assert!(
        !body.contains("\n  @smoke\n  @critical"),
        "@smoke must be gone from scenario; got:\n{body}"
    );
}

#[test]
fn tags_on_sibling_scenario_are_untouched() {
    // @step Given a project root tempdir with spec/features/auth.feature containing Scenario 'Login' tagged @smoke and Scenario 'Logout' tagged @smoke @regression
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/auth.feature";
    write_feature(tmp.path(), path, &feature_two_scenarios_shared_smoke());

    // @step When I dispatch remove-tag-from-scenario with file='spec/features/auth.feature' scenario='Login' tags=['@smoke']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Login", "tags": ["@smoke"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");

    // @step And the file on disk shows the Login scenario with no tag lines immediately above it
    let body = read_feature(tmp.path(), path);
    assert!(
        body.contains("\n  Scenario: Login\n"),
        "Login line must remain; got:\n{body}"
    );
    // Verify no tag line immediately above Scenario: Login
    let idx = body.find("  Scenario: Login\n").expect("found Login");
    // The 3 chars before "  Scenario: Login\n" should be "\n\n" or similar (no @ tag)
    let prefix = &body[..idx];
    let last_line = prefix.rsplit('\n').nth(1).unwrap_or("");
    assert!(
        !last_line.trim_start().starts_with('@'),
        "Login should have no tag line immediately above; prev line='{last_line}'; full:\n{body}"
    );

    // @step And the file on disk shows the Logout scenario tagged @smoke @regression
    assert!(
        body.contains("\n  @smoke\n  @regression\n  Scenario: Logout\n"),
        "Logout @smoke @regression must persist; got:\n{body}"
    );
}
