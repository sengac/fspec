#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-architecture-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-architecture` (RPC-167).
// Each scenario maps to one #[test] fn with @step comments mirroring the
// Gherkin steps verbatim. At PHASE B (red) the command is still a stub, so the
// dispatcher returns the NotYetPorted envelope; the green-phase assertions are
// written as the real contract and pass once the impl lands in PHASE C.

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
        command: "add-architecture".to_string(),
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
// Scenario: Inserts a doc string after the Feature line when none exists
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_inserts_doc_string_after_feature_line() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-architecture with feature='spec/features/login.feature' and text='Uses bcrypt for password hashing'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "feature": "spec/features/login.feature",
            "text": "Uses bcrypt for password hashing"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher message contains 'Added architecture documentation to spec/features/login.feature'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("Added architecture documentation to spec/features/login.feature"),
        "unexpected message: {msg}"
    );

    // @step And the file on disk contains the line '  Uses bcrypt for password hashing'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    let lines: Vec<&str> = after.lines().collect();
    assert!(
        lines.contains(&"  Uses bcrypt for password hashing"),
        "missing indented doc-string body; got:\n{after}"
    );

    // @step And the doc-string fences appear immediately after the 'Feature: Login' line
    let feat = lines
        .iter()
        .position(|l| *l == "Feature: Login")
        .expect("Feature line");
    assert_eq!(
        lines.get(feat + 1).map(|l| l.trim()),
        Some("\"\"\""),
        "opening fence must immediately follow Feature line; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Inserts a multi-line doc string preserving each line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_inserts_multiline_doc_string() {
    // @step Given a project root tempdir with spec/features/login.feature containing only 'Feature: Login\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-architecture with feature='spec/features/login.feature' and text='Uses bcrypt\nSessions in Redis'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "feature": "spec/features/login.feature",
            "text": "Uses bcrypt\nSessions in Redis"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk contains the line '  Uses bcrypt'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        after.lines().any(|l| l == "  Uses bcrypt"),
        "missing line 1; got:\n{after}"
    );

    // @step And the file on disk contains the line '  Sessions in Redis'
    assert!(
        after.lines().any(|l| l == "  Sessions in Redis"),
        "missing line 2; got:\n{after}"
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

    // @step When I dispatch add-architecture with feature='spec/features/login.feature' and text=''
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "text": ""}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message equals 'Architecture text cannot be empty'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Architecture text cannot be empty"),
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

    // @step When I dispatch add-architecture with feature='spec/features/login.feature' and text='   '
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "text": "   "}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message equals 'Architecture text cannot be empty'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Architecture text cannot be empty"),
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

    // @step When I dispatch add-architecture with feature='spec/features/missing.feature' and text='Uses bcrypt'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/missing.feature", "text": "Uses bcrypt"}),
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

    // @step When I dispatch add-architecture with feature='dashboard' and text='Uses a worker pool'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "dashboard", "text": "Uses a worker pool"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher message contains 'Added architecture documentation to dashboard'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("Added architecture documentation to dashboard"),
        "unexpected message: {msg}"
    );

    // @step And the file spec/features/dashboard.feature on disk contains the line '  Uses a worker pool'
    let after = read_feature(tmp.path(), "spec/features/dashboard.feature");
    assert!(
        after.lines().any(|l| l == "  Uses a worker pool"),
        "missing doc-string body; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Replaces an existing Feature-line doc string in place
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_replaces_existing_doc_string_in_place() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  """\n  Old architecture\n  """\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        "Feature: Login\n  \"\"\"\n  Old architecture\n  \"\"\"\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch add-architecture with feature='spec/features/login.feature' and text='New architecture'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "text": "New architecture"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk contains the line '  New architecture'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        after.lines().any(|l| l == "  New architecture"),
        "missing new body; got:\n{after}"
    );

    // @step And the file on disk does NOT contain the line '  Old architecture'
    assert!(
        !after.lines().any(|l| l == "  Old architecture"),
        "old body must be removed; got:\n{after}"
    );

    // @step And the file on disk contains exactly two doc-string fence lines
    let fences = after.lines().filter(|l| l.trim() == "\"\"\"").count();
    assert_eq!(
        fences, 2,
        "expected exactly two fence lines; got {fences}:\n{after}"
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

    // @step When I dispatch add-architecture with feature='spec/features/bad.feature' and text='Uses bcrypt'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/bad.feature", "text": "Uses bcrypt"}),
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
// Scenarios merged from add-architecture (COV-071): the rest of the file is kept
// ─────────────────────────────────────────────────────────────────────────

/// The original file with a one-line architecture doc string inserted right
/// after `feature_line`, fences and content indented two spaces.
fn with_doc_after_feature(original: &str, feature_line: &str, text: &str) -> String {
    original.replacen(
        &format!("{feature_line}\n"),
        &format!("{feature_line}\n  \"\"\"\n  {text}\n  \"\"\"\n"),
        1,
    )
}

fn dispatch_text(
    project_root: &Path,
    feature: &str,
    text: &str,
) -> codelet_fspec_core::DispatchResult {
    dispatch_command(req(project_root, json!({"feature": feature, "text": text})))
}

const CHECKOUT_FEATURE: &str = "Feature: Checkout\n\n  Scenario: Pay by card\n    Given a cart\n    When I pay by card\n    Then the order is placed\n\n  Scenario: Pay by invoice\n    Given a cart\n    When I pay by invoice\n    Then an invoice is sent\n\n  Scenario: Cancel order\n    Given an order\n    When I cancel it\n    Then it is cancelled\n";

#[test]
fn scenario_adding_notes_keeps_every_scenario_and_its_steps() {
    // @step Given a project root tempdir with spec/features/checkout.feature holding the scenarios 'Pay by card', 'Pay by invoice' and 'Cancel order' with their steps
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/checkout.feature",
        CHECKOUT_FEATURE,
    );

    // @step When I dispatch add-architecture with feature='checkout' and text='Payment processing architecture'
    let result = dispatch_text(tmp.path(), "checkout", "Payment processing architecture");

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk equals the original file with only the doc string added after the Feature line
    assert_eq!(
        read_feature(tmp.path(), "spec/features/checkout.feature"),
        with_doc_after_feature(
            CHECKOUT_FEATURE,
            "Feature: Checkout",
            "Payment processing architecture"
        )
    );
}

const AUTH_FEATURE: &str = "Feature: Auth\n\n  Background: User Story\n    As a user\n    I want to sign in\n    So that I can work\n\n  Scenario: Sign in\n    Given an account\n    Then I am signed in\n";

#[test]
fn scenario_adding_notes_keeps_the_background_after_the_new_doc_string() {
    // @step Given a project root tempdir with spec/features/auth.feature that has a Background section
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/auth.feature", AUTH_FEATURE);

    // @step When I dispatch add-architecture with feature='auth' and text='OAuth 2.0 implementation'
    let result = dispatch_text(tmp.path(), "auth", "OAuth 2.0 implementation");

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk equals the original file with only the doc string added after the Feature line
    let after = read_feature(tmp.path(), "spec/features/auth.feature");
    assert_eq!(
        after,
        with_doc_after_feature(AUTH_FEATURE, "Feature: Auth", "OAuth 2.0 implementation")
    );

    // @step And the closing doc-string fence comes before the 'Background:' line
    let lines: Vec<&str> = after.lines().collect();
    let last_fence = lines
        .iter()
        .rposition(|l| l.trim() == "\"\"\"")
        .expect("closing fence");
    let background = lines
        .iter()
        .position(|l| l.trim_start().starts_with("Background:"))
        .expect("Background line");
    assert!(last_fence < background, "{after}");
}

const SEARCH_FEATURE: &str =
    "@api\n@critical\nFeature: Search\n\n  Scenario: Find a page\n    Given an index\n    Then a page is found\n";

#[test]
fn scenario_adding_notes_keeps_the_feature_level_tags_above_the_feature_line() {
    // @step Given a project root tempdir with spec/features/search.feature tagged @api and @critical
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/search.feature", SEARCH_FEATURE);

    // @step When I dispatch add-architecture with feature='search' and text='ElasticSearch integration'
    let result = dispatch_text(tmp.path(), "search", "ElasticSearch integration");

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk equals the original file with only the doc string added after the Feature line
    assert_eq!(
        read_feature(tmp.path(), "spec/features/search.feature"),
        with_doc_after_feature(
            SEARCH_FEATURE,
            "Feature: Search",
            "ElasticSearch integration"
        )
    );
}

const NOTIFICATIONS_FEATURE: &str = "Feature: Notifications\n\n  @email\n  @sms\n  Scenario: Send a notice\n    Given a subscriber\n    Then a notice is sent\n";

#[test]
fn scenario_adding_notes_keeps_scenario_level_tags() {
    // @step Given a project root tempdir with spec/features/notifications.feature whose scenario is tagged @email and @sms
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/notifications.feature",
        NOTIFICATIONS_FEATURE,
    );

    // @step When I dispatch add-architecture with feature='notifications' and text='Notification service architecture'
    let result = dispatch_text(
        tmp.path(),
        "notifications",
        "Notification service architecture",
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk equals the original file with only the doc string added after the Feature line
    assert_eq!(
        read_feature(tmp.path(), "spec/features/notifications.feature"),
        with_doc_after_feature(
            NOTIFICATIONS_FEATURE,
            "Feature: Notifications",
            "Notification service architecture"
        )
    );
}

#[test]
fn scenario_doc_string_fences_and_content_lines_are_indented_two_spaces() {
    // @step Given a project root tempdir with spec/features/reporting.feature containing only 'Feature: Reporting\n  Scenario: A\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/reporting.feature",
        "Feature: Reporting\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch add-architecture with feature='reporting' and text='Line 1\nLine 2\nLine 3'
    let result = dispatch_text(tmp.path(), "reporting", "Line 1\nLine 2\nLine 3");

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the lines after 'Feature: Reporting' are '  """', '  Line 1', '  Line 2', '  Line 3' and '  """'
    let after = read_feature(tmp.path(), "spec/features/reporting.feature");
    let lines: Vec<&str> = after.lines().collect();
    assert_eq!(lines.first(), Some(&"Feature: Reporting"), "{after}");
    assert_eq!(
        lines.get(1..6),
        Some(&["  \"\"\"", "  Line 1", "  Line 2", "  Line 3", "  \"\"\""][..]),
        "{after}"
    );
}
