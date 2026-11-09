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
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

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
    let feat = lines
        .iter()
        .position(|l| *l == "Feature: Login")
        .expect("Feature line");
    let bg = lines
        .iter()
        .position(|l| *l == "  Background: User Story")
        .expect("Background line");
    let scen = lines
        .iter()
        .position(|l| l.trim() == "Scenario: A")
        .expect("Scenario line");
    assert!(
        feat < bg && bg < scen,
        "expected Feature < Background < Scenario; got {feat}/{bg}/{scen}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Empty text is rejected and the file is left untouched
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_empty_text_is_rejected_and_file_untouched() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );
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
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

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
    assert!(
        msg.contains("Added background to dashboard"),
        "unexpected message: {msg}"
    );

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
    let count = after
        .lines()
        .filter(|l| l.trim() == "Background: User Story")
        .count();
    assert_eq!(
        count, 1,
        "expected exactly one Background line; got {count}:\n{after}"
    );
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
    let last_fence = lines
        .iter()
        .rposition(|l| l.trim() == "\"\"\"")
        .expect("closing fence");
    let bg = lines
        .iter()
        .position(|l| l.trim() == "Background: User Story")
        .expect("Background line");
    let scen = lines
        .iter()
        .position(|l| l.trim() == "Scenario: A")
        .expect("Scenario line");
    assert!(
        last_fence < bg && bg < scen,
        "expected fence < Background < Scenario; got {last_fence}/{bg}/{scen}"
    );

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
    write_feature(
        tmp.path(),
        "spec/features/bad.feature",
        "# just a comment\n",
    );

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

// ─────────────────────────────────────────────────────────────────────────
// Scenarios merged from add-background (COV-071): the rest of the file is kept
// ─────────────────────────────────────────────────────────────────────────

/// The original file with a Background block (blank line, keyword, story
/// lines, blank line) inserted right after `feature_line`.
fn with_background_after_feature(original: &str, feature_line: &str, story: &[&str]) -> String {
    let mut block = format!("{feature_line}\n\n  Background: User Story\n");
    for line in story {
        block.push_str(&format!("    {line}\n"));
    }
    block.push('\n');
    original.replacen(&format!("{feature_line}\n"), &block, 1)
}

fn dispatch_text(
    project_root: &Path,
    feature: &str,
    text: &str,
) -> codelet_fspec_core::DispatchResult {
    dispatch_command(req(project_root, json!({"feature": feature, "text": text})))
}

const PAYMENT_FEATURE: &str = "Feature: Payment\n\n  Scenario: Pay by card\n    Given a cart\n    When I pay by card\n    Then the order is placed\n\n  Scenario: Pay by invoice\n    Given a cart\n    When I pay by invoice\n    Then an invoice is sent\n\n  Scenario: Refund\n    Given a paid order\n    When I ask for a refund\n    Then the money is returned\n";

#[test]
fn scenario_adding_a_background_keeps_every_scenario_and_its_steps() {
    // @step Given a project root tempdir with spec/features/payment.feature holding the scenarios 'Pay by card', 'Pay by invoice' and 'Refund' with their steps
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/payment.feature", PAYMENT_FEATURE);

    // @step When I dispatch add-background with feature='payment' and text='As a customer\nI want to pay securely\nSo that my data is protected'
    let result = dispatch_text(
        tmp.path(),
        "payment",
        "As a customer\nI want to pay securely\nSo that my data is protected",
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk equals the original file with only the Background block added after the Feature line
    assert_eq!(
        read_feature(tmp.path(), "spec/features/payment.feature"),
        with_background_after_feature(
            PAYMENT_FEATURE,
            "Feature: Payment",
            &[
                "As a customer",
                "I want to pay securely",
                "So that my data is protected"
            ]
        )
    );
}

const AUTH_FEATURE: &str =
    "@security\n@critical\nFeature: Auth\n\n  Scenario: Sign in\n    Given an account\n    Then I am signed in\n";

#[test]
fn scenario_adding_a_background_keeps_the_feature_level_tags() {
    // @step Given a project root tempdir with spec/features/auth.feature tagged @security and @critical
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/auth.feature", AUTH_FEATURE);

    // @step When I dispatch add-background with feature='auth' and text='As a user\nI want authentication\nSo that my data is secure'
    let result = dispatch_text(
        tmp.path(),
        "auth",
        "As a user\nI want authentication\nSo that my data is secure",
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk equals the original file with only the Background block added after the Feature line
    assert_eq!(
        read_feature(tmp.path(), "spec/features/auth.feature"),
        with_background_after_feature(
            AUTH_FEATURE,
            "Feature: Auth",
            &[
                "As a user",
                "I want authentication",
                "So that my data is secure"
            ]
        )
    );
}

const ORDERS_FEATURE: &str = "Feature: Orders\n\n  @smoke\n  @regression\n  Scenario: List orders\n    Given two orders\n    Then both are listed\n";

#[test]
fn scenario_adding_a_background_keeps_scenario_level_tags() {
    // @step Given a project root tempdir with spec/features/orders.feature whose scenario is tagged @smoke and @regression
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/orders.feature", ORDERS_FEATURE);

    // @step When I dispatch add-background with feature='orders' and text='As a customer\nI want to manage orders\nSo that I track purchases'
    let result = dispatch_text(
        tmp.path(),
        "orders",
        "As a customer\nI want to manage orders\nSo that I track purchases",
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk equals the original file with only the Background block added after the Feature line
    assert_eq!(
        read_feature(tmp.path(), "spec/features/orders.feature"),
        with_background_after_feature(
            ORDERS_FEATURE,
            "Feature: Orders",
            &[
                "As a customer",
                "I want to manage orders",
                "So that I track purchases"
            ]
        )
    );
}

#[test]
fn scenario_the_background_holds_the_user_story_lines_in_order_indented_four_spaces() {
    // @step Given a project root tempdir with spec/features/search.feature containing only 'Feature: Search\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/search.feature",
        "Feature: Search\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch add-background with feature='search' and text='As a customer\nI want to search products\nSo that I can find what I need'
    let result = dispatch_text(
        tmp.path(),
        "search",
        "As a customer\nI want to search products\nSo that I can find what I need",
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the line '  Background: User Story' is followed by the lines '    As a customer', '    I want to search products' and '    So that I can find what I need'
    let after = read_feature(tmp.path(), "spec/features/search.feature");
    let lines: Vec<&str> = after.lines().collect();
    let at = lines
        .iter()
        .position(|l| *l == "  Background: User Story")
        .expect("Background line with two-space indentation");
    assert_eq!(
        lines.get(at + 1..at + 4),
        Some(
            &[
                "    As a customer",
                "    I want to search products",
                "    So that I can find what I need",
            ][..]
        ),
        "{after}"
    );
}
