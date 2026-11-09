#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-scenario-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-scenario` (RPC-190).
// Each scenario maps to one #[test] fn with @step comments mirroring the
// Gherkin steps verbatim. At PHASE B time the command is still a stub, so the
// dispatcher returns success=false (NotYetPorted) — these tests are expected to
// FAIL until PHASE C lands the implementation.

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
        command: "add-scenario".to_string(),
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
const FEATURE_WITH_OUTLINE: &str =
    "Feature: Login\n  Scenario: A\n    Given x\n\n  Scenario Outline: O\n    Given <v>\n";

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Appends a scenario with placeholder steps to a feature
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_appends_a_scenario_with_placeholder_steps_to_a_feature() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-scenario with feature='spec/features/login.feature' and scenario='Login with invalid password'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "Login with invalid password"}),
    ));

    // @step Then the dispatcher returns success=true and valid=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(true));
    assert_eq!(data["valid"].as_bool(), Some(true));

    // @step And the file on disk contains the line '  Scenario: Login with invalid password'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        after
            .lines()
            .any(|l| l == "  Scenario: Login with invalid password"),
        "missing scenario line:\n{after}"
    );

    // @step And the file on disk contains the placeholder steps '[precondition]', '[action]', and '[expected outcome]'
    assert!(
        after.contains("[precondition]"),
        "missing [precondition]:\n{after}"
    );
    assert!(after.contains("[action]"), "missing [action]:\n{after}");
    assert!(
        after.contains("[expected outcome]"),
        "missing [expected outcome]:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Resolves a bare identifier to spec/features/<id>.feature
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_resolves_a_bare_identifier_to_spec_features_id_feature() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-scenario with feature='login' and scenario='Another scenario'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "login", "scenario": "Another scenario"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step And the file spec/features/login.feature contains the line '  Scenario: Another scenario'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        after.lines().any(|l| l == "  Scenario: Another scenario"),
        "missing scenario line:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Inserts the new scenario before an existing Scenario Outline
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_inserts_the_new_scenario_before_an_existing_scenario_outline() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n\n  Scenario Outline: O\n    Given <v>\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_WITH_OUTLINE,
    );

    // @step When I dispatch add-scenario with feature='spec/features/login.feature' and scenario='Inserted'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "Inserted"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And in the file on disk the line '  Scenario: Inserted' appears before the line '  Scenario Outline: O'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    let lines: Vec<&str> = after.lines().collect();
    let inserted = lines
        .iter()
        .position(|l| l.trim() == "Scenario: Inserted")
        .expect("inserted line");
    let outline = lines
        .iter()
        .position(|l| l.trim() == "Scenario Outline: O")
        .expect("outline line");
    assert!(
        inserted < outline,
        "Scenario: Inserted must appear before the outline"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Duplicate scenario name succeeds with a warning
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_duplicate_scenario_name_succeeds_with_a_warning() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-scenario with feature='spec/features/login.feature' and scenario='A'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "A"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step And the dispatcher warning equals 'A scenario named "A" already exists in this feature'
    assert_eq!(
        data["warning"].as_str(),
        Some("A scenario named \"A\" already exists in this feature")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Missing feature file surfaces the not-found error
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_missing_feature_file_surfaces_the_not_found_error() {
    // @step Given a project root tempdir with NO spec/features/missing.feature file
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("spec/features")).expect("mkdir");

    // @step When I dispatch add-scenario with feature='spec/features/missing.feature' and scenario='X'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/missing.feature", "scenario": "X"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        result.success,
        "dispatcher envelope itself succeeds; inner success=false"
    );
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(false));

    // @step And the error contains 'Feature file not found: '
    let err = data["error"].as_str().unwrap_or("");
    assert!(
        err.contains("Feature file not found: "),
        "unexpected error: {err}"
    );

    // @step And the suggestion equals "Use 'fspec create-feature' to create a new feature file"
    assert_eq!(
        data["suggestion"].as_str(),
        Some("Use 'fspec create-feature' to create a new feature file")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dry run does not write the file
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dry_run_does_not_write_the_file() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-scenario with feature='spec/features/login.feature', scenario='Ghost' and dryRun=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "Ghost", "dryRun": true}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step And the file spec/features/login.feature does NOT contain the line '  Scenario: Ghost'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        !after.lines().any(|l| l == "  Scenario: Ghost"),
        "dry run must not write the scenario:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Two added scenarios stay in the order they were added
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_two_added_scenarios_stay_in_the_order_they_were_added() {
    // @step Given a project root tempdir with spec/features/payment.feature containing 'Feature: Payment\n  Scenario: Existing\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/payment.feature",
        "Feature: Payment\n  Scenario: Existing\n    Given x\n",
    );

    // @step When I dispatch add-scenario with feature='payment' and scenario='Credit card payment'
    let first = dispatch_command(req(
        tmp.path(),
        json!({"feature": "payment", "scenario": "Credit card payment"}),
    ));
    assert!(first.success, "expected success=true, got {first:?}");

    // @step And I dispatch add-scenario with feature='payment' and scenario='PayPal payment'
    let second = dispatch_command(req(
        tmp.path(),
        json!({"feature": "payment", "scenario": "PayPal payment"}),
    ));
    assert!(second.success, "expected success=true, got {second:?}");

    // @step Then the file spec/features/payment.feature has the scenarios 'Existing', 'Credit card payment' and 'PayPal payment' in that order
    let after = read_feature(tmp.path(), "spec/features/payment.feature");
    let scenarios: Vec<&str> = after
        .lines()
        .filter_map(|l| l.trim().strip_prefix("Scenario: "))
        .collect();
    assert_eq!(
        scenarios,
        vec!["Existing", "Credit card payment", "PayPal payment"],
        "{after}"
    );

    // @step And the second dispatch returns valid=true
    let data: Value = serde_json::from_str(&second.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(true));
    assert_eq!(data["valid"].as_bool(), Some(true));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Adding a scenario keeps the tags, Background and existing scenarios
// ─────────────────────────────────────────────────────────────────────────

const FEATURE_WITH_TAGS_AND_BACKGROUND: &str = "@auth\n@critical\nFeature: My feature\n\n  Background: User Story\n    As a user\n    I want to log in\n    So that I can work\n\n  Scenario: First\n    Given one\n    Then two\n\n  Scenario: Second\n    Given three\n    Then four\n";

#[test]
fn scenario_adding_a_scenario_keeps_the_tags_background_and_existing_scenarios() {
    // @step Given a project root tempdir with spec/features/my-feature.feature that has the feature tags @auth and @critical, a Background and the scenarios 'First' and 'Second'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/my-feature.feature",
        FEATURE_WITH_TAGS_AND_BACKGROUND,
    );

    // @step When I dispatch add-scenario with feature='my-feature' and scenario='New scenario'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "my-feature", "scenario": "New scenario"}),
    ));
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step Then the file on disk starts with the original content unchanged
    let after = read_feature(tmp.path(), "spec/features/my-feature.feature");
    assert!(
        after.starts_with(FEATURE_WITH_TAGS_AND_BACKGROUND),
        "original content changed:\n{after}"
    );

    // @step And 'Scenario: New scenario' is the last scenario in the file
    let last = after
        .lines()
        .rfind(|l| l.trim_start().starts_with("Scenario"));
    assert_eq!(last, Some("  Scenario: New scenario"), "{after}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Invalid Gherkin fails with a validate suggestion and leaves the file unchanged
// ─────────────────────────────────────────────────────────────────────────

const BROKEN_FEATURE: &str = "Scenario: Some scenario\n  Given a\n";

#[test]
fn scenario_invalid_gherkin_fails_with_a_validate_suggestion_and_leaves_the_file_unchanged() {
    // @step Given a project root tempdir with spec/features/broken.feature that does not parse as Gherkin
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/broken.feature", BROKEN_FEATURE);

    // @step When I dispatch add-scenario with feature='broken' and scenario='New scenario'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "broken", "scenario": "New scenario"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        result.success,
        "dispatcher envelope itself succeeds; inner success=false"
    );
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(false));

    // @step And the error starts with 'Feature file has invalid Gherkin syntax: '
    let err = data["error"].as_str().unwrap_or("");
    assert!(
        err.starts_with("Feature file has invalid Gherkin syntax: "),
        "unexpected error: {err}"
    );

    // @step And the suggestion equals "Run 'fspec validate broken' to see syntax errors"
    assert_eq!(
        data["suggestion"].as_str(),
        Some("Run 'fspec validate broken' to see syntax errors")
    );

    // @step And spec/features/broken.feature is unchanged byte for byte
    assert_eq!(
        read_feature(tmp.path(), "spec/features/broken.feature"),
        BROKEN_FEATURE
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The added scenario uses two-space scenario and four-space step indentation
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_the_added_scenario_uses_two_space_scenario_and_four_space_step_indentation() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-scenario with feature='login' and scenario='New scenario'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "login", "scenario": "New scenario"}),
    ));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the line '  Scenario: New scenario' is followed by the lines '    Given [precondition]', '    When [action]' and '    Then [expected outcome]'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    let lines: Vec<&str> = after.lines().collect();
    let at = lines
        .iter()
        .position(|l| *l == "  Scenario: New scenario")
        .expect("scenario line with two-space indentation");
    assert_eq!(
        lines.get(at + 1..at + 4),
        Some(
            &[
                "    Given [precondition]",
                "    When [action]",
                "    Then [expected outcome]",
            ][..]
        ),
        "{after}"
    );
}
