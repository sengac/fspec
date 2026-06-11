#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/add-tag-to-scenario-rust-port.feature (RPC-194)
//!
//! Validates the dispatcher-level contract for the Rust port of
//! `add-tag-to-scenario`. Each `#[test]` maps 1:1 to a Gherkin scenario;
//! @step comments mirror the feature steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-tag-to-scenario".to_string(),
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

fn feature_with_scenario_no_tags() -> String {
    String::from(
        "Feature: Login\n\
         \n\
         \x20\x20Scenario: Login with valid credentials\n\
         \x20\x20\x20\x20Given a user\n\
         \x20\x20\x20\x20When the user logs in\n\
         \x20\x20\x20\x20Then the dashboard appears\n",
    )
}

fn feature_with_scenario_tagged(tags: &[&str]) -> String {
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

fn feature_with_two_scenarios_feature_tag() -> String {
    String::from(
        "@authentication\n\
         Feature: Auth\n\
         \n\
         \x20\x20@smoke\n\
         \x20\x20Scenario: Login\n\
         \x20\x20\x20\x20Given a user\n\
         \x20\x20\x20\x20When the user logs in\n\
         \x20\x20\x20\x20Then the dashboard appears\n\
         \n\
         \x20\x20@regression\n\
         \x20\x20Scenario: Logout\n\
         \x20\x20\x20\x20Given a user\n\
         \x20\x20\x20\x20When the user logs out\n\
         \x20\x20\x20\x20Then the login appears\n",
    )
}

fn write_tags_json(project_root: &Path, body: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("tags.json"), body).expect("write tags.json");
}

// ---------- scenarios ----------

#[test]
fn add_single_tag_to_scenario_with_no_tags() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login with valid credentials' with no tags
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/login.feature", &feature_with_scenario_no_tags());

    // @step When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login with valid credentials' tags=['@smoke']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "scenario": "Login with valid credentials", "tags": ["@smoke"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains valid=true
    let data = data_value(&result);
    assert_eq!(data["valid"].as_bool(), Some(true));

    // @step And the returned data contains message="Added @smoke to scenario 'Login with valid credentials'"
    assert_eq!(
        data["message"].as_str(),
        Some("Added @smoke to scenario 'Login with valid credentials'")
    );

    // @step And the file on disk shows a single '  @smoke' line immediately above the Scenario line
    let body = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        body.contains("\n  @smoke\n  Scenario: Login with valid credentials\n"),
        "expected '@smoke' immediately above Scenario; got:\n{body}"
    );

    // @step And the file on disk still parses as valid Gherkin
    let _ = gherkin::Feature::parse(&body, gherkin::GherkinEnv::default())
        .expect("file should still parse as Gherkin");
}

#[test]
fn append_new_tag_after_existing_tag_block() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        &feature_with_scenario_tagged(&["@smoke"]),
    );

    // @step When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['@critical']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "scenario": "Login", "tags": ["@critical"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");

    // @step And the file on disk shows '  @smoke' followed by '  @critical' immediately above the Scenario line
    let body = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        body.contains("\n  @smoke\n  @critical\n  Scenario: Login\n"),
        "expected '@smoke' then '@critical' above Scenario; got:\n{body}"
    );

    // @step And no other lines in the file are mutated
    assert!(body.starts_with("Feature: Login\n"));
    assert!(body.contains("    Given a user\n"));
    assert!(body.contains("    When the user logs in\n"));
    assert!(body.contains("    Then the dashboard appears\n"));
}

#[test]
fn multiple_tags_inserted_in_argument_order() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    let tmp = TempDir::new().expect("tempdir");
    let no_tags = feature_with_scenario_no_tags().replace("Login with valid credentials", "Login");
    write_feature(tmp.path(), "spec/features/login.feature", &no_tags);

    // @step When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['@critical','@regression']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": "spec/features/login.feature", "scenario": "Login", "tags": ["@critical", "@regression"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success);

    // @step And the file on disk shows '  @critical' followed by '  @regression' above the Scenario line
    let body = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        body.contains("\n  @critical\n  @regression\n  Scenario: Login\n"),
        "expected critical then regression; got:\n{body}"
    );

    // @step And the returned data contains message="Added @critical, @regression to scenario 'Login'"
    let data = data_value(&result);
    assert_eq!(
        data["message"].as_str(),
        Some("Added @critical, @regression to scenario 'Login'")
    );
}

#[test]
fn duplicate_tag_is_rejected() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/login.feature";
    write_feature(tmp.path(), path, &feature_with_scenario_tagged(&["@smoke"]));
    let pre = fs::read(tmp.path().join(path)).unwrap();

    // @step When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['@smoke']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Login", "tags": ["@smoke"]}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring 'Tag @smoke already exists on this scenario'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Tag @smoke already exists on this scenario"),
        "expected duplicate-tag message; got: {err}"
    );

    // @step And the file on disk is byte-equal to its pre-call contents
    let post = fs::read(tmp.path().join(path)).unwrap();
    assert_eq!(pre, post);
}

#[test]
fn tag_without_leading_at_is_rejected() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/login.feature";
    let body = feature_with_scenario_no_tags().replace("Login with valid credentials", "Login");
    write_feature(tmp.path(), path, &body);
    let pre = fs::read(tmp.path().join(path)).unwrap();

    // @step When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['InvalidTag']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Login", "tags": ["InvalidTag"]}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring 'Invalid tag format. Tags must start with @'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Invalid tag format. Tags must start with @"),
        "expected @-prefix error; got: {err}"
    );

    // @step And the file on disk is byte-equal to its pre-call contents
    let post = fs::read(tmp.path().join(path)).unwrap();
    assert_eq!(pre, post);
}

#[test]
fn mixed_case_regular_tag_is_rejected() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/login.feature";
    let body = feature_with_scenario_no_tags().replace("Login with valid credentials", "Login");
    write_feature(tmp.path(), path, &body);
    let pre = fs::read(tmp.path().join(path)).unwrap();

    // @step When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['@CamelCase']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Login", "tags": ["@CamelCase"]}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring 'Regular tags must use lowercase-with-hyphens'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Regular tags must use lowercase-with-hyphens"),
        "expected lowercase guidance; got: {err}"
    );

    // @step And the file on disk is byte-equal to its pre-call contents
    let post = fs::read(tmp.path().join(path)).unwrap();
    assert_eq!(pre, post);
}

#[test]
fn work_unit_tag_with_uppercase_is_accepted() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/login.feature";
    let body = feature_with_scenario_no_tags().replace("Login with valid credentials", "Login");
    write_feature(tmp.path(), path, &body);

    // @step When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['@AUTH-001']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Login", "tags": ["@AUTH-001"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");

    // @step And the file on disk shows '  @AUTH-001' immediately above the Scenario line
    let body = read_feature(tmp.path(), path);
    assert!(
        body.contains("\n  @AUTH-001\n  Scenario: Login\n"),
        "expected @AUTH-001 above Scenario; got:\n{body}"
    );
}

#[test]
fn registry_validation_accepts_registered_tag() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/login.feature";
    let body = feature_with_scenario_no_tags().replace("Login with valid credentials", "Login");
    write_feature(tmp.path(), path, &body);

    // @step And spec/tags.json registers @custom-tag under category 'Test Tags'
    write_tags_json(
        tmp.path(),
        r#"{
          "categories": [
            { "name": "Test Tags", "tags": [{ "name": "@custom-tag", "description": "x" }] }
          ]
        }"#,
    );

    // @step When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['@custom-tag'] validateRegistry=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Login", "tags": ["@custom-tag"], "validateRegistry": true}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");

    // @step And the file on disk shows '  @custom-tag' immediately above the Scenario line
    let body = read_feature(tmp.path(), path);
    assert!(
        body.contains("\n  @custom-tag\n  Scenario: Login\n"),
        "expected @custom-tag above Scenario; got:\n{body}"
    );
}

#[test]
fn registry_validation_rejects_unregistered_tag() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/login.feature";
    let body = feature_with_scenario_no_tags().replace("Login with valid credentials", "Login");
    write_feature(tmp.path(), path, &body);
    let pre = fs::read(tmp.path().join(path)).unwrap();

    // @step And spec/tags.json does NOT register @unregistered
    write_tags_json(
        tmp.path(),
        r#"{
          "categories": [
            { "name": "Test Tags", "tags": [{ "name": "@something-else", "description": "x" }] }
          ]
        }"#,
    );

    // @step When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Login' tags=['@unregistered'] validateRegistry=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Login", "tags": ["@unregistered"], "validateRegistry": true}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring '@unregistered is not registered in spec/tags.json'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("@unregistered is not registered in spec/tags.json"),
        "expected registry error; got: {err}"
    );

    // @step And the file on disk is byte-equal to its pre-call contents
    let post = fs::read(tmp.path().join(path)).unwrap();
    assert_eq!(pre, post);
}

#[test]
fn missing_scenario_name_surfaces_canonical_error() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/login.feature";
    let body = feature_with_scenario_no_tags().replace("Login with valid credentials", "Login");
    write_feature(tmp.path(), path, &body);
    let pre = fs::read(tmp.path().join(path)).unwrap();

    // @step When I dispatch add-tag-to-scenario with file='spec/features/login.feature' scenario='Nonexistent' tags=['@smoke']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Nonexistent", "tags": ["@smoke"]}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring "Scenario 'Nonexistent' not found in spec/features/login.feature"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Scenario 'Nonexistent' not found in spec/features/login.feature"),
        "expected missing-scenario message; got: {err}"
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

    // @step When I dispatch add-tag-to-scenario with file='spec/features/missing.feature' scenario='Login' tags=['@smoke']
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
fn feature_level_tag_and_other_scenarios_are_preserved() {
    // @step Given a project root tempdir with spec/features/auth.feature containing feature tag @authentication and two scenarios 'Login' (tagged @smoke) and 'Logout' (tagged @regression)
    let tmp = TempDir::new().expect("tempdir");
    let path = "spec/features/auth.feature";
    write_feature(tmp.path(), path, &feature_with_two_scenarios_feature_tag());

    // @step When I dispatch add-tag-to-scenario with file='spec/features/auth.feature' scenario='Login' tags=['@critical']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"file": path, "scenario": "Login", "tags": ["@critical"]}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");

    // @step And the file on disk still contains the feature-level tag @authentication
    let body = read_feature(tmp.path(), path);
    assert!(
        body.starts_with("@authentication\n"),
        "feature tag must persist; got:\n{body}"
    );

    // @step And the file on disk shows the Login scenario tags as '@smoke' then '@critical'
    assert!(
        body.contains("\n  @smoke\n  @critical\n  Scenario: Login\n"),
        "Login tags must be @smoke then @critical; got:\n{body}"
    );

    // @step And the file on disk shows the Logout scenario tags as '@regression' unchanged
    assert!(
        body.contains("\n  @regression\n  Scenario: Logout\n"),
        "Logout @regression must persist unchanged; got:\n{body}"
    );
    assert!(
        !body.contains("@regression\n  @critical"),
        "Logout must NOT have been touched"
    );
}
