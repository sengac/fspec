#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-background-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-background` (RPC-171).
// Each scenario maps to one #[test] fn with @step comments mirroring the
// Gherkin steps verbatim. At PHASE B (red) the command is still a stub, so the
// dispatcher returns the NotYetPorted envelope (success=false, error contains
// "not yet ported"); the green-phase assertions are written as the real
// contract and will pass once the impl lands in PHASE C.

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
        command: "add-background".to_string(),
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

const FEATURE_LOGIN_PLAIN: &str = "Feature: Login\n  Scenario: A\n    Given x\n";

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Adds a Background section to a feature with no existing Background
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_adds_background_to_feature_with_no_existing_background() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/login.feature", FEATURE_LOGIN_PLAIN);

    // @step When I dispatch add-background with feature='spec/features/login.feature' and text='As a user\nI want to log in\nSo that I access my account'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "feature": "spec/features/login.feature",
            "text": "As a user\nI want to log in\nSo that I access my account"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher message contains 'Added background to spec/features/login.feature'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("Added background to spec/features/login.feature"),
        "unexpected message: {msg}"
    );

    // @step And the file on disk contains the line '  Background: User Story'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    let lines: Vec<&str> = after.lines().collect();
    assert!(
        lines.contains(&"  Background: User Story"),
        "missing Background line; got:\n{after}"
    );

    // @step And the file on disk contains the line '    As a user'
    assert!(
        lines.contains(&"    As a user"),
        "missing indented user story line; got:\n{after}"
    );

    // @step And the Background block appears after the 'Feature: Login' line and before the 'Scenario: A' line
    let feat = lines.iter().position(|l| *l == "Feature: Login").expect("Feature line");
    let bg = lines.iter().position(|l| *l == "  Background: User Story").expect("Background line");
    let scen = lines.iter().position(|l| l.trim() == "Scenario: A").expect("Scenario line");
    assert!(feat < bg && bg < scen, "expected Feature < Background < Scenario; got {feat}/{bg}/{scen}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Empty text is rejected and the file is left untouched
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_empty_text_is_rejected_and_file_untouched() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/login.feature", FEATURE_LOGIN_PLAIN);
    let pre_bytes = fs::read(tmp.path().join("spec/features/login.feature")).unwrap();

    // @step When I dispatch add-background with feature='spec/features/login.feature' and text=''
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "text": ""}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message equals 'Background text cannot be empty'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Background text cannot be empty"),
        "expected canonical empty-text message; got: {err}"
    );

    // @step And the file on disk is byte-for-byte unchanged
    let post = fs::read(tmp.path().join("spec/features/login.feature")).unwrap();
    assert_eq!(pre_bytes, post, "file must remain untouched");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Whitespace-only text is rejected
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_whitespace_only_text_is_rejected() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/login.feature", FEATURE_LOGIN_PLAIN);

    // @step When I dispatch add-background with feature='spec/features/login.feature' and text='   '
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "text": "   "}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message equals 'Background text cannot be empty'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Background text cannot be empty"),
        "expected canonical empty-text message; got: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Missing feature file surfaces the canonical not-found error
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_missing_feature_file_surfaces_not_found() {
    // @step Given a project root tempdir with NO spec/features/missing.feature file
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features/missing.feature").exists());

    // @step When I dispatch add-background with feature='spec/features/missing.feature' and text='As a user'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/missing.feature", "text": "As a user"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message equals 'Feature file not found: spec/features/missing.feature'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Feature file not found: spec/features/missing.feature"),
        "expected canonical not-found message; got: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Bare feature name resolves by basename glob over spec/features
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_bare_feature_name_resolves_by_basename() {
    // @step Given a project root tempdir with spec/features/dashboard.feature containing only 'Feature: Dashboard\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/dashboard.feature",
        "Feature: Dashboard\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch add-background with feature='dashboard' and text='As a user\nI want a dashboard\nSo that I see an overview'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "feature": "dashboard",
            "text": "As a user\nI want a dashboard\nSo that I see an overview"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher message contains 'Added background to dashboard'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let msg = data["message"].as_str().unwrap_or("");
    assert!(msg.contains("Added background to dashboard"), "unexpected message: {msg}");

    // @step And the file spec/features/dashboard.feature on disk contains the line '  Background: User Story'
    let after = read_feature(tmp.path(), "spec/features/dashboard.feature");
    assert!(
        after.lines().any(|l| l == "  Background: User Story"),
        "missing Background line; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Replaces an existing Background section in place
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_replaces_existing_background_in_place() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n\n  Background: User Story\n    As an old user\n\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        "Feature: Login\n\n  Background: User Story\n    As an old user\n\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch add-background with feature='spec/features/login.feature' and text='As a new user\nI want X\nSo that Y'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "feature": "spec/features/login.feature",
            "text": "As a new user\nI want X\nSo that Y"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk contains the line '    As a new user'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        after.lines().any(|l| l == "    As a new user"),
        "missing new user-story line; got:\n{after}"
    );

    // @step And the file on disk does NOT contain the line '    As an old user'
    assert!(
        !after.lines().any(|l| l == "    As an old user"),
        "old user-story line must be removed; got:\n{after}"
    );

    // @step And the file on disk contains exactly one 'Background: User Story' line
    let count = after.lines().filter(|l| l.trim() == "Background: User Story").count();
    assert_eq!(count, 1, "expected exactly one Background line; got {count}:\n{after}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Inserts the Background after a Feature-line doc string
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_inserts_background_after_feature_doc_string() {
    // @step Given a project root tempdir with spec/features/api.feature containing 'Feature: API\n  """\n  Architecture notes\n  """\n\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/api.feature",
        "Feature: API\n  \"\"\"\n  Architecture notes\n  \"\"\"\n\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch add-background with feature='spec/features/api.feature' and text='As a developer\nI want the API\nSo that I integrate'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "feature": "spec/features/api.feature",
            "text": "As a developer\nI want the API\nSo that I integrate"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the Background block appears after the closing doc-string fence and before the 'Scenario: A' line
    let after = read_feature(tmp.path(), "spec/features/api.feature");
    let lines: Vec<&str> = after.lines().collect();
    let last_fence = lines.iter().rposition(|l| l.trim() == "\"\"\"").expect("closing fence");
    let bg = lines.iter().position(|l| l.trim() == "Background: User Story").expect("Background line");
    let scen = lines.iter().position(|l| l.trim() == "Scenario: A").expect("Scenario line");
    assert!(last_fence < bg && bg < scen, "expected fence < Background < Scenario; got {last_fence}/{bg}/{scen}");

    // @step And the file on disk still contains the line '  Architecture notes'
    assert!(
        lines.contains(&"  Architecture notes"),
        "architecture doc-string must be preserved; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A file with no Feature line is rejected
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_no_feature_line_is_rejected() {
    // @step Given a project root tempdir with spec/features/bad.feature containing only '# just a comment\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/bad.feature", "# just a comment\n");

    // @step When I dispatch add-background with feature='spec/features/bad.feature' and text='As a user'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/bad.feature", "text": "As a user"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains 'No Feature line found in file'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("No Feature line found in file"),
        "expected canonical no-Feature-line message; got: {err}"
    );
}
